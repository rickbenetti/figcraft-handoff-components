#!/usr/bin/env python3
"""The Settings panel, driven the way the host drives it (headless, with the built dist/).

The host hands its events to the plugin in the order of the panel's controls and, in one frame, a field that
loses focus and the button that took it both fire. With the buttons AFTER the fields the field's event comes
first, so the button already sees the new value. This script plays that order and checks that the first Save
writes what was typed:

  1. Version = "0.3", then Save   -> the component's sheet holds version 0.3 after that one Save.
  2. Line colour = "#112233", then Save -> the page's settings hold line #112233 after that one Save.
  3. No button comes before any field, in the real panel, for every kind of selection.
  4. Selection changed, then Save -> refused: one refusal note only, nothing written.
  5. The messages: no "all saved" next to "Settings saved"; the short Reload note unless something was unsaved.
  6. Nothing loaded: the component section says so and the "selection changed" warning is not shown.

`--states` prints the ordered controls (type and label) of each state of the panel, in pt-BR, as Markdown.
Everything is written to a temporary folder (never over a fixture). Needs the plugin built (scripts/build.sh)
and figcraft-cli on the PATH (or FIGCRAFT_CLI=/path/to/figcraft-cli)."""
import json, os, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIST = os.path.join(ROOT, "dist")
FIX = os.path.join(ROOT, "tests/fixtures")
DEMO = os.path.join(ROOT, "demo/ds-base-demo.figcraft")
PID = "me.benetti.handoff-components"
CLI = os.environ.get("FIGCRAFT_CLI", "figcraft-cli")
FIELDS = {"text", "number", "boolean", "select", "color", "list", "slider"}


def run(src, steps, tmp, n=[0]):
    n[0] += 1
    script = os.path.join(tmp, f"s{n[0]}.json")
    out = os.path.join(tmp, f"s{n[0]}.figcraft")
    json.dump([{"command": "plugin.import", "params": {"path": DIST}}] + steps, open(script, "w"))
    p = subprocess.run([CLI, "run", "--in", src, "--out", out, script], capture_output=True, text=True, timeout=300)
    lines = [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]
    bad = [l for l in lines if not l.get("ok")]
    if bad:
        sys.exit(f"step failed: {bad[0]}")
    return lines[1:]  # without plugin.import


def settings(**params):
    return {"command": "plugin.run", "params": {"id": PID, "command": "settings", "params": params}}


def open_panel():
    return {"command": "plugin.run", "params": {"id": PID, "command": "settings"}}


def event(control, value=None):
    p = {"id": PID, "control": control}
    if value is not None:
        p["value"] = value
    return {"command": "plugin.event", "params": p}


def select(*ids):
    return {"command": "select.set", "params": {"ids": list(ids)}}


def controls(line):
    return [(c["type"], c.get("label", "")) for c in line["result"]["panel"]["controls"]]


def check_order(ctl, where):
    kinds = [t for t, _ in ctl]
    first_button = kinds.index("button")
    last_field = max((i for i, k in enumerate(kinds) if k in FIELDS), default=-1)
    assert last_field < first_button, f"{where}: a button comes before a field: {kinds}"
    assert ctl[first_button][1] in ("Save changes", "Salvar alterações"), f"{where}: the first button is not Save"


