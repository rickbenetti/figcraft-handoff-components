#!/usr/bin/env python3
"""Regression net for Handoff Components: runs `generate` headless on a fixed set of cases and
compares what comes out (summary line, audited lines, sections, instances, the Anatomy's blocks, badges and counts, the variant rows' delta lines, fuel) with
tests/baseline.json. `--update [text]` rewrites the baseline from the current build (only the cases whose name
contains `text`, when given; the other entries are kept). `--dump FILE` writes the results to FILE and compares nothing.
`--private` runs only the cases in tests/private/ (a folder git ignores) and compares them with
tests/private/baseline-private.json; without the folder it says so and exits 0.

The cases named "pt-BR / ..." run the same way with the language set to pt-BR in the parameters (their summary,
deltas and block titles are in Portuguese). The cases without that prefix run in English (the default) and are the
net that the English report does not change.

The DS Base Demo cases run on demo/ds-base-demo.figcraft and find each component BY NAME every time (never by a fixed id).
Everything is written to a temporary folder (never over a fixture)."""
import json, os, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIST = os.path.join(ROOT, "dist")
FIX = os.path.join(ROOT, "tests/fixtures")
DEMO = os.path.join(ROOT, "demo/ds-base-demo.figcraft")
BASELINE = os.path.join(ROOT, "tests/baseline.json")
PRIVATE = os.path.join(ROOT, "tests/private")
BASELINE_PRIVATE = os.path.join(PRIVATE, "baseline-private.json")
PID = "me.benetti.handoff-components"
FUEL_TOLERANCE = 0.15
CLI = os.environ.get("FIGCRAFT_CLI", "figcraft-cli")

# name: (file, id of the layer to select, or name of the top-level component/set to look up)
CASES = {
    "copia-2 / id 32": (f"{FIX}/fixture-copia-2.figcraft", 32),
    "copia-2 / id 3 (Toast variant)": (f"{FIX}/fixture-copia-2.figcraft", 3),
    "carga-v2 / id 3 (Toast variant)": (f"{FIX}/fixture-carga-v2.figcraft", 3),
    "carga-v2 / id 194 (Card de carga)": (f"{FIX}/fixture-carga-v2.figcraft", 194),
}
# DS Base Demo: the name of the top-level component or set; the id is looked up by name on every run.
for _n in ["Button", "Card", "Accordion", "Code", "Icon"]:
    CASES[f"ds-base-demo / {_n}"] = (DEMO, _n)

# The same layers with the report in Brazilian Portuguese (added in 0.9.6; the English cases above are untouched).
# name: (file, id or name, language passed in the parameters)
PT_CASES = {
    "pt-BR / copia-2 / id 32": (f"{FIX}/fixture-copia-2.figcraft", 32, "pt-BR"),
    "pt-BR / carga-v2 / id 3 (Toast variant)": (f"{FIX}/fixture-carga-v2.figcraft", 3, "pt-BR"),
    "pt-BR / ds-base-demo / Button": (DEMO, "Button", "pt-BR"),
    "pt-BR / ds-base-demo / Card": (DEMO, "Card", "pt-BR"),
    "pt-BR / ds-base-demo / Accordion": (DEMO, "Accordion", "pt-BR"),
}

# Private cases: files that cannot be published, kept in tests/private/ (ignored by git).
PRIVATE_CASES = {
    "private / set 1 (id 20415)": (f"{PRIVATE}/handoff-demo.figcraft", 20415),
    "private / set 2 (id 70861)": (f"{PRIVATE}/handoff-demo.figcraft", 70861),
    "private / set 3 (id 60488)": (f"{PRIVATE}/handoff-demo.figcraft", 60488),
    "private / component 4 (id 77211)": (f"{PRIVATE}/handoff-demo.figcraft", 77211),
    "private / styles frame (id 99941)": (f"{PRIVATE}/fixture-estilos.figcraft", 99941),
}


def cli(args, timeout=300):
    return subprocess.run([CLI] + args, capture_output=True, text=True, timeout=timeout)


def find_by_name(src, name, tmp, n):
    """Id of the top-level component or component set called exactly `name` in `src` (None if there is none)."""
    script = os.path.join(tmp, f"find{n}.json")
    out = os.path.join(tmp, f"find{n}.figcraft")
    json.dump([{"command": "node.find", "params": {"name": name, "type": t, "limit": 100}} for t in ("component_set", "component")], open(script, "w"))
    p = cli(["run", "--in", src, "--out", out, script])
    ids = []
    for l in p.stdout.splitlines():
        if l.startswith("{"):
            r = json.loads(l)
            if r.get("ok"):
                ids += r["result"].get("ids", [])
    if not ids:
        return None
    json.dump([{"command": "node.get", "params": {"id": i, "fields": ["name", "id"]}} for i in ids], open(script, "w"))
    p = cli(["run", "--in", src, "--out", out, script])
    for l in p.stdout.splitlines():
        if l.startswith("{"):
            r = json.loads(l)
            if r.get("ok") and r["result"].get("name") == name:
                return r["result"]["id"]
    return None


