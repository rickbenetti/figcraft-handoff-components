#!/usr/bin/env python3
"""Regression net for Handoff Components (0.11): runs `generate` headless on a fixed set of cases and compares what comes out
with tests/baseline.json: the frames of the Section (key, x, y, height), the Section's size, the rows of each table (Properties,
Anatomy, the matrix of each variant axis, Layouts), the warnings, the frames that failed, and fuel and host calls (5% tolerance).
It records no ids and no texts. `--update [text]` rewrites the baseline from the current build (only the cases whose name
contains `text`, when given; the other entries are kept). `--dump FILE` writes the results to FILE and compares nothing.
`--baseline FILE` compares with another baseline file. `--private` runs only the cases in tests/private/ (a folder git ignores; extra cases can be listed in tests/private/cases.json) and
compares them with tests/private/baseline-private.json; without the folder it says so and exits 0. A private baseline written by
an older version (without the "frames" key) is reported as old: regenerate it with `scripts/regress.sh --private --update`.

The cases named "pt-BR / ..." run the same way with the language set to pt-BR in the parameters.

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
FUEL_TOLERANCE = 0.05
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

# More private cases may be listed in tests/private/cases.json (that folder is ignored by git, so what it names never enters the
# repository): {"case name": ["file inside tests/private/", id of the component or set]}.
_EXTRA = os.path.join(PRIVATE, "cases.json")
if os.path.exists(_EXTRA):
    for _name, (_file, _id) in json.load(open(_EXTRA)).items():
        PRIVATE_CASES[_name] = (os.path.join(PRIVATE, _file), _id)


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


def marker_check(a):
    """What the Anatomy's markers must be (item 3 of Prompt 50), as numbers: how many, the nearest a centre is to the stage's edge (never
    under 48), the nearest two centres are (never under 36 + 8), and whether the component's own marker (the first one) is straight."""
    if not a or not a.get("markers"):
        return None
    sx, sy, sw, sh = a["stage"]
    ms = a["markers"]
    edge = min(min(m["centre"][0] - sx, sx + sw - m["centre"][0], m["centre"][1] - sy, sy + sh - m["centre"][1]) for m in ms)
    gap = min([((p["centre"][0] - q["centre"][0]) ** 2 + (p["centre"][1] - q["centre"][1]) ** 2) ** 0.5 for i, p in enumerate(ms) for q in ms[i + 1:]] or [999])
    root = next((m for m in ms if m["way"] == "Root"), None)
    straight = root is None or abs(root["centre"][0] - root["end"][0]) < 0.01 or abs(root["centre"][1] - root["end"][1]) < 0.01
    return {"count": len(ms), "edge_ok": edge >= 47.99, "gap_ok": gap >= 43.99, "root_straight": straight, "dropped": len(a.get("dropped", []))}


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
    return {
        "markers": marker_check(o.get("anatomy")),
        "frames": [[f["key"], f["x"], f["y"], f["h"]] for f in o["frames"]],
        "section": [o["section_size"]["w"], o["section_size"]["h"]],
        "tables": [[t["name"], t["rows"]] for t in o["tables"]],
        "warnings": o["warnings"],
        "failed": o["failed"],
        "fuel": run["result"]["fuel"],
        "calls": run["result"]["calls"],
        # Not in the baseline: the Section is born with one SVG export.
        "export": o["export"],
    }


UPDATES = ["all", "tables", "tokens", "properties", "anatomy", "variants", "layouts"]


def cover_snapshot(src, cover, tmp):
    """Everything inside the Cover frame: id, parent, name, text and, for an instance, its properties; plus its place."""
    nodes, level = [], [(cover, None)]
    while level:
        steps = [{"command": "node.get", "params": {"id": i, "fields": ["name", "type", "characters", "children", "x", "y", "width", "height"]}} for i, _ in level]
        json.dump(steps, open(os.path.join(tmp, "cs.json"), "w"))
        p = cli(["run", "--in", src, "--out", os.path.join(tmp, "cs.figcraft"), os.path.join(tmp, "cs.json")])
        res = [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]
        current, level = level, []
        for (i, parent), r in zip(current, res):
            v = r.get("result") or {}
            nodes.append([i, parent, v.get("name"), v.get("type"), v.get("characters"), v.get("x"), v.get("y"), v.get("width"), v.get("height")])
            level += [(c["id"], i) for c in v.get("children") or []]
    inst = [n[0] for n in nodes if n[3] == "INSTANCE"]
    props = []
    if inst:
        json.dump([{"command": "component.properties", "params": {"id": i}} for i in inst], open(os.path.join(tmp, "cs.json"), "w"))
        p = cli(["run", "--in", src, "--out", os.path.join(tmp, "cs.figcraft"), os.path.join(tmp, "cs.json")])
        props = [json.dumps(json.loads(l).get("result"), sort_keys=True) for l in p.stdout.splitlines() if l.startswith("{")]
    return nodes, props


