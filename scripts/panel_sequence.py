#!/usr/bin/env python3
"""The "Generate documentation…" panel, driven the way the host drives it (headless, with the built dist/).

The host hands its events to the plugin in the order of the panel's controls; with the buttons AFTER the fields
a field's event comes first, so the button already sees the new value. This script plays that order:

  1. The menu (generate, no parameters) opens the panel and makes nothing; `direct: true` makes the document with no panel.
     The panel has 11 controls (at most 12), the first button is Generate and every field comes before the buttons.
  2. Language typed, then "Generate": the Section is made, the panel says ✔ in Portuguese, the screen is at the Section and the
     selection is the component set / component (it lives in the Component Master now, not the frame).
  3. Nothing selected: refused with a note, nothing in the document. A selection that is not a component: refused.
  4. The stage typed, then the button: saved on the page on the first click; Restore defaults puts language and stage back.
  5. "Update": a rerun replaces only the frames it names (Tokens only: one frame) and the Component Master, which is remade
     on every run; the Cover, the Changelog and the support components keep their ids; "New Section" makes a second Section
     without a Master and leaves the first one alone.
  6. A loose 0.10.0 documentation (a frame with the old mark and no Section) is left alone and the panel says so.
  7. The panel has the same controls and labels before and after a document and a refusal.
  8. A forced error (an instruction budget too small for the callback): everything is rolled back.

(The checks of the 0.10.0 drawing, measure rectangles, repeated property names and swap chips, came back with the
Properties and Anatomy frames in the next steps of 0.11: they are not in this version of the script.)

`--states` prints the ordered controls (type and label) of the panel in each state, in pt-BR, as Markdown.
Everything is written to a temporary folder (never over a fixture). Needs the plugin built (scripts/build.sh)
and figcraft-cli on the PATH (or FIGCRAFT_CLI=/path/to/figcraft-cli)."""
import json, os, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIST = os.path.join(ROOT, "dist")
FIX = os.path.join(ROOT, "tests/fixtures")
PID = "me.benetti.handoff-components"
CLI = os.environ.get("FIGCRAFT_CLI", "figcraft-cli")
FIELDS = {"text", "number", "boolean", "select", "color", "list", "slider"}


def run(src, steps, tmp, n=[0], expect_error=False):
    n[0] += 1
    script = os.path.join(tmp, f"s{n[0]}.json")
    out = os.path.join(tmp, f"s{n[0]}.figcraft")
    json.dump([{"command": "plugin.import", "params": {"path": DIST}}] + steps, open(script, "w"))
    p = subprocess.run([CLI, "run", "--in", src, "--out", out, script], capture_output=True, text=True, timeout=300)
    lines = [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]
    bad = [l for l in lines if not l.get("ok")]
    if expect_error:
        return bad[0] if bad else None
    if bad:
        sys.exit(f"step failed: {bad[0]}")
    return lines[1:]  # without plugin.import


def generate(**params):
    return {"command": "plugin.run", "params": {"id": PID, "command": "generate", "params": params}}


def event(control, value=None):
    p = {"id": PID, "control": control}
    if value is not None:
        p["value"] = value
    return {"command": "plugin.event", "params": p}


def ui():
    return {"command": "plugin.ui", "params": {"id": PID}}


def select(*ids):
    return {"command": "select.set", "params": {"ids": list(ids)}}


def find(name):
    return {"command": "node.find", "params": {"type": "section", "name": name, "limit": 50}}


def frames_of(section_id):
    return {"command": "node.get", "params": {"id": section_id, "fields": ["children", "pluginData"]}}


def output(line):
    return line["result"]["output"]


def controls(line):
    panel = line["result"].get("panel") or line["result"]
    return [(c["type"], c.get("label", "")) for c in panel["controls"]]


def check_order(ctl, where):
    kinds = [t for t, _ in ctl]
    first_button = kinds.index("button")
    last_field = max((i for i, k in enumerate(kinds) if k in FIELDS), default=-1)
    assert last_field < first_button, f"{where}: a button comes before a field: {kinds}"
    assert ctl[first_button][1] in ("Generate", "Gerar"), f"{where}: the first button is not Generate"


