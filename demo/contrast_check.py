#!/usr/bin/env python3
"""WCAG contrast of every text and icon layer against the fill behind it, in Light and Dark.

Usage: python3 contrast_check.py file.figcraft [out.json]
Reads the file in a headless session (nothing is saved). Colours come from the variables (resolved in each
mode) or, for a loose value, from the layer itself. The background is the nearest ancestor with a solid fill,
else Colors/background of the mode. Layers under a node with opacity below 1 (Disabled, Locked) are exempt.
Text must reach 4.5:1, icons 3:1.
"""
import json, os, sys, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcpc import FC


def parse_hex(h):
    h = h.lstrip("#")
    r, g, b = (int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    a = int(h[6:8], 16) / 255 if len(h) == 8 else 1.0
    return (r, g, b, a)


def lin(v):
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def lum(c):
    return 0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])


def over(top, bottom):
    a = top[3]
    return tuple(top[i] * a + bottom[i] * (1 - a) for i in range(3)) + (1.0,)


def to_hex(c):
    return "#%02X%02X%02X" % tuple(round(v * 255) for v in c[:3])


def ratio(a, b):
    la, lb = lum(a), lum(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def audit(path):
    fc = FC()
    c = fc.cmd
    fc.tool("open_file", {"path": path})
    vl = c("variable.list")
    colors = [x for x in vl["collections"] if x["name"] == "Colors"][0]
    mode_id = {m["name"]: m["id"] for m in colors["modes"]}
    var = {}
    for v in vl["variables"]:
        if v["collection"] != colors["id"]:
            continue
        var[v["id"]] = {"name": v["name"], "by_mode": {}}
        for mn, mid in mode_id.items():
            val = v["values"][mid]["value"]
            var[v["id"]]["by_mode"][mn] = (val["r"], val["g"], val["b"], val["a"])
    bg_var = [i for i, v in var.items() if v["name"] == "color/background"][0]
    rows = collections.OrderedDict()

    def fill_of(node, mode):
        """(rgba, description) of the node's first drawn solid fill, or None."""
        b = (node.get("boundVariables") or {}).get("fill")
        paints = [p for p in (node.get("fills") or []) if p.get("visible", True) and p.get("type") == "solid"]
        if not paints:
            return None
        p = paints[0]
        op = p.get("opacity", 1.0)
        if b in var:
            col = var[b]["by_mode"][mode]
            desc = var[b]["name"]
        else:
            col = parse_hex(p["color"])
            desc = p["color"].upper() + (" (loose)" if not (node.get("styles") or {}).get("fill") else " (paint style)")
        return (col[:3] + (col[3] * op,), desc)

    def walk(i, comp, variant, path, bg, mode, exempt, out):
        n = c("node.get", id=i, fields=["name", "type", "children", "fills", "boundVariables", "opacity", "visible", "styles", "characters"])
        if n.get("visible") is False:
            return
        ex = exempt or (n.get("opacity") not in (None, 1, 1.0))
        f = fill_of(n, mode)
        kind = n["type"]
        here = path + [n["name"]]
        if kind in ("TEXT", "VECTOR"):
            if f:
                limit = 4.5 if kind == "TEXT" else 3.0
                fg = over(f[0], bg[0])
                r = ratio(fg, bg[0])
                key = (comp, " / ".join(here[1:]), f[1], bg[1], mode, "text" if kind == "TEXT" else "icon")
                row = rows.setdefault(key, {"ratio": r, "limit": limit, "fg": to_hex(fg), "bg": to_hex(bg[0]), "variants": [], "exempt": ex})
                row["variants"].append(variant)
                row["exempt"] = row["exempt"] and ex
            return
        nb = bg
        if f and kind in ("FRAME", "COMPONENT", "INSTANCE"):
            nb = (over(f[0], bg[0]), f[1])
        for ch in n.get("children", []) or []:
            walk(ch["id"], comp, variant, here, nb, mode, ex, out)

    top = []
    for p in c("page.list"):
        for s in c("node.find", root=p["id"], type="component_set", limit=100)["ids"]:
            sn = c("node.get", id=s, fields=["name", "children"])
            for v in sn["children"]:
                top.append((sn["name"], v["id"], c("node.get", id=v["id"], fields=["name"])["name"]))
        for s in c("node.find", root=p["id"], type="component", limit=1000)["ids"]:
            nm = c("node.get", id=s, fields=["name"])["name"]
            if "=" not in nm:
                top.append((nm, s, nm))
    boards = []
    for p in c("page.list"):
        for ch in c("node.get", id=p["id"], fields=["children"])["children"]:
            nm = c("node.get", id=ch["id"], fields=["name", "type"])
            if nm["type"] == "FRAME" and "Dark" in nm["name"]:
                boards.append((nm["name"], ch["id"], nm["name"]))
    for mode in mode_id:
        bgc = var[bg_var]["by_mode"][mode]
        for comp, vid, vname in top:
            walk(vid, comp, vname, [], (bgc, "color/background"), mode, False, None)
    dark_bg = var[bg_var]["by_mode"]["Dark"]
    for comp, vid, vname in boards:  # artboards set to Dark: their own fill is the background
        walk(vid, comp, vname, [], (dark_bg, "color/background"), "Dark", False, None)
    fc.p.terminate()
    out = []
    for (comp, layer, fg, bg, mode, kind), r in rows.items():
        out.append({"component": comp, "layer": layer, "kind": kind, "fg": fg, "bg": bg, "mode": mode, "fg_hex": r["fg"], "bg_hex": r["bg"],
                    "ratio": round(r["ratio"], 2), "limit": r["limit"], "exempt": r["exempt"], "n": len(r["variants"]), "variants": r["variants"]})
    return out


if __name__ == "__main__":
    res = audit(sys.argv[1])
    if len(sys.argv) > 2:
        json.dump(res, open(sys.argv[2], "w"), indent=1)
    fails = [r for r in res if r["ratio"] < r["limit"] and not r["exempt"]]
    print(len(res), "pairs;", len(fails), "below the limit (Disabled/Locked exempt)")
    for r in fails:
        print(f'  {r["component"]} / {r["layer"]} [{r["mode"]}] {r["fg"]} on {r["bg"]}: {r["ratio"]} < {r["limit"]} ({r["n"]} variants)')