def cover_case(tmp):
    """"cover editada": the Button of the demo is documented, its Cover is edited by hand (texts, a pill's area, name and state, a
    pill deleted, another duplicated), and each of the 7 options of Update is run from that same state. The Cover must come out the same:
    same id, same nodes (ids, names, texts, properties of every pill, places). Also: selecting a pill and generating is refused."""
    out = {}
    name = "Button"
    cid = find_by_name(DEMO, name, tmp, 900)
    if cid is None:
        return {"cover editada": {"error": "Button not found"}}
    step = lambda cmd, params: {"command": cmd, "params": params}

    def plugin(layer, params):
        return [step("plugin.import", {"path": DIST}), step("select.set", {"ids": [layer]}), step("plugin.run", {"id": PID, "command": "generate", "params": {"direct": True, **params}})]

    def go(steps, i, o):
        json.dump(steps, open(os.path.join(tmp, "cv.json"), "w"))
        p = cli(["run", "--in", i, "--out", o, os.path.join(tmp, "cv.json")])
        return [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")], p.stderr

    g1 = os.path.join(tmp, "cv1.figcraft")
    r, _ = go(plugin(cid, {}), DEMO, g1)
    o = r[2]["result"]["output"]
    cover = next(f["id"] for f in o["frames"] if f["key"] == "cover")
    find = lambda **q: step("node.find", {"root": cover, "limit": 100, **q})
    r, _ = go([find(text="v0.0.0"), find(text="status:"), find(text="Write a short"), find(text="Name of the last"), find(type="instance")], g1, os.path.join(tmp, "cv0.figcraft"))
    ids = [x["result"]["ids"] for x in r]
    pills = ids[4]
    edit = [step("node.set", {"ids": [ids[0][0]], "props": {"characters": "v1.2.3"}}), step("node.set", {"ids": [ids[1][0]], "props": {"characters": "status: approved"}}),
            step("node.set", {"ids": [ids[2][0]], "props": {"characters": "Edited description"}}), step("node.set", {"ids": [ids[3][0]], "props": {"characters": "Rick B."}}),
            step("instance.setProperties", {"ids": [pills[0]], "properties": {"Area": "Design lead", "Name · date": "Rick · 17/11", "Estado": "aprovado"}}),
            step("edit.delete", {"ids": [pills[1]]}), step("edit.duplicate", {"ids": [pills[2]]})]
    g2 = os.path.join(tmp, "cv2.figcraft")
    r, err = go(edit, g1, g2)
    if not all(x["ok"] for x in r):
        return {"cover editada": {"error": f"the edit failed: {err[:200]}"}}
    before = cover_snapshot(g2, cover, tmp)
    out["cover editada / pills before"] = {"nodes": len(before[0]), "pills": len(before[1])}
    for opt in UPDATES:
        dst = os.path.join(tmp, f"cv_{opt}.figcraft")
        r, _ = go(plugin(cid, {"update": opt}), g2, dst)
        if not r or not r[-1]["ok"]:
            out[f"cover editada / {opt}"] = {"error": str((r[-1] if r else {}).get("error"))[:200]}
            continue
        o = r[2]["result"]["output"]
        f = next(f for f in o["frames"] if f["key"] == "cover")
        after = cover_snapshot(dst, cover, tmp) if f["id"] == cover else ([], [])
        out[f"cover editada / {opt}"] = {"cover_id_same": f["id"] == cover, "action": f["action"], "nodes_same": before[0] == after[0], "pills_same": before[1] == after[1], "pills": len(after[1])}
    # a pill selected: refused, and nothing else is made
    r, err = go(plugin(pills[0], {}), g2, os.path.join(tmp, "cv_pill.figcraft"))
    refused = len(r) <= 2 or not r[2]["ok"]
    out["cover editada / a pill selected"] = {"refused": refused, "mentions_support": "support component" in (err + json.dumps(r[-1:]))}
    # The support set: a new Section names it with a leading dot; a Section made before (the name without it) is found by its mark: its Cover,
    # deleted by hand, is made again with six pills of that very set, and the set keeps its name.
    set_id = r_set = None
    r, _ = go([step("node.get", {"id": cover, "fields": ["name"]}), step("node.find", {"root": cover, "type": "instance", "limit": 1})], g1, os.path.join(tmp, "cvs.figcraft"))
    inst = r[1]["result"]["ids"][0]
    r, _ = go([step("component.properties", {"id": inst})], g1, os.path.join(tmp, "cvs.figcraft"))
    set_id = r[0]["result"].get("set")
    r, _ = go([step("node.get", {"id": set_id, "fields": ["name"]})], g1, os.path.join(tmp, "cvs.figcraft"))
    new_name = r[0]["result"]["name"]
    old_name = new_name.lstrip(".")
    r, _ = go([step("node.set", {"ids": [set_id], "props": {"name": old_name}}), step("edit.delete", {"ids": [cover]})] + plugin(cid, {})[0:3] + [step("node.get", {"id": set_id, "fields": ["name"]}), step("node.find", {"type": "instance", "name": old_name, "limit": 100})], g1, os.path.join(tmp, "cvs2.figcraft"))
    ok = r[-1]["ok"] and r[-2]["ok"]
    now_name = r[-2]["result"]["name"] if ok else None
    out["cover editada / support set"] = {"new_section_name_has_dot": new_name.startswith("."), "old_name_kept": now_name == old_name, "pills_of_the_old_set": len(r[-1]["result"]["ids"]) if ok else None}
    return out


def support_case(tmp):
    """"apoio compartilhado": the Button and then the Card of the demo are documented in the same file. There is ONE set of approval pills
    (in the Support frame, outside every Section), the Card's Section has eight frames and no Support frame, the second Cover has six pills of
    the same set, the Support frame stands to the right of everything and the Sections to the right of it."""
    step = lambda cmd, params: {"command": cmd, "params": params}

    def go(steps, i, o):
        json.dump(steps, open(os.path.join(tmp, "sp.json"), "w"))
        p = cli(["run", "--in", i, "--out", o, os.path.join(tmp, "sp.json")])
        return [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]

    def plugin(layer):
        return [step("plugin.import", {"path": DIST}), step("select.set", {"ids": [layer]}), step("plugin.run", {"id": PID, "command": "generate", "params": {"direct": True}})]

    ids = {}
    for n, name in enumerate(["Button", "Card"]):
        ids[name] = find_by_name(DEMO, name, tmp, 910 + n)
    a, b = os.path.join(tmp, "sp1.figcraft"), os.path.join(tmp, "sp2.figcraft")
    r1 = go(plugin(ids["Button"]), DEMO, a)
    o1 = r1[2]["result"]["output"]
    r2 = go(plugin(ids["Card"]), a, b)
    o2 = r2[2]["result"]["output"]
    sets = go([step("node.find", {"name": "Handoff", "type": "component_set", "limit": 50})], b, os.path.join(tmp, "sp3.figcraft"))[0]["result"]["ids"]
    cover = next(f["id"] for f in o2["frames"] if f["key"] == "cover")
    inst = go([step("node.find", {"root": cover, "type": "instance", "limit": 20})], b, os.path.join(tmp, "sp3.figcraft"))[0]["result"]["ids"]
    props = go([step("component.properties", {"id": i}) for i in inst], b, os.path.join(tmp, "sp3.figcraft"))
    made = o1["support"]["made"]
    return {"apoio compartilhado": {
        "one_set": len(sets) == 1,
        "second_cover_pills_of_the_same_set": len(inst) == 6 and all((p.get("result") or {}).get("set") == o1["support"]["set"] for p in props),
        "card_support_made": o2["support"]["made"] is not None,
        "frames_of_the_card_section": [f["key"] for f in o2["frames"]],
        "support_right_of_everything_before_the_sections": bool(made) and o1["x"] > made["x"] + made["w"] and o2["x"] > o1["x"] + o1["section_size"]["w"],
    }}


def master_case(tmp):
    """"master": a set wider than the 988 the Master has has its variants laid out again in a grid (the Card of the demo: 3 variants side by side,
    1528 wide, go into 2 columns); a second run changes nothing (the set is where it was left); the frames stay 1180 wide. A component wider
    than 988 that cannot be laid out (a single 1100-wide component) makes the Master and every frame of the Section as wide, with the warning."""
    step = lambda cmd, params: {"command": cmd, "params": params}

    def go(steps, i, o):
        json.dump(steps, open(os.path.join(tmp, "ms.json"), "w"))
        p = cli(["run", "--in", i, "--out", o, os.path.join(tmp, "ms.json")])
        return [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]

    def plugin(layer, params=None):
        return [step("plugin.import", {"path": DIST}), step("select.set", {"ids": [layer]}), step("plugin.run", {"id": PID, "command": "generate", "params": {"direct": True, **(params or {})}})]

    def boxes(f, ids):
        return [x["result"]["absoluteBounds"] for x in go([step("node.get", {"id": i, "fields": ["absoluteBounds"]}) for i in ids], f, os.path.join(tmp, "ms_t.figcraft"))]

    card = find_by_name(DEMO, "Card", tmp, 920)
    a, b = os.path.join(tmp, "ms1.figcraft"), os.path.join(tmp, "ms2.figcraft")
    o1 = go(plugin(card), DEMO, a)[2]["result"]["output"]
    kids = go([step("node.get", {"id": card, "fields": ["children"]})], a, os.path.join(tmp, "ms_t.figcraft"))[0]["result"]["children"]
    ids = [card] + [k["id"] for k in kids]
    after1 = boxes(a, ids)
    o2 = go(plugin(card, {"update": "all"}), a, b)[2]["result"]["output"]
    after2 = boxes(b, ids)
    widths = sorted({f["w"] for f in o1["frames"]})
    out = {"master / card": {"plan": o1["master"]["plan"], "moved": o1["master"]["moved"], "columns": o1["master"]["grid"]["columns"], "frame_widths": widths, 
                             "set_width_fits": after1[0]["width"] <= 988, "second_run_plan": o2["master"]["plan"], "second_run_moved": o2["master"]["moved"], "second_run_set_unchanged": after1 == after2}}
    w = os.path.join(tmp, "ms3.figcraft")
    fid = go([step("frame.create", {"x": 9000, "y": 0, "width": 1100, "height": 200, "name": "WideThing", "props": {"fill": "#FFD1DC"}})], DEMO, w)[0]["result"]["id"]
    go([step("component.create", {"ids": [fid]})], w, os.path.join(tmp, "ms4.figcraft"))
    o3 = go(plugin(fid), os.path.join(tmp, "ms4.figcraft"), os.path.join(tmp, "ms5.figcraft"))[2]["result"]["output"]
    out["master / wide"] = {"plan": o3["master"]["plan"], "frame_widths": sorted({f["w"] for f in o3["frames"]}), "warnings": o3["warnings"], "section_w": o3["section_size"]["w"]}
    return out


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
    if "--baseline" in args:
        baseline = args[args.index("--baseline") + 1]
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
        if not private and not (update and only and only not in "cover editada"):
            results.update(cover_case(tmp))
            results.update(support_case(tmp))
            results.update(master_case(tmp))
    secs = time.time() - started
    if dump:
        json.dump(results, open(dump, "w"), indent=2, ensure_ascii=False)
        print(f"results written: {dump} ({len(results)} cases, {secs:.1f} s)")
        return 0
    if update:
        if any("error" in r for r in results.values()):
            sys.exit("not writing the baseline: " + "; ".join(f"{k}: {v['error']}" for k, v in results.items() if "error" in v))
        merged = json.load(open(baseline)) if only and os.path.exists(baseline) else {}
        merged.update({k: {f: v for f, v in r.items() if f != "export"} for k, r in results.items()})
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
        if name.startswith(("cover editada", "apoio compartilhado", "master /")):
            if got != want:
                diffs.append(f"{name}: baseline {want!r}, now {got!r}")
            continue
        if "frames" not in want:
            diffs.append(f"{name}: the baseline entry is in the old format: regenerate it with --update")
            continue
        for key in ("frames", "section", "tables", "warnings", "failed", "markers"):
            if got[key] != want.get(key):
                diffs.append(f"{name}: {key}: baseline {want.get(key)!r}, now {got[key]!r}")
        # A private file may already hold a Section of the component (the person generated it in the app): then the run is a rerun and the
        # Section's export is kept.
        if got["export"] != "ok" and not (private and got["export"] == "kept"):
            diffs.append(f"{name}: export: expected 'ok', now {got['export']!r}")
        for key in ("fuel", "calls"):
            wf, gf = want.get(key), got[key]
            if wf and abs(gf - wf) / wf > FUEL_TOLERANCE:
                diffs.append(f"{name}: {key}: baseline {wf}, now {gf} ({(gf - wf) / wf:+.0%}, tolerance {FUEL_TOLERANCE:.0%})")
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