def main():
    states = "--states" in sys.argv
    if not os.path.isdir(DIST):
        sys.exit(f"build the plugin first: {DIST} is missing")
    failures = []

    def check(ok, text, detail=""):
        print(f"[{'ok' if ok else 'FAIL'}] {text}{(' — ' + detail) if detail else ''}")
        if not ok:
            failures.append(text + (f": {detail}" if detail else ""))

    def values(line):
        return line["result"]["values"]

    with tempfile.TemporaryDirectory(prefix="handoff-panel-") as tmp:
        copia = f"{FIX}/fixture-copia-2.figcraft"
        EN = "Handoff"

        # 1. The menu opens the panel; `direct` makes the document with no panel.
        r = run(copia, [select(32), generate(), ui(), find(EN)], tmp)
        opened, panel, sections = r[1], r[2], r[3]
        ok = output(opened)["phase"] == "open" and bool(panel["result"]) and sections["result"]["total"] == 0
        n = len(controls(panel))
        check(ok, "generate with no parameters (the menu) opens the panel and makes nothing", f"{n} controls, {sections['result']['total']} Sections")
        check(n == 11 and n <= 12, "the panel has 11 controls (at most 12)", str(n))
        check_order(controls(panel), "menu")
        names = [c.get("name") for c in panel["result"]["controls"] if c["type"] != "note"]
        check(names == ["lang", "stage", "update", "generate", "new_section", "defaults", "close"], "fields: lang, stage, update; buttons: generate, new_section, defaults, close", str(names))
        r = run(copia, [select(32), generate(direct=True), ui(), find(EN)], tmp)
        check(bool(output(r[1]).get("frame")) and not r[2]["result"] and r[3]["result"]["total"] == 1, "generate with direct: true makes the Section and opens no panel")

        # 2. Language typed, then the button; in Portuguese.
        r = run(copia, [select(32), generate(), event("lang", "Português (Brasil)"), event("generate"), ui(), find(EN)], tmp)
        click, after, sections = r[3], r[4], r[5]
        out = output(click)
        check(not out["refused"] and len(out["frames"]) == 8, "Português typed, then one click: made, with 8 frames", str(len(out.get("frames", []))))
        check(out["selected"] is not None and out["selected"] == out["selected"], "the selection is the component that lives in the Component Master", str(out["selected"]))
        v = out["view"]
        check("error" not in v and v["zoom"] is not None and 0 < v["zoom"] <= 1.0, "the screen went to the Section (zoom at most 100%)", json.dumps({k: v.get(k) for k in ("zoom", "cx", "cy", "vw", "vh")}))
        check(sections["result"]["total"] == 1, "one Section named Handoff · …")
        done = [l for _, l in controls(after) if l.startswith("✔ Documentação gerada: ")]
        check(bool(done), "the panel says ✔ Documentação gerada", done[0] if done else "")

        # 3. Nothing selected, and a selection that is not a component.
        r = run(copia, [select(), generate(), event("generate"), find(EN)], tmp)
        labels = [l for _, l in controls(r[2])]
        check(output(r[2])["refused"] and r[3]["result"]["total"] == 0 and any(l.startswith("✖ Nothing is selected") for l in labels), "nothing selected: refused with a ✖ note, nothing made")
        r = run(copia, [select(32), generate(direct=True), {"command": "node.find", "params": {"type": "section", "limit": 5}}], tmp)
        sec = r[2]["result"]["ids"][0]
        r = run(copia, [select(32), generate(direct=True), select(sec), generate(), event("generate"), find(EN)], tmp)
        labels = [l for _, l in controls(r[4])]
        check(output(r[4])["refused"] and r[5]["result"]["total"] == 1 and any(l.startswith("✖ The selection is a Section") for l in labels), "a Section selected: refused with a ✖ note, nothing new made")

        # 4. A field, then the button: written on the first click. Restore defaults.
        r = run(copia, [select(32), generate(), event("stage", "Dark"), event("generate"), generate(), ui()], tmp)
        check(values(r[5]).get("stage") == "Dark", "stage typed, then one click: saved on the page (a reopened panel shows it)", str(values(r[5]).get("stage")))
        r = run(copia, [select(32), generate(), event("lang", "Português (Brasil)"), event("stage", "Escuro"), event("generate"), select(32), generate(), event("defaults"), event("generate"), generate(), ui()], tmp)
        last = values(r[-1])
        check(last.get("stage") == "Auto" and last.get("lang") == "English", "Restore defaults, then the button: language and stage are cleared on the page", str((last.get("lang"), last.get("stage"))))

        # 5. Update, and New Section.
        r = run(copia, [select(32), generate(), event("generate"), select(32), generate(), event("update", "Tokens only"), event("generate")], tmp)
        first, second = {f["key"]: f for f in output(r[2])["frames"]}, {f["key"]: f for f in output(r[6])["frames"]}
        acts = {k: v["action"] for k, v in second.items()}
        check([k for k, a in acts.items() if a == "replaced"] == ["master", "tokens"] and all(first[k]["id"] == second[k]["id"] for k in ("cover", "changelog")) and first["master"]["id"] != second["master"]["id"], "Tokens only: Tokens and the Master (remade on every run) are replaced; Cover and Changelog keep their ids", str(acts))
        r = run(copia, [select(32), generate(), event("generate"), select(32), generate(), event("new_section"), find(EN), {"command": "node.find", "params": {"type": "section", "limit": 5}}], tmp)
        out = output(r[5])
        keys = [f["key"] for f in out["frames"]]
        check(out["new_section"] and "master" not in keys and "kit" not in keys and r[7]["result"]["total"] == 2, "New Section: a second Section with eight frames (the Support frame is shared, not made again) and no Master", str(keys))

        # 6. A loose 0.10.0 documentation: a frame on the page with the old mark and no Section.
        legacy = {"command": "frame.create", "params": {"x": 5000, "y": 0, "width": 1280, "height": 400, "name": "Documentation — x", "select": False, "props": {"pluginData": {"handoff-report": "32"}}}}
        r = run(copia, [legacy, select(32), generate(direct=True)], tmp)
        check("legacy_doc" in output(r[2])["warnings"], "a loose 0.10.0 documentation is reported and left alone", str(output(r[2])["warnings"]))

        # 7. The panel has the same controls and labels before and after a document and a refusal.
        r = run(copia, [select(32), generate(), event("generate"), select(), event("generate")], tmp)
        a, b, c = controls(r[1]), controls(r[2]), controls(r[4])
        strip = lambda ctl: ctl[:-1]  # without the message
        check(len(a) == len(b) == len(c) == 11 and strip(a) == strip(b) == strip(c), "11 controls with the same labels when opened, after a document and after a refusal")
        for name, src, ids in [("no selection", copia, []), ("component", copia, [32]), ("variant", copia, [3])]:
            r = run(src, [select(*ids), generate()], tmp)
            try:
                check_order(controls(r[1]), name)
                print(f"[ok] order, {name}: {len(controls(r[1]))} controls, buttons after every field")
            except AssertionError as e:
                failures.append(str(e))
                print(f"[FAIL] {e}")

        # 8. A forced error: the instruction budget is too small for the callback, so the host rolls everything back.
        err = run(copia, [select(32), {"command": "plugin.run", "params": {"id": PID, "command": "generate", "fuel": 9000000}}, event("generate")], tmp, expect_error=True)
        check(err is not None and err.get("rolledBack") is True and "instruction budget" in err.get("error", ""), "a callback that fails is rolled back by the host (a budget too small is the forced error)", (err or {}).get("error", "no error")[:120])

        if states:
            print_states(tmp, copia)
    if failures:
        print("\nFAILURES:")
        for f in failures:
            print(" -", f)
        return 1
    print("\nall panel sequences hold")
    return 0


def md(title, ctl):
    print(f"\n#### {title} ({len(ctl)} controles)\n")
    for i, (t, l) in enumerate(ctl, 1):
        print(f"{i}. `{t}` {l!r}" if l else f"{i}. `{t}` (espaçador vazio)")


def print_states(tmp, copia):
    pt = generate(direct=True, lang="pt-BR")
    r = run(copia, [select(32), pt, select(32), generate()], tmp)
    md("Aberto (com ou sem seleção: a lista é a mesma)", controls(r[3]))
    r = run(copia, [select(32), pt, select(), generate(), event("generate")], tmp)
    md("Recusa: nada selecionado", controls(r[4]))
    r = run(copia, [select(32), pt, generate(), event("generate")], tmp)
    md("Recusa: seleção que não é componente", controls(r[3]))


if __name__ == "__main__":
    sys.exit(main())
