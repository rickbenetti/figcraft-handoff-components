#!/usr/bin/env python3
"""The "Generate documentation…" panel, driven the way the host drives it (headless, with the built dist/).

The host hands its events to the plugin in the order of the panel's controls; with the buttons AFTER the fields
a field's event comes first, so the button already sees the new value. This script plays that order:

  1. The menu (generate, no parameters) opens the panel and makes nothing; `direct: true` makes the
     documentation with no panel.
  2. Version="1.2" and Status="Aprovado" typed, then "Generate documentation": the documentation says
     "Versão: 1.2 · Status: Aprovado", the first click wrote it, Version and Status are empty again, the
     selection is what it was, and the screen is at the start of the documentation.
  3. Nothing selected: refused with a note, nothing in the document.
  4. A selection that is not a component (the documentation itself): refused with a note, nothing new.
  5. The line colour (and the fill opacity) typed, then the button: saved on the page on the first click; a fill opacity
     that is not a whole number from 0 to 100 is refused with a note; the fill's alpha is absolute.
  6. A forced error (an instruction budget too small for the callback): everything is rolled back.

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
    return {"command": "node.find", "params": {"type": "frame", "name": name, "limit": 50}}


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

    with tempfile.TemporaryDirectory(prefix="handoff-panel-") as tmp:
        copia = f"{FIX}/fixture-copia-2.figcraft"

        # 1. The menu opens the panel; `direct` makes the documentation with no panel.
        r = run(copia, [select(32), generate(), ui(), find("Documentation")], tmp)
        opened, panel, frames = r[1], r[2], r[3]
        ok = output(opened)["phase"] == "open" and bool(panel["result"]) and frames["result"]["total"] == 0
        check(ok, "generate with no parameters (the menu) opens the panel and makes nothing", f"{len(controls(panel))} controls, {frames['result']['total']} documentation frames")
        check_order(controls(panel), "menu")
        r = run(copia, [select(32), generate(direct=True), ui(), find("Documentation")], tmp)
        check(bool(output(r[1]).get("frame")) and not r[2]["result"] and r[3]["result"]["total"] == 1, "generate with direct: true makes the documentation and opens no panel")

        # 2. Version and Status typed, then the button; in Portuguese.
        r = run(
            copia,
            [select(32), generate(), event("lang", "Português (Brasil)"), event("version", "1.2"), event("status", "Aprovado"), event("generate"), ui(), event("stage", "Claro"), find("Documentação")],
            tmp,
        )
        click, after, seen, frames = r[5], r[6], r[7], r[8]
        out = output(click)
        values = after["result"]["values"]
        check(not out["refused"] and out["meta"] == {"version": "1.2", "status": "Aprovado"}, "Versão=1.2 and Status=Aprovado typed, then one click: made, with both texts", json.dumps(out["meta"], ensure_ascii=False))
        check(values.get("version") == "" and values.get("status") == "", "Version and Status are empty again after the documentation")
        check(output(seen)["selection_now"] == [32], "the selection is back to what it was before the click", str(output(seen)["selection_now"]))
        v = out["view"]
        check("error" not in v and v["zoom"] is not None and 0 < v["zoom"] <= 1.0, "the screen went to the documentation (zoom at most 100%)", json.dumps({k: v.get(k) for k in ("zoom", "cx", "cy", "vw", "vh")}))
        check(frames["result"]["total"] == 1, "one documentation frame named Documentação — …")
        msg = [l for _, l in controls(after)]
        done = [l for l in msg if l.startswith("✔ Documentação gerada: ")]
        check(bool(done), "the panel says ✔ Documentação gerada", done[0] if done else "")
        # The header line, read from the documentation (the frame is 50 in this fixture; its Header's texts follow).
        r = run(copia, [select(32), generate(direct=True, lang="pt-BR", version="1.2", status="Aprovado")] + [{"command": "node.get", "params": {"id": i, "fields": ["characters"]}} for i in range(52, 58)], tmp)
        lines = [x["result"].get("characters") for x in r[2:]]
        check("Versão: 1.2 · Status: Aprovado" in lines, "the header says «Versão: 1.2 · Status: Aprovado»", str([l for l in lines if l][:3]))
        r = run(copia, [select(32), generate(direct=True)] + [{"command": "node.get", "params": {"id": i, "fields": ["characters"]}} for i in range(52, 58)], tmp)
        lines = [x["result"].get("characters") for x in r[2:]]
        check("Version: to be defined · Status: to be defined" in lines, "with nothing typed the header says «to be defined» twice")

        # 3. Nothing selected.
        r = run(copia, [select(), generate(), event("generate"), find("Documentation")], tmp)
        out, frames = output(r[2]), r[3]
        labels = [l for _, l in controls(r[2])]
        check(out["refused"] and frames["result"]["total"] == 0 and any(l.startswith("✖ Nothing is selected") for l in labels), "nothing selected: refused with a ✖ note, nothing made")

        # 4. The selection is not a component: the documentation just made.
        r = run(copia, [select(32), generate(direct=True), generate(), event("generate"), find("Documentation")], tmp)
        out, frames = output(r[3]), r[4]
        labels = [l for _, l in controls(r[3])]
        refusal = [l for l in labels if l.startswith("✖")]
        check(out["refused"] and frames["result"]["total"] == 1 and any(l.startswith("✖ The selection is of type Frame") for l in labels), "a frame selected: refused with a ✖ note, nothing new made", refusal[0] if refusal else "")

        # 5. A field, then the button: written on the first click.
        r = run(copia, [select(32), generate(), event("line", "#112233"), event("generate"), generate(), ui()], tmp)
        reopened = r[5]["result"]["values"]
        check(reopened.get("line") == "#112233", "line colour typed, then one click: saved on the page (a reopened panel shows it)", str(reopened.get("line")))
        r = run(copia, [select(32), generate(), event("line", "#112233"), event("generate"), select(32), generate(), event("defaults"), event("generate"), generate(), ui()], tmp)
        check(r[9]["result"]["values"].get("line") == "#FCA311", "Restore defaults, then the button: the page's settings are cleared", str(r[9]["result"]["values"].get("line")))

        # 5b. Fill opacity: 50 typed, then one click: saved; 12.5 typed: refused with a note, nothing made.
        r = run(copia, [select(32), generate(), event("fill_opacity", 50), event("generate"), generate(), ui()], tmp)
        check(r[5]["result"]["values"].get("fill_opacity") == 50, "Fill opacity = 50 typed, then one click: saved on the page (a reopened panel shows 50)", str(r[5]["result"]["values"].get("fill_opacity")))
        r = run(copia, [select(32), generate(), event("fill_opacity", 12.5), event("generate"), find("Documentation")], tmp)
        labels = [l for _, l in controls(r[3])]
        check(output(r[3])["refused"] and r[4]["result"]["total"] == 0 and any(l.startswith("✖ The fill must be a whole number from 0 to 100") for l in labels), "Fill opacity = 12.5: refused with a ✖ note, nothing made, the field kept")
        r = run(copia, [select(32), generate(direct=True, fill_opacity=50, line="#11F2FC5B")], tmp)
        check(output(r[1])["settings"]["fill"] == "#11F2FC80", "direct: fill_opacity 50 on a line with its own alpha gives the absolute alpha 80", output(r[1])["settings"]["fill"])
        r = run(copia, [select(32), generate(direct=True, line="#11F2FC5B")], tmp)
        check(output(r[1])["settings"]["fill"] == "#11F2FC4D", "direct: the default 30 on #11F2FC5B gives #11F2FC4D")
        # 5b2. 0 makes no fill layer at all; 1, 30 and 100 give the absolute alpha 03, 4D and FF (direct mode and panel).
        def measure_fills(steps):
            rr = run(copia, steps + [{"command": "node.find", "params": {"name": "Measure", "limit": 200}}], tmp)
            ids = rr[-1]["result"]["ids"]
            gets = run(copia, steps + [{"command": "node.get", "params": {"id": i, "fields": ["fills"]}} for i in ids[:3]], tmp)
            return ids, [g["result"].get("fills") for g in gets[len(steps):]]

        for pct, want in [(0, None), (1, "#11F2FC03"), (30, "#11F2FC4D"), (100, "#11F2FCFF")]:
            rr = run(copia, [select(32), generate(direct=True, fill_opacity=pct, line="#11F2FC")], tmp)
            check(output(rr[1])["settings"]["fill"] == want, f"direct: fill_opacity {pct} -> settings.fill {want}", str(output(rr[1])["settings"]["fill"]))
            ids, fills = measure_fills([select(32), generate(direct=True, fill_opacity=pct, line="#11F2FC")])
            if want is None:
                check(bool(ids) and all(not f for f in fills), "fill_opacity 0: the measure rectangles have no fill layer", str(fills))
            else:
                check(bool(ids) and all(f and len(f) == 1 and str(f[0]).upper().find(want[1:7]) >= 0 for f in fills), f"fill_opacity {pct}: one fill layer on the measure rectangles", str(fills[:1]))
        ids, fills = measure_fills([select(32), generate(), event("fill_opacity", 0), event("generate")])
        check(bool(ids) and all(not f for f in fills), "panel: fill opacity 0 typed, then one click: no fill layer", str(fills))
        err = run(copia, [select(32), generate(direct=True, fill_opacity="101")], tmp, expect_error=True)
        check(err is not None and "0 to 100" in err.get("error", ""), "direct: fill_opacity 101 is refused with the dictionary's message", (err or {}).get("error", "")[:90])

        # 5b3. A synthetic component whose properties repeat their names (Title/Title, Message/Message, Show link/Show link,
        # with a text whose default is empty): the rows are numbered, in both languages, and a name that is alone is not.
        props = FIX + "/fixture-propriedades.figcraft"
        for lang, word, closable in (("en", "Property", "Closable"), ("pt-BR", "Propriedade", "Closable")):
            wanted = [f"{word}: {n}" for n in ("Title 1", "Title 2", "Message 1", "Message 2", "Show link 1", "Show link 2", closable)]
            rr = run(props, [select(2), generate(direct=True, lang=lang)] + [{"command": "node.find", "params": {"type": "frame", "name": w, "limit": 5}} for w in wanted] + [{"command": "node.find", "params": {"type": "frame", "name": f"{word}: Closable 1", "limit": 5}}], tmp)
            totals = [x["result"]["total"] for x in rr[2:]]
            check(totals[:-1] == [1] * 7 and totals[-1] == 0, f"repeated property names are numbered ({lang}): 6 numbered rows and Closable alone", str(totals))

        # 5b4. Swap properties show the name of their default component: "Glyph" points at a component of the file, "Gone" at an
        # id that does not exist (the Generate goes on, the chip says it was not found).
        for lang, word, nf in (("en", "Property", "(not found)"), ("pt-BR", "Propriedade", "(não encontrado)")):
            rr = run(props, [select(2), generate(direct=True, lang=lang)] + [{"command": "node.find", "params": {"type": "frame", "name": f"{word}: {n}", "limit": 5}} for n in ("Glyph", "Gone")], tmp)
            check(bool(output(rr[1]).get("frame")), f"swap property with an id that does not resolve: the Generate goes on ({lang})")
            texts = []
            for line in rr[2:]:
                row = line["result"]["ids"][0]
                level = [row]
                found = []
                while level:
                    got = run(props, [select(2), generate(direct=True, lang=lang)] + [{"command": "node.get", "params": {"id": i, "fields": ["type", "characters", "children"]}} for i in level], tmp)[2:]
                    nxt = []
                    for g in got:
                        r = g["result"]
                        if r.get("type") == "TEXT":
                            found.append(r.get("characters"))
                        nxt += [c["id"] for c in r.get("children", [])]
                    level = nxt
                texts.append(found)
            check(texts[0][-1] == "Icon/plus" and texts[1][-1] == nf, f"swap chips ({lang}): the default's name, and {nf} for an id that does not resolve", str(texts))

        # 5c. Restore defaults keeps the language (the panel must not turn English), and saves it with the next click.
        r = run(copia, [select(32), generate(direct=True, lang="pt-BR", line="#112233"), select(32), generate(), event("defaults"), ui(), event("generate"), generate(), ui()], tmp)
        shown = r[5]["result"]
        labels = [l for _, l in controls(r[5])]
        check(shown["values"].get("lang") == "Português (Brasil)" and shown["values"].get("line") == "#FCA311" and "Idioma" in labels, "Restore defaults keeps Portuguese and restores the rest", str(shown["values"].get("lang")))
        check(any(l.startswith("Cor da linha, preenchimento, palco e “ao rodar de novo” voltaram aos padrões") for l in labels), "the restore note is in Portuguese and says the language was kept")
        check(r[8]["result"]["values"].get("lang") == "Português (Brasil)", "after the click the language is still Portuguese (the page keeps `lang`)")

        # 5d. The panel has the same controls and labels before and after a documentation and a refusal.
        r = run(copia, [select(32), generate(), event("generate"), select(), event("generate")], tmp)
        a, b, c = controls(r[1]), controls(r[2]), controls(r[4])
        strip = lambda ctl: ctl[:-2] + ctl[-1:]  # without the state note
        check(len(a) == len(b) == len(c) == 20 and strip(a) == strip(b) == strip(c), "20 controls with the same labels when opened, after a documentation and after a refusal")

        # 6. The order of the controls in the real panel, whatever is selected.
        for name, src, ids in [("no selection", copia, []), ("component", copia, [32]), ("variant", copia, [3])]:
            r = run(src, [select(*ids), generate()], tmp)
            try:
                check_order(controls(r[1]), name)
                print(f"[ok] order, {name}: {len(controls(r[1]))} controls, buttons after every field")
            except AssertionError as e:
                failures.append(str(e))
                print(f"[FAIL] {e}")

        # 7. A forced error: the instruction budget is too small for the callback, so the host rolls everything back.
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
