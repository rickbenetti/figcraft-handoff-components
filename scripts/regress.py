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
# repository): {"case name": ["file inside tests/private/", id of the component or set]}. The key "_second_component" is the case of components
# documented one after the other in one private file: {"name": "case name", "file": "file inside tests/private/", "picks": [[id, "language"], ...]}.
_EXTRA = os.path.join(PRIVATE, "cases.json")
_SECOND = None
if os.path.exists(_EXTRA):
    _extra = json.load(open(_EXTRA))
    _SECOND = _extra.pop("_second_component", None)
    for _name, (_file, _id) in _extra.items():
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


def _seg_rect(p, q, r, shrink=0.5):
    """Whether the segment p-q passes through the rectangle r = [x, y, w, h] (shrunk a hair, so touching is fine)."""
    x0, y0, x1, y1 = r[0] + shrink, r[1] + shrink, r[0] + r[2] - shrink, r[1] + r[3] - shrink
    if x1 <= x0 or y1 <= y0:
        return False
    t0, t1 = 0.0, 1.0
    for a, d, lo, hi in ((p[0], q[0] - p[0], x0, x1), (p[1], q[1] - p[1], y0, y1)):
        if abs(d) < 1e-9:
            if a < lo or a > hi:
                return False
        else:
            ta, tb = (lo - a) / d, (hi - a) / d
            if ta > tb:
                ta, tb = tb, ta
            t0, t1 = max(t0, ta), min(t1, tb)
            if t0 > t1:
                return False
    return True


def _cross(a, b, c, d):
    def side(p, q, r):
        return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    d1, d2, d3, d4 = side(c, d, a), side(c, d, b), side(a, b, c), side(a, b, d)
    e = 1e-6
    return ((d1 > e and d2 < -e) or (d1 < -e and d2 > e)) and ((d3 > e and d4 < -e) or (d3 < -e and d4 > e))


def marker_rows(a):
    """One row per marker, read from what the plugin answered: side and distance from the component, the guide's angle, where its tip ends
    (in / edge / out of its layer), whether the tip is inside a text, which other texts the guide runs over and which guides it crosses."""
    ms = a["markers"]
    cx, cy, cw, ch = a["component"]
    texts = a.get("texts") or []
    rows = []
    for m in ms:
        c, tip = m["centre"], m["tip"]
        dx, dy = tip[0] - c[0], tip[1] - c[1]
        d = max((dx * dx + dy * dy) ** 0.5, 0.001)
        start = (c[0] + dx / d * 18, c[1] + dy / d * 18)
        ang = abs(__import__("math").degrees(__import__("math").atan2(-dy, dx))) % 180
        straight = min(ang, abs(ang - 90), abs(ang - 180)) < 0.5
        lx, ly, lw, lh = m.get("layer", [0, 0, 0, 0])
        inside = lx + 0.5 < tip[0] < lx + lw - 0.5 and ly + 0.5 < tip[1] < ly + lh - 0.5
        edge = not inside and lx - 0.5 <= tip[0] <= lx + lw + 0.5 and ly - 0.5 <= tip[1] <= ly + lh + 0.5
        if c[1] < cy:
            side, dist = "Top", cy - c[1]
        elif c[1] > cy + ch:
            side, dist = "Bottom", c[1] - (cy + ch)
        elif c[0] < cx:
            side, dist = "Left", cx - c[0]
        else:
            side, dist = "Right", c[0] - (cx + cw)
        under = lambda t: max(0, min(lx + lw, t[0] + t[2]) - max(lx, t[0])) * max(0, min(ly + lh, t[1] + t[3]) - max(ly, t[1])) >= 0.5 * lw * lh and lw * lh > 0
        own = lambda t: abs(t[0] - lx) < 0.6 and abs(t[1] - ly) < 0.6 and abs(t[2] - lw) < 0.6 and abs(t[3] - lh) < 0.6
        tip_text = any(not own(t) and not under(t) and t[0] + 0.5 < tip[0] < t[0] + t[2] - 0.5 and t[1] + 0.5 < tip[1] < t[1] + t[3] - 0.5 for t in texts) or (any(own(t) for t in texts) and inside)
        over = sum(1 for t in texts if not own(t) and not under(t) and _seg_rect(start, tip, t))
        rows.append({"n": m["n"], "way": m["way"], "side": side, "dist": round(dist, 1), "angle": round(ang, 1), "straight": straight,
                     "tip": "in" if inside else ("edge" if edge else "out"), "tip_in_text": tip_text, "over_text": over, "start": start, "end": tip, "centre": c, "layer": [lx, ly, lw, lh]})
    for i, r in enumerate(rows):
        r["crosses"] = [q["n"] for j, q in enumerate(rows) if j != i and _cross(r["start"], r["end"], q["start"], q["end"])]
    return rows