def main():
    states = "--states" in sys.argv
    if not os.path.isdir(DIST):
        sys.exit(f"build the plugin first: {DIST} is missing")
    failures = []
    with tempfile.TemporaryDirectory(prefix="handoff-panel-") as tmp:
        copia = f"{FIX}/fixture-copia-2.figcraft"

        # 1. Version, then Save: the first Save writes it.
        r = run(copia, [select(32), open_panel(), event("version", "0.3"), event("save"), event("reload")], tmp)
        opened, typed, saved, reloaded = r[1], r[2], r[3], r[4]
        ok = saved["result"]["output"]["extra"]["saved"] and saved["result"]["output"]["extra"]["wrote_component"]
        shown = reloaded["result"]["panel"]["values"].get("version")
        if not (ok and shown == "0.3"):
            failures.append(f"version then save: saved={ok}, reloaded shows {shown!r}")
        print(f"[{'ok' if ok and shown == '0.3' else 'FAIL'}] Version=0.3, then Save: written on the first Save, and a Reload shows {shown!r}")

        # 2. The line colour typed in the hex field, then Save.
        r = run(copia, [select(32), open_panel(), event("line", "#112233"), event("save"), event("reload")], tmp)
        saved, reloaded = r[3], r[4]
        ok = saved["result"]["output"]["extra"]["saved"] and saved["result"]["output"]["extra"]["wrote_page"]
        shown = reloaded["result"]["panel"]["values"].get("line")
        if not (ok and shown == "#112233"):
            failures.append(f"line then save: saved={ok}, reloaded shows {shown!r}")
        print(f"[{'ok' if ok and shown == '#112233' else 'FAIL'}] Line colour=#112233, then Save: written on the first Save, and a Reload shows {shown!r}")

        # 3. No button before any field, whatever is selected.
        cases = [("no selection", copia, []), ("component", copia, [32]), ("variant", copia, [3]), ("set", DEMO, None)]
        for name, src, ids in cases:
            if ids is None:
                found = find_set(src, "Button", tmp)
                ids = [found]
            r = run(src, [select(*ids), open_panel()], tmp)
            ctl = controls(r[1])
            try:
                check_order(ctl, name)
                print(f"[ok] order, {name}: {len(ctl)} controls, buttons after every field")
            except AssertionError as e:
                failures.append(str(e))
                print(f"[FAIL] {e}")

        # 4. A save refused because the selection is another one: a single refusal note.
        r = run(copia, [select(32), open_panel(), select(3), event("version", "9"), event("save")], tmp)
        out = r[4]["result"]["output"]
        labels = [l for _, l in controls(r[4])]
        refusals = [l for l in labels if l.startswith("✖")]
        ok = (not out["extra"]["saved"]) and len(refusals) == 1 and "Nothing was saved" in refusals[0]
        if not ok:
            failures.append(f"refusal: saved={out['extra']['saved']}, notes={refusals}")
        print(f"[{'ok' if ok else 'FAIL'}] Selection changed, then Save: refused, {len(refusals)} refusal note: {refusals}")

        # 5. Messages: no duplicate after a save; the short reload note unless there were unsaved changes.
        r = run(copia, [select(32), open_panel(), event("version", "0.4"), event("save"), event("reload"), event("version", "0.5"), event("reload")], tmp)
        after_save = [l for _, l in controls(r[3])]
        clean = [l for _, l in controls(r[4])]
        dirty = [l for _, l in controls(r[6])]
        ok = (not any("All changes are saved" in l for l in after_save) and any(l == "✔ Settings saved" for l in after_save)
              and "✔ Reloaded from what is saved." in clean and not any("discarded" in l for l in clean)
              and any("unsaved changes were discarded" in l for l in dirty))
        if not ok:
            failures.append(f"messages: after save={after_save[-8:]}, clean reload={clean[-8:]}, dirty reload={dirty[-8:]}")
        print(f"[{'ok' if ok else 'FAIL'}] Messages: no 'all saved' next to 'Settings saved'; short Reload note, long one only after unsaved changes")

        # 6. Nothing loaded: the component section says so, and the selection warning is not shown.
        r = run(copia, [select(), open_panel(), select(32), event("line", "#112233")], tmp)
        for where, line in (("opened", r[1]), ("after a selection change", r[3])):
            labels = [l for _, l in controls(line)]
            ok = "COMPONENT" in labels and any(l.startswith("No component loaded.") for l in labels) and not any("selection changed" in l for l in labels)
            if not ok:
                failures.append(f"no component, {where}: {labels}")
            print(f"[{'ok' if ok else 'FAIL'}] No component loaded, {where}: section note present, no selection warning")

        if states:
            print_states(tmp, copia)
    if failures:
        print("\nFAILURES:")
        for f in failures:
            print(" -", f)
        return 1
    print("\nall panel sequences hold")
    return 0


def find_set(src, name, tmp):
    r = run(src, [{"command": "node.find", "params": {"name": name, "type": "component_set", "limit": 10}}], tmp)
    ids = r[0]["result"]["ids"]
    for i in ids:
        g = run(src, [{"command": "node.get", "params": {"id": i, "fields": ["name"]}}], tmp)
        if g[0]["result"]["name"] == name:
            return i
    sys.exit(f"set {name!r} not found")


def md(title, ctl):
    print(f"\n#### {title} ({len(ctl)} controles)\n")
    for i, (t, l) in enumerate(ctl, 1):
        print(f"{i}. `{t}` {l!r}" if l else f"{i}. `{t}` (espaçador vazio)")


def print_states(tmp, copia):
    pt = settings(lang="pt-BR")
    button = find_set(DEMO, "Button", tmp)
    r = run(copia, [pt, select(), open_panel()], tmp)
    md("Sem seleção", controls(r[2]))
    r = run(copia, [pt, select(32), open_panel(), event("version", "0.3"), event("save"), event("reload")], tmp)
    md("Componente (recém-aberto)", controls(r[2]))
    md("Componente, com alterações não salvas (após digitar a Versão)", controls(r[3]))
    md("Componente, logo depois de salvar", controls(r[4]))
    md("Componente, logo depois de recarregar", controls(r[5]))
    r = run(DEMO, [pt, select(button), open_panel()], tmp)
    md("Conjunto com variantes (Button do DS Base Demo)", controls(r[2]))
    r = run(copia, [pt, select(3), open_panel()], tmp)
    md("Variante de um conjunto (copia-2, id 3)", controls(r[2]))
    r = run(copia, [pt, select(32), open_panel(), select(3), event("version", "9"), event("save")], tmp)
    md("Recusa por seleção diferente (salvar com outra seleção)", controls(r[5]))
    r = run(copia, [pt, select(32), open_panel(), select(3), event("version", "9")], tmp)
    md("Seleção mudou, sem salvar (aviso de seleção)", controls(r[4]))


if __name__ == "__main__":
    sys.exit(main())