def run_case(src, layer, tmp, n, lang=None):
    if isinstance(layer, str):
        found = find_by_name(src, layer, tmp, n)
        if found is None:
            return {"error": f"component {layer!r} not found in {os.path.basename(src)}"}
        layer = found
    script = os.path.join(tmp, f"case{n}.json")
    out = os.path.join(tmp, f"case{n}.figcraft")
    # `direct` makes the documentation with no panel (the menu sends no parameters and opens the panel).
    params = {"direct": True, **({"lang": lang} if lang else {})}
    steps = [
        {"command": "plugin.import", "params": {"path": DIST}},
        {"command": "select.set", "params": {"ids": [layer]}},
        {"command": "plugin.run", "params": {"id": PID, "command": "generate", "params": params}},
    ]
    json.dump(steps, open(script, "w"))
    p = cli(["run", "--in", src, "--out", out, script])
    lines = [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]
    run = lines[-1] if lines else {}
    if not run.get("ok"):
        return {"error": (run.get("error") or p.stderr or "no output")[:300]}
    o = run["result"]["output"]
    frame = o["frame"]
    # Sections are the report's frames named "... section" and "Axis: ...".
    steps = [
        {"command": "node.find", "params": {"root": frame, "type": "frame", "name": "section", "limit": 1000}},
        {"command": "node.find", "params": {"root": frame, "type": "frame", "name": "Axis:", "limit": 1000}},
    ]
    json.dump(steps, open(script, "w"))
    q = cli(["run", "--in", out, "--out", out + ".q", script])
    found = [json.loads(l) for l in q.stdout.splitlines() if l.startswith("{")]
    sections = sum(f["result"]["total"] for f in found if f.get("ok"))
    instances = 1 + (1 if o.get("spacing") else 0) + ((o.get("axes") or {}).get("rows") or 0)
    return {
        "summary": o["summary"],
        "lines": o["rows"]["in_table"],
        "sections": sections,
        "instances": instances,
        "fuel": run["result"]["fuel"],
        # The variant axes: for each row, the lines of what it changes from the default (the default row has none).
        "deltas": {st["name"]: st.get("deltas", []) for st in (o.get("axes") or {}).get("stages", [])},
        # The Anatomy: the legend's blocks in order (title, number of the badge or null), the numbers of the
        # badges on the stage, the "xN" counts and what was left without a number.
        "anatomy": {
            "blocks": [[b["title"], b["badge"]] for b in o.get("anatomy_blocks", [])],
            "badges": [m["n"] for m in o.get("badge_info", [])],
            "multiples": {b["title"]: b["count"] for b in o.get("anatomy_blocks", []) if b["count"] > 1},
            "unnumbered": o.get("unnumbered_layers", 0),
        },
    }


def main():
    args = sys.argv[1:]
    update = "--update" in args
    private = "--private" in args
    only = None
    if update:
        k = args.index("--update")
        if k + 1 < len(args) and not args[k + 1].startswith("--"):
            only = args[k + 1]
    dump = args[args.index("--dump") + 1] if "--dump" in args else None
    cases, baseline = {**{k: (*v, None) for k, v in CASES.items()}, **PT_CASES}, BASELINE
    if private:
        if not os.path.isdir(PRIVATE):
            print("tests/private/ does not exist: nothing to run (that folder is not part of the repository).")
            return 0
        cases, baseline = {k: (*v, None) for k, v in PRIVATE_CASES.items()}, BASELINE_PRIVATE
    if not os.path.isdir(DIST):
        sys.exit(f"build the plugin first: {DIST} is missing")
    started = time.time()
    results = {}
    with tempfile.TemporaryDirectory(prefix="handoff-regress-") as tmp:
        for n, (name, (src, layer, lang)) in enumerate(cases.items()):
            if update and only and only not in name:
                continue
            results[name] = run_case(src, layer, tmp, n, lang)
    secs = time.time() - started
    if dump:
        json.dump(results, open(dump, "w"), indent=2, ensure_ascii=False)
        print(f"results written: {dump} ({len(results)} cases, {secs:.1f} s)")
        return 0
    if update:
        if any("error" in r for r in results.values()):
            sys.exit("not writing the baseline: " + "; ".join(f"{k}: {v['error']}" for k, v in results.items() if "error" in v))
        merged = json.load(open(baseline)) if only and os.path.exists(baseline) else {}
        merged.update(results)
        json.dump(merged, open(baseline, "w"), indent=2, ensure_ascii=False)
        open(baseline, "a").write("\n")
        print(f"baseline written: {baseline} ({len(results)} cases rewritten, {len(merged)} in the file, {secs:.1f} s)")
        return 0
    base = json.load(open(baseline))
    diffs = []
    for name, got in results.items():
        want = base.get(name)
        if want is None:
            diffs.append(f"{name}: not in the baseline")
            continue
        if "error" in got:
            diffs.append(f"{name}: ERROR {got['error']}")
            continue
        for key in ("summary", "lines", "sections", "instances", "anatomy", "deltas"):
            if got[key] != want.get(key):
                diffs.append(f"{name}: {key}: baseline {want.get(key)!r}, now {got[key]!r}")
        wf, gf = want.get("fuel"), got["fuel"]
        if wf and abs(gf - wf) / wf > FUEL_TOLERANCE:
            diffs.append(f"{name}: fuel: baseline {wf}, now {gf} ({(gf - wf) / wf:+.0%}, tolerance {FUEL_TOLERANCE:.0%})")
    for name in base:
        if name not in results:
            diffs.append(f"{name}: in the baseline but not run")
    print(f"{len(results)} cases in {secs:.1f} s")
    if diffs:
        print("DIFFERENCES:")
        for d in diffs:
            print("  -", d)
        return 1
    print("no differences")
    return 0


if __name__ == "__main__":
    sys.exit(main())