def marker_check(a):
    """What the Anatomy's markers must be, as numbers: how many, the nearest a centre is to the stage's edge (never under 48), `min_gap`
    (the least gap between two circles, at least 12), the guides that are not horizontal or vertical (`oblique`, 0), the tips inside a text
    (`tip_in_text`, 0), the guides that cross another guide or run over another text (`cross`, 0), and whether the component's own marker
    (the first one) is straight."""
    if not a or not a.get("markers"):
        return None
    sx, sy, sw, sh = a["stage"]
    ms = a["markers"]
    rows = marker_rows(a)
    edge = min(min(m["centre"][0] - sx, sx + sw - m["centre"][0], m["centre"][1] - sy, sy + sh - m["centre"][1]) for m in ms)
    gap = min([((p["centre"][0] - q["centre"][0]) ** 2 + (p["centre"][1] - q["centre"][1]) ** 2) ** 0.5 for i, p in enumerate(ms) for q in ms[i + 1:]] or [999]) - 36
    root = next((m for m in ms if m["way"] == "Root"), None)
    straight = root is None or abs(root["centre"][0] - root["end"][0]) < 0.01 or abs(root["centre"][1] - root["end"][1]) < 0.01
    return {"count": len(ms), "edge_ok": edge >= 47.99, "min_gap": round(min(gap, 99), 1), "root_straight": straight, "dropped": len(a.get("dropped", [])),
            "oblique": sum(1 for r in rows if not r["straight"]), "tip_in_text": sum(1 for r in rows if r["tip_in_text"]),
            "cross": sum(len(r["crosses"]) for r in rows) // 2 + sum(r["over_text"] for r in rows)}


def spacing_check(a):
    """What the Layouts drawings must be, as numbers. Each level has a drawing; `groups` is a number for each set of equal bands, `inside`
    (in a band), `outside` (one band, with a guide), `spines` (two or more bands, a spine) and `dropped` (bands with no number). Checked on
    every drawing: `badge_over_content` (a number whose patch or marker meets a text, a vector or an instance with nothing in it: 0),
    `guide_over_content` (a guide, spine or trace within 1.9 px of one: 0; the guide is 2 px thick), `cross` (guides that cross each other:
    0), `oblique` (guides that are not horizontal or vertical: 0), `stack_overlap` (bands of two layers over each other in one drawing: 0), `spine_cross` (a guide, spine or trace through a number: 0)
    and `min_gap` (the least gap between two markers outside, at least 12)."""
    if not a or "list" not in a:
        return None
    rects = lambda r: (r[0] + 0.5, r[1] + 0.5, r[2] - 1.0, r[3] - 1.0)
    over = lambda p, r: p[0] < r[0] + r[2] and r[0] < p[0] + p[2] and p[1] < r[1] + r[3] and r[1] < p[1] + p[3]
    badge_over = guide_over = cross = oblique = stack = spine_cross = 0
    gap = 999.0
    for d in a["list"]:
        content = d.get("content") or []
        badge_over += sum(1 for b in d["badges"] if any(over(rects(b["rect"]), c) for c in content))
        guides = d.get("guides") or []
        # A content box the guide's tip lies in (a big image instance the band is drawn on) is one the guide has to enter.
        guide_over += sum(1 for g in guides for c in content if _seg_rect((g[0], g[1]), (g[2], g[3]), c, -1.9) and not (c[0] - 0.5 <= g[2] <= c[0] + c[2] + 0.5 and c[1] - 0.5 <= g[3] <= c[1] + c[3] + 0.5))
        cross += sum(1 for i, g in enumerate(guides) for h in guides[i + 1:] if _cross((g[0], g[1]), (g[2], g[3]), (h[0], h[1]), (h[2], h[3])))
        oblique += sum(1 for g in guides if abs(g[0] - g[2]) > 0.01 and abs(g[1] - g[3]) > 0.01)
        # A spine, stub, trace or guide that runs through a number (its own stub ends on its edge: that is not through).
        spine_cross += sum(1 for g in guides for b in d["badges"] if not b["inside"] and _seg_rect((g[0], g[1]), (g[2], g[3]), b["rect"], 1.0))
        bands = d["list"]
        stack += sum(1 for i, p in enumerate(bands) for q in bands[i + 1:] if p["layer"] != q["layer"] and over(rects(p["rect"]), rects(q["rect"])))
        # Each marker is a capsule (a circle is one with a segment of length 0): the gap between two is the distance between their segments
        # less the two radii.
        caps = [(b["rect"][0] + b["rect"][3] / 2, b["rect"][0] + b["rect"][2] - b["rect"][3] / 2, b["rect"][1] + b["rect"][3] / 2, b["rect"][3] / 2) for b in d["badges"] if not b["inside"]]
        for i, p in enumerate(caps):
            for q in caps[i + 1:]:
                dx = max(0.0, q[0] - p[1], p[0] - q[1])
                gap = min(gap, (dx * dx + (p[2] - q[2]) ** 2) ** 0.5 - p[3] - q[3])
    return {"levels": a["levels"], "drawings": a["drawings"], "deeper": a["deeper"], "groups": a["groups"], "inside": a["inside"], "outside": a["outside"],
            "spines": a["spines"], "dropped": a.get("outside_dropped", 0), "badge_over_content": badge_over, "guide_over_content": guide_over, "cross": cross,
            "oblique": oblique, "stack_overlap": stack, "spine_cross": spine_cross, "min_gap": round(min(gap, 99), 1)}


FRAME_PAD, FRAME_GAP, SECTION_PAD = 48, 64, 64


def frame_invariants(out_file, o, tmp, n):
    """The frames of a Section are laid out from heights READ from the file: (a) the bottom of the content is at most the frame's bottom
    minus its padding (48), (b) the space between consecutive frames is exactly 64, (c) no frame overlaps another, (d) the Section is
    64 + the frames + the 64 between them + 64 tall. Returns the list of violations (empty when all hold)."""
    frames = o["frames"]
    script = os.path.join(tmp, f"inv{n}.json")
    json.dump([{"command": "node.get", "params": {"id": f["id"], "fields": ["children", "y", "height"]}} for f in frames], open(script, "w"))
    p = cli(["run", "--in", out_file, "--out", out_file + ".inv", script])
    res = [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]
    bad = []
    if len(res) != len(frames):
        return ["could not read the frames back"]
    rows = []
    for f, r in zip(frames, res):
        v = r.get("result") or {}
        content = max([c["y"] + c["height"] for c in v.get("children") or []] or [0])
        rows.append((f["key"], v.get("y"), v.get("height"), content))
    for k, y, h, content in rows:
        if content > h - FRAME_PAD + 0.5:
            bad.append(f"{k}: content bottom {content} is past {h - FRAME_PAD} (frame {h} - {FRAME_PAD}): {content - (h - FRAME_PAD):+.0f} px")
    ordered = sorted(rows, key=lambda r: r[1])
    for a, b in zip(ordered, ordered[1:]):
        gap = b[1] - (a[1] + a[2])
        if gap < -0.5:
            bad.append(f"{a[0]} overlaps {b[0]} by {-gap:.0f} px")
        elif abs(gap - FRAME_GAP) > 0.5:
            bad.append(f"space between {a[0]} and {b[0]} is {gap:.0f}, not {FRAME_GAP}")
    expect = SECTION_PAD + sum(r[2] for r in rows) + FRAME_GAP * (len(rows) - 1) + SECTION_PAD
    if abs(o["section_size"]["h"] - expect) > 0.5:
        bad.append(f"Section height {o['section_size']['h']}, formula {expect}")
    return bad


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
        "invariants": frame_invariants(out, o, tmp, n),
        "markers": marker_check(o.get("anatomy")),
        "spacing": spacing_check(o.get("spacing")),
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
    """"master": a set wider than the 988 the Master has is given auto layout, horizontal, with wrap (the Card of the demo: 3 variants side by
    side, 1528 wide, are laid out in lines by the host inside a set 988 wide that hugs its height); a second run changes nothing (the set is
    already 988 wide); the frames stay 1180 wide. A component wider than 988 that cannot be laid out (a single 1100-wide component) makes the
    Master and every frame of the Section as wide, with the warning."""
    step = lambda cmd, params: {"command": cmd, "params": params}

    def go(steps, i, o):
        json.dump(steps, open(os.path.join(tmp, "ms.json"), "w"))
        p = cli(["run", "--in", i, "--out", o, os.path.join(tmp, "ms.json")])
        return [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]

    def plugin(layer, params=None):
        return [step("plugin.import", {"path": DIST}), step("select.set", {"ids": [layer]}), step("plugin.run", {"id": PID, "command": "generate", "params": {"direct": True, **(params or {})}})]

    def read(f, ids):
        return [x["result"] for x in go([step("node.get", {"id": i, "fields": ["absoluteBounds", "layout", "sizing"]}) for i in ids], f, os.path.join(tmp, "ms_t.figcraft"))]

    card = find_by_name(DEMO, "Card", tmp, 920)
    a, b = os.path.join(tmp, "ms1.figcraft"), os.path.join(tmp, "ms2.figcraft")
    o1 = go(plugin(card), DEMO, a)[2]["result"]["output"]
    kids = go([step("node.get", {"id": card, "fields": ["children"]})], a, os.path.join(tmp, "ms_t.figcraft"))[0]["result"]["children"]
    ids = [card] + [k["id"] for k in kids]
    after1 = read(a, ids)
    o2 = go(plugin(card, {"update": "all"}), a, b)[2]["result"]["output"]
    after2 = read(b, ids)
    widths = sorted({f["w"] for f in o1["frames"]})
    st = after1[0]
    inside = all(k["absoluteBounds"]["x"] >= st["absoluteBounds"]["x"] and k["absoluteBounds"]["x"] + k["absoluteBounds"]["width"] <= st["absoluteBounds"]["x"] + st["absoluteBounds"]["width"] for k in after1[1:])
    out = {"master / card": {"plan": o1["master"]["plan"], "organized": o1["master"]["organized"], "per_row": o1["master"]["per_row"], "frame_widths": widths,
                             "set": {"mode": st["layout"]["mode"], "wrap": st["layout"]["wrap"], "sizing": st["sizing"], "w": st["absoluteBounds"]["width"], "padding": st["layout"]["padding"], "gap": st["layout"]["gap"], "cross_gap": st["layout"]["crossGap"]},
                             "variants_inside_the_set": inside, "second_run_plan": o2["master"]["plan"], "second_run_organized": o2["master"]["organized"], "second_run_set_unchanged": after1 == after2}}
    w = os.path.join(tmp, "ms3.figcraft")
    fid = go([step("frame.create", {"x": 9000, "y": 0, "width": 1100, "height": 200, "name": "WideThing", "props": {"fill": "#FFD1DC"}})], DEMO, w)[0]["result"]["id"]
    go([step("component.create", {"ids": [fid]})], w, os.path.join(tmp, "ms4.figcraft"))
    o3 = go(plugin(fid), os.path.join(tmp, "ms4.figcraft"), os.path.join(tmp, "ms5.figcraft"))[2]["result"]["output"]
    out["master / wide"] = {"plan": o3["master"]["plan"], "frame_widths": sorted({f["w"] for f in o3["frames"]}), "warnings": o3["warnings"], "section_w": o3["section_size"]["w"]}
    return out


def second_component_case(tmp, label, src, picks):
    """"2º componente no mesmo arquivo": components are documented one after the other in the SAME copy of a file, the first in English and
    the next ones in Brazilian Portuguese (the person changed the plugin's language between runs), then a "New Section" of the second one.
    The set of approval pills made by the first run has English property names ("Area"); the others must find it by its mark and write
    by ITS names. Nothing may fail, no run may error, and at the end there is exactly ONE set of approval pills in the file.
    `picks`: [(name or id, language)], the last one is run again with `new_section`."""
    step = lambda cmd, params: {"command": cmd, "params": params}

    def go(steps, i, o):
        json.dump(steps, open(os.path.join(tmp, "sc.json"), "w"))
        p = cli(["run", "--in", i, "--out", o, os.path.join(tmp, "sc.json")])
        return [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")], (p.stdout + p.stderr)

    runs = [(layer, lang, {}) for layer, lang in picks] + [(picks[1][0], picks[1][1], {"new_section": True})]
    cur, runs_out = src, []
    for n, (layer, lang, extra) in enumerate(runs):
        if isinstance(layer, str):
            layer = find_by_name(cur, layer, tmp, 940 + n)
            if layer is None:
                return {label: {"error": f"component {picks[min(n, len(picks) - 1)][0]!r} not found"}}
        nxt = os.path.join(tmp, f"sc{n}.figcraft")
        steps = [step("plugin.import", {"path": DIST}), step("select.set", {"ids": [layer]}), step("plugin.run", {"id": PID, "command": "generate", "params": {"direct": True, "lang": lang, **extra}})]
        r, raw = go(steps, cur, nxt)
        if len(r) < 3 or not r[2].get("ok"):
            return {label: {"error": f"run {n + 1} failed: {raw.strip()[-250:]}"}}
        o = r[2]["result"]["output"]
        runs_out.append({"lang": lang, "new_section": o["new_section"], "failed": o["failed"], "warnings": o["warnings"], "sets_found": o["support"]["sets_found"], "area_retries": o["area_retries"]})
        cur = nxt
    # node.find looks at the current page only: every page with layers is visited.
    r, _ = go([step("page.list", {})], cur, os.path.join(tmp, "sc_t.figcraft"))
    pages = [pg["id"] for pg in r[0]["result"] if pg.get("children")]
    r, _ = go([x for pg in pages for x in (step("page.select", {"id": pg}), step("node.find", {"name": "Handoff", "type": "component_set", "limit": 50}))], cur, os.path.join(tmp, "sc_t.figcraft"))
    sets = [i for x in r if x.get("command") == "node.find" and x.get("ok") for i in x["result"]["ids"]]
    return {label: {"runs": runs_out, "approval_sets_in_the_file": len(sets), "failures": sum(len(x["failed"]) for x in runs_out)}}


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
        # Components one after the other in the same file, with the language changed between runs.
        if not update or not only or only in "2º componente":
            if private:
                if _SECOND and os.path.exists(os.path.join(PRIVATE, _SECOND["file"])):
                    results.update(second_component_case(tmp, _SECOND["name"], os.path.join(PRIVATE, _SECOND["file"]), [(i, lang) for i, lang in _SECOND["picks"]]))
            else:
                results.update(second_component_case(tmp, "2º componente / ds-base-demo", DEMO, [("Card", "en"), ("Button", "pt-BR"), ("Accordion", "pt-BR")]))
    secs = time.time() - started
    if dump:
        json.dump(results, open(dump, "w"), indent=2, ensure_ascii=False)
        print(f"results written: {dump} ({len(results)} cases, {secs:.1f} s)")
        return 0
    if update:
        if any("error" in r for r in results.values()):
            sys.exit("not writing the baseline: " + "; ".join(f"{k}: {v['error']}" for k, v in results.items() if "error" in v))
        merged = json.load(open(baseline)) if only and os.path.exists(baseline) else {}
        merged.update({k: {f: v for f, v in r.items() if f not in ("export", "invariants")} for k, r in results.items()})
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
        if name.startswith(("cover editada", "apoio compartilhado", "master /", "2º componente")):
            if got != want:
                diffs.append(f"{name}: baseline {want!r}, now {got!r}")
            continue
        for v in got.get("invariants") or []:
            diffs.append(f"{name}: INVARIANT: {v}")
        mk = got.get("markers") or {}
        if mk and (mk["oblique"] or mk["tip_in_text"] or mk["cross"] or mk["min_gap"] < 12):
            diffs.append(f"{name}: INVARIANT: markers: oblique {mk['oblique']}, tip_in_text {mk['tip_in_text']}, cross {mk['cross']}, min_gap {mk['min_gap']} (>= 12)")
        sp = got.get("spacing") or {}
        if sp and (sp["badge_over_content"] or sp["guide_over_content"] or sp["cross"] or sp["oblique"] or sp["stack_overlap"] or sp["spine_cross"] or sp["min_gap"] < 12):
            diffs.append(f"{name}: INVARIANT: layouts: badge_over_content {sp['badge_over_content']}, guide_over_content {sp['guide_over_content']}, cross {sp['cross']}, oblique {sp['oblique']}, stack_overlap {sp['stack_overlap']}, spine_cross {sp['spine_cross']}, min_gap {sp['min_gap']} (>= 12)")
        if "frames" not in want:
            diffs.append(f"{name}: the baseline entry is in the old format: regenerate it with --update")
            continue
        for key in ("frames", "section", "tables", "warnings", "failed", "markers", "spacing"):
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
