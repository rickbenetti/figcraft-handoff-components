#!/usr/bin/env python3
"""The "Generate documentation…" panel, driven the way the host drives it (headless, with the built dist/).

The host hands its events to the plugin in the order of the panel's controls; with the buttons AFTER the fields
a field's event comes first, so the button already sees the new value. This script plays that order:

  1. The menu (generate, no parameters) opens the panel and makes nothing; `direct: true` makes the document with no panel.
  2. Version="1.2" and Status="Aprovado" typed, then "Generate documentation": the Cover's chips say them, the first click
     wrote it, Version and Status are empty again, the selection is what it was, and the screen is at the Section.
  3. Nothing selected: refused with a note, nothing in the document.
  4. A selection that is not a component (the Section): refused with a note, nothing new.
  5. The stage typed, then the button: saved on the page on the first click; Restore defaults puts it back to automatic.
  6. The switches: all ticked for a component with no Section; a rerun has Cover, Component Master and Changelog unticked;
     a rerun replaces only the ticked sections (the Cover, the Master and the Changelog keep their ids); ticking the Cover
     replaces it; no section ticked is refused.
  7. "Go to Component Master": found and selected when the Section exists, a ✖ note when it doesn't.
  8. A loose 0.10.0 documentation (a frame with the old mark and no Section) is left alone and the panel says so.
  9. A forced error (an instruction budget too small for the callback): everything is rolled back.

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
    assert ctl[first_button][1] in ("Generate documentation", "Gerar documentação"), f"{where}: the first button is not Generate"


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
        EN, PT = "Handoff", "Handoff"

        # 1. The menu opens the panel; `direct` makes the document with no panel.
        r = run(copia, [select(32), generate(), ui(), find(EN)], tmp)
        opened, panel, sections = r[1], r[2], r[3]
        ok = output(opened)["phase"] == "open" and bool(panel["result"]) and sections["result"]["total"] == 0
        check(ok, "generate with no parameters (the menu) opens the panel and makes nothing", f"{len(controls(panel))} controls, {sections['result']['total']} Sections")
        check(len(controls(panel)) == 30, "the panel has 30 controls (the 17 of 0.10.0 and 13 new ones), at most 64", str(len(controls(panel))))
        check_order(controls(panel), "menu")
        r = run(copia, [select(32), generate(direct=True), ui(), find(EN)], tmp)
        check(bool(output(r[1]).get("frame")) and not r[2]["result"] and r[3]["result"]["total"] == 1, "generate with direct: true makes the Section and opens no panel")

        # 2. Version and Status typed, then the button; in Portuguese.
        r = run(
            copia,
            [select(32), generate(), event("lang", "Português (Brasil)"), event("version", "1.2"), event("status", "Aprovado"), event("generate"), ui(), event("stage", "Claro"), find(EN)],
            tmp,
        )
        click, after, seen, sections = r[5], r[6], r[7], r[8]
        out = output(click)
        check(not out["refused"] and out["meta"] == {"version": "1.2", "status": "Aprovado"}, "Versão=1.2 and Status=Aprovado typed, then one click: made, with both texts", json.dumps(out["meta"], ensure_ascii=False))
        check(values(after).get("version") == "" and values(after).get("status") == "", "Version and Status are empty again after the document")
        check(output(seen)["selection_now"] == [32], "the selection is back to what it was before the click", str(output(seen)["selection_now"]))
        v = out["view"]
        check("error" not in v and v["zoom"] is not None and 0 < v["zoom"] <= 1.0, "the screen went to the Section (zoom at most 100%)", json.dumps({k: v.get(k) for k in ("zoom", "cx", "cy", "vw", "vh")}))
        check(sections["result"]["total"] == 1, "one Section named Handoff · …")
        msg = [l for _, l in controls(after)]
        done = [l for l in msg if l.startswith("✔ Documentação gerada: ")]
        check(bool(done), "the panel says ✔ Documentação gerada", done[0] if done else "")
        # The Cover's chips say the Version and the Status (texts of the document).
        r = run(copia, [select(32), generate(direct=True, lang="pt-BR", version="1.2", status="Aprovado"), {"command": "node.find", "params": {"type": "text", "text": "Versão: 1.2", "limit": 5}}, {"command": "node.find", "params": {"type": "text", "text": "Status: Aprovado", "limit": 5}}], tmp)
        check(r[2]["result"]["total"] == 1 and r[3]["result"]["total"] == 1, "the Cover's chips say «Versão: 1.2» and «Status: Aprovado»")
        r = run(copia, [select(32), generate(direct=True), {"command": "node.find", "params": {"type": "text", "text": "Version: to be defined", "limit": 5}}, {"command": "node.find", "params": {"type": "text", "text": "Status: to be defined", "limit": 5}}], tmp)
        check(r[2]["result"]["total"] == 1 and r[3]["result"]["total"] == 1, "with nothing typed the chips say «to be defined» twice")

        # 3. Nothing selected.
        r = run(copia, [select(), generate(), event("generate"), find(EN)], tmp)
        out, sections = output(r[2]), r[3]
        labels = [l for _, l in controls(r[2])]
        check(out["refused"] and sections["result"]["total"] == 0 and any(l.startswith("✖ Nothing is selected") for l in labels), "nothing selected: refused with a ✖ note, nothing made")

        # 4. The selection is not a component: the Section just made.
        r = run(copia, [select(32), generate(direct=True), {"command": "node.find", "params": {"type": "section", "limit": 5}}], tmp)
        sec = r[2]["result"]["ids"][0]
        r = run(copia, [select(32), generate(direct=True), select(sec), generate(), event("generate"), find(EN)], tmp)
        out, sections = output(r[4]), r[5]
        labels = [l for _, l in controls(r[4])]
        refusal = [l for l in labels if l.startswith("✖")]
        check(out["refused"] and sections["result"]["total"] == 1 and any(l.startswith("✖ The selection is of type Section") for l in labels), "a Section selected: refused with a ✖ note, nothing new made", refusal[0] if refusal else "")

        # 5. A field, then the button: written on the first click.
        r = run(copia, [select(32), generate(), event("stage", "Dark"), event("generate"), generate(), ui()], tmp)
        reopened = values(r[5])
        check(reopened.get("stage") == "Dark", "stage typed, then one click: saved on the page (a reopened panel shows it)", str(reopened.get("stage")))
        check("line" not in reopened and "fill_opacity" not in reopened, "the panel has no line colour and no fill opacity control", str(sorted(reopened)))
        r = run(copia, [select(32), generate(), event("stage", "Dark"), event("generate"), select(32), generate(), event("defaults"), event("generate"), generate(), ui()], tmp)
        check(values(r[9]).get("stage") == "Auto", "Restore defaults, then the button: the page's settings are cleared (the stage is automatic again)", str(values(r[9]).get("stage")))

        # 6. The switches.
        SECS = ["sec_cover", "sec_master", "sec_properties", "sec_anatomy", "sec_variants", "sec_layouts", "sec_tokens", "sec_changelog"]
        r = run(copia, [select(32), generate(), ui(), event("generate"), select(32), generate(), ui()], tmp)
        first, again = values(r[2]), values(r[6])
        check(all(first[k] is True for k in SECS), "a component with no Section: all eight switches ticked", str({k: first[k] for k in SECS}))
        want = {"sec_cover": False, "sec_master": False, "sec_properties": True, "sec_anatomy": True, "sec_variants": True, "sec_layouts": True, "sec_tokens": True, "sec_changelog": False}
        check({k: again[k] for k in SECS} == want, "a rerun: Cover, Component Master and Changelog unticked, the rest ticked", str({k: again[k] for k in SECS}))
        notes = [l for _, l in controls(r[6])]
        check(any("ticking them replaces it" in l for l in notes) and any("its switch is ignored" in l for l in notes), "the panel warns that ticking Cover or Changelog replaces the manual text and that the Master switch is ignored once it exists")
        r = run(copia, [select(32), generate(), event("generate"), select(32), generate(), event("generate")], tmp)
        first, second = {f["key"]: f for f in output(r[2])["frames"]}, {f["key"]: f for f in output(r[5])["frames"]}
        same = [k for k in ("cover", "master", "changelog") if first[k]["id"] == second[k]["id"]]
        newer = [k for k in ("properties", "anatomy", "variants", "layouts", "tokens") if first[k]["id"] != second[k]["id"]]
        check(same == ["cover", "master", "changelog"] and len(newer) == 5 and output(r[5])["frames"] and output(r[2])["frames"], "rerun: Cover, Master and Changelog keep their ids; the other five frames are new", f"kept {same}, replaced {newer}")
        r = run(copia, [select(32), generate(), event("generate"), select(32), generate(), event("sec_cover", True), event("generate")], tmp)
        acts = {f["key"]: f["action"] for f in output(r[6])["frames"]}
        check(acts["cover"] == "replaced" and acts["master"] == "kept" and acts["changelog"] == "kept", "ticking the Cover in a rerun replaces it; the Master and the Changelog are kept", str(acts))
        r = run(copia, [select(32), generate()] + [event(k, False) for k in SECS] + [event("generate"), find(EN)], tmp)
        labels = [l for _, l in controls(r[-2])]
        check(output(r[-2])["refused"] and any(l.startswith("✖ No section is ticked") for l in labels) and r[-1]["result"]["total"] == 0, "no section ticked: refused with a ✖ note, nothing made")

        # 7. Go to Component Master.
        r = run(copia, [select(32), generate(), event("generate"), select(32), generate(), event("goto_master")], tmp)
        go = output(r[5])
        v = go["result"]
        check(go["found"] and v["view_after"]["zoom"] is not None and v["view_after"] != v["view_before"], "Go to Component Master: found, and the screen moved there", json.dumps({k: v[k] for k in ("view_before", "view_after")}))
        r = run(copia, [select(32), generate(), event("goto_master")], tmp)
        labels = [l for _, l in controls(r[2])]
        check(output(r[2])["found"] is False and any(l.startswith("✖ No Component Master found") for l in labels), "Go to Component Master with no Section: a ✖ note in the panel")

        # 8. A loose 0.10.0 documentation: a frame on the page with the old mark and no Section.
        legacy = {"command": "frame.create", "params": {"x": 5000, "y": 0, "width": 1280, "height": 400, "name": "Documentation — x", "select": False, "props": {"pluginData": {"handoff-report": "32"}}}}
        r = run(copia, [legacy, select(32), generate(direct=True)], tmp)
        check("legacy_doc" in output(r[2])["warnings"], "a loose 0.10.0 documentation is reported and left alone", str(output(r[2])["warnings"]))

        # 5c. Restore defaults keeps the language (the panel must not turn English), and saves it with the next click.
        r = run(copia, [select(32), generate(direct=True, lang="pt-BR", stage="dark"), select(32), generate(), event("defaults"), ui(), event("generate"), generate(), ui()], tmp)
        shown = r[5]["result"]
        labels = [l for _, l in controls(r[5])]
        check(shown["values"].get("lang") == "Português (Brasil)" and shown["values"].get("stage") == "Automático" and "Idioma" in labels, "Restore defaults keeps Portuguese and restores the rest", str(shown["values"].get("lang")))
        check(any(l.startswith("Palco e “ao gerar de novo” voltaram aos padrões") for l in labels), "the restore note is in Portuguese and says the language was kept")
        check(r[8]["result"]["values"].get("lang") == "Português (Brasil)", "after the click the language is still Portuguese (the page keeps `lang`)")

        # 5d. The panel has the same controls and labels before and after a document and a refusal.
        r = run(copia, [select(32), generate(), event("generate"), select(), event("generate")], tmp)
        a, b, c = controls(r[1]), controls(r[2]), controls(r[4])
        strip = lambda ctl: ctl[:-2] + ctl[-1:]  # without the state note
        check(len(a) == len(b) == len(c) == 30 and strip(a) == strip(b) == strip(c), "30 controls with the same labels when opened, after a document and after a refusal")

        # 6b. The order of the controls in the real panel, whatever is selected.
        for name, src, ids in [("no selection", copia, []), ("component", copia, [32]), ("variant", copia, [3])]:
            r = run(src, [select(*ids), generate()], tmp)
            try:
                check_order(controls(r[1]), name)
                print(f"[ok] order, {name}: {len(controls(r[1]))} controls, buttons after every field")
            except AssertionError as e:
                failures.append(str(e))
                print(f"[FAIL] {e}")

        # 9. A forced error: the instruction budget is too small for the callback, so the host rolls everything back.
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
    # The page is set to pt-BR by a direct run first (that also leaves a documentation, so the file is reopened
    # fresh for what comes next); then the panel is opened from the menu.
    r = run(copia, [select(32), pt, select(32), generate()], tmp)
    md("Aberto (com ou sem seleção: a lista é a mesma)", controls(r[3]))
    r = run(copia, [select(32), pt, select(32), generate(), event("version", "1.2"), event("status", "Aprovado"), event("generate")], tmp)
    md("Depois de gerar com sucesso (Versão e Status em branco)", controls(r[6]))
    r = run(copia, [select(32), pt, select(), generate(), event("generate")], tmp)
    md("Recusa: nada selecionado", controls(r[4]))
    r = run(copia, [select(32), pt, generate(), event("generate")], tmp)
    md("Recusa: seleção que não é componente (a documentação gerada pelo passo direto)", controls(r[3]))


if __name__ == "__main__":
    sys.exit(main())
