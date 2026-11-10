#!/usr/bin/env python3
"""Builds "DS Base Demo" (a neutral design system for Handoff Components) from nothing, headless.

Clean room: nothing here comes from another client's file or from handoff-demo.figcraft. Tokens follow
shadcn/ui (MIT). Usage: python3 build_ds.py [out.figcraft]
"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcpc import FC

OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "ds-base-demo.figcraft")
fc = FC()
c = fc.cmd

# ------------------------------------------------------------------ tokens
COLORS = {  # name: (light, dark)
    "background": ("#FFFFFF", "#090B0C"),
    "foreground": ("#090B0C", "#F9FBFB"),
    "card": ("#FFFFFF", "#161B1D"),
    "primary": ("#C6005C", "#A3004C"),
    "primary-foreground": ("#FDF2F8", "#FDF2F8"),
    "secondary": ("#F4F4F5", "#27272A"),
    "secondary-foreground": ("#18181B", "#FAFAFA"),
    "muted": ("#F1F3F3", "#22292B"),
    "muted-foreground": ("#67787C", "#9CA8AB"),
    "accent": ("#F1F3F3", "#22292B"),
    "accent-foreground": ("#161B1D", "#F9FBFB"),
    "destructive": ("#E7000B", "#FF6467"),
    "border": ("#E3E7E8", "#FFFFFF1A"),
    "input": ("#E3E7E8", "#FFFFFF26"),
    "ring": ("#9CA8AB", "#67787C"),
}
SPACING = [4, 8, 12, 16, 24]
RADIUS = 10

# ---- OKLCH helpers (hover and pressed are the enabled colour moved in L, keeping hue and chroma)
import math


def _lin(v):
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def _gam(v):
    return 12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055


def hex_to_oklch(h):
    r, g, b = (_lin(int(h[i:i + 2], 16) / 255) for i in (1, 3, 5))
    l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
    m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
    s_ = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
    l, m, s_ = l ** (1 / 3), m ** (1 / 3), s_ ** (1 / 3)
    L = 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s_
    a = 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s_
    bb = 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s_
    return L, math.hypot(a, bb), math.degrees(math.atan2(bb, a)) % 360


def oklch_to_rgb(L, C, H):
    a, b = C * math.cos(math.radians(H)), C * math.sin(math.radians(H))
    l, m, s_ = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3, (L - 0.1055613458 * a - 0.0638541728 * b) ** 3, (L - 0.0894841775 * a - 1.2914855480 * b) ** 3
    return (4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s_, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s_, -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s_)


def oklch_to_hex(L, C, H):
    while True:  # keep it in gamut by lowering the chroma
        rgb = oklch_to_rgb(L, C, H)
        if all(-1e-4 <= v <= 1 + 1e-4 for v in rgb) or C < 1e-4:
            break
        C *= 0.97
    return "#" + "".join(f"{round(min(1, max(0, _gam(max(0, v))) ) * 255):02X}" for v in rgb)


def luminance(h):  # WCAG relative luminance
    r, g, b = (_lin(int(h[i:i + 2], 16) / 255) for i in (1, 3, 5))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def shift(hexc, delta, mode):
    """Move the colour by `delta` in OKLCH L: darker, except a dark colour in Dark mode, which goes lighter (it stays visible against the page)."""
    L, C, H = hex_to_oklch(hexc)
    sign = +1 if (mode == "Dark" and L < 0.45) else -1
    return oklch_to_hex(L + sign * delta, C, H)


STATE_DELTA = {"hover": 0.04, "pressed": 0.08}
DERIVED = {  # token: (base token, state)
    "primary-hover": ("primary", "hover"), "primary-pressed": ("primary", "pressed"),
    "destructive-hover": ("destructive", "hover"), "destructive-pressed": ("destructive", "pressed"),
    "secondary-hover": ("secondary", "hover"), "secondary-pressed": ("secondary", "pressed"),
}
for tok, (base, st) in DERIVED.items():
    COLORS[tok] = tuple(shift(COLORS[base][i], STATE_DELTA[st], mode) for i, mode in enumerate(["Light", "Dark"]))

c("variable.createCollection", name="Colors", modes=["Light", "Dark"])
V = {}
for name, (light, dark) in COLORS.items():
    V["color/" + name] = c("variable.create", collection="Colors", name="color/" + name, type="color", values={"Light": light, "Dark": dark})["id"]
c("variable.createCollection", name="Spacing")
for n in SPACING:
    V[f"space/{n}"] = c("variable.create", collection="Spacing", name=f"space/{n}", type="number", value=n)["id"]
c("variable.createCollection", name="Radius")
V["radius/default"] = c("variable.create", collection="Radius", name="radius/default", type="number", value=RADIUS)["id"]


def bind(node, field, token):
    c("variable.bind", ids=[node], field=field, variable=V[token])


def child_ids(node):
    return [x["id"] for x in c("node.get", id=node, fields=["children"]).get("children", [])]


def descendants(node):
    out = []
    for k in child_ids(node):
        out.append(k)
        out += descendants(k)
    return out


def by_name(node, name):
    for d in descendants(node):
        if c("node.get", id=d, fields=["name"])["name"] == name:
            return d
    raise KeyError(name)


def frame(parent, name, layout="vertical", w=100, h=20, x=0, y=0, **props):
    p = {"layout": layout, "fill": None, "sizingH": "hug", "sizingV": "hug", "crossAlign": "min"}
    p.update(props)
    if p["sizingH"] == "fixed" and "w" not in props:
        pass
    args = {"x": x, "y": y, "width": w, "height": h, "name": name, "select": False, "props": p}
    if parent:
        args["parent"] = parent
    return c("frame.create", **args)["id"]


def text(parent, s, name, size=14, family="Inter", style="Regular", fill="#090B0C", line=None, width=None, spacing=None, **extra):
    props = {"fontSize": size, "fontFamily": family, "fontStyle": style, "fill": fill}
    if line:
        props["lineHeight"] = line
    if spacing:
        props["letterSpacing"] = spacing
    props.update(extra)
    args = {"text": s, "x": 0, "y": 0, "parent": parent, "name": name, "select": False, "props": props}
    if width:
        args["width"] = width
    return c("text.create", **args)["id"]


# ------------------------------------------------------------------ pages
pages = c("page.list")
comp_page = pages[0]["id"]
c("page.rename", id=comp_page, name="Components")
found_page = c("page.add", name="Foundations")["id"]
c("page.select", id=found_page)

# ------------------------------------------------------------------ foundations: text and effect styles (sources are specimens on the page)
SPEC_X = 0
specimen_frame = frame(None, "Type styles", "vertical", x=0, y=700, gap=16, padding=24, fill="#FFFFFF", cornerRadius=10)
TEXT_STYLES = [
    ("text/h3", dict(size=24, style="SemiBold", line="32px", spacing="-1%"), "Heading three"),
    ("text/body-large", dict(size=16, line="24px"), "Body large: the quick brown fox"),
    ("text/body", dict(size=14, line="20px"), "Body: the quick brown fox jumps over the lazy dog"),
    ("text/muted", dict(size=14, line="20px"), "Muted: supporting text"),
    ("text/label-s", dict(size=12, style="Medium", line="16px"), "Label small"),
    ("text/label-m", dict(size=14, style="Medium", line="20px"), "Label medium"),
    ("text/label-l", dict(size=16, style="Medium", line="24px"), "Label large"),
    ("text/code", dict(size=13, family="JetBrains Mono", line="20px"), "npm install demo-ds"),
]
STYLE_SRC = {}
for sname, kw, sample in TEXT_STYLES:
    t = text(specimen_frame, sample, sname, **kw)
    fg = "color/muted-foreground" if sname == "text/muted" else "color/foreground"
    bind(t, "fill", fg)
    c("style.create", type="text", name=sname, **{"from": t})
    STYLE_SRC[sname] = t

# effect styles: shadow/sm, shadow/md
fx = frame(None, "Effect specimens", "horizontal", x=0, y=1100, gap=48, padding=40, fill="#FFFFFF")
for ename, y, blur, alpha in [("shadow/sm", 1, 2, "0D"), ("shadow/md", 4, 8, "26")]:
    b = frame(fx, ename, "horizontal", w=96, h=64, sizingH="fixed", sizingV="fixed", fill="#FFFFFF", cornerRadius=10)
    c("effect.add", ids=[b], effect={"type": "dropShadow", "offset": [0, y], "radius": blur, "color": "#000000" + alpha})
    c("style.create", type="effect", name=ename, **{"from": b})
# a paint style (a plain colour kept as a style, no variable: the code block surface)
pb = frame(fx, "surface/code", "horizontal", w=96, h=64, sizingH="fixed", sizingV="fixed", fill="#F1F3F3", cornerRadius=10)
c("style.create", type="paint", name="surface/code", **{"from": pb})

# ------------------------------------------------------------------ foundations: swatches in Light and Dark
def swatches(label, mode, y):
    outer = frame(None, f"Colors / {label}", "vertical", x=0, y=y, gap=16, padding=24, fill=None, cornerRadius=10)
    bind(outer, "fill", "color/background")
    c("variable.setMode", ids=[outer], collection="Colors", mode=mode)
    text(outer, f"Colors / {label}", "Title", size=16, style="SemiBold", fill="#000000")
    bind(c("node.find", root=outer, type="text", limit=1)["ids"][0], "fill", "color/foreground")
    grid = frame(outer, "Swatches", "horizontal", gap=12, wrap=True, sizingH="fixed", w=900)
    for name in COLORS:
        s = frame(grid, name, "vertical", gap=6, w=130, sizingH="fixed")
        chip = frame(s, "Chip", "horizontal", w=130, h=48, sizingH="fixed", sizingV="fixed", fill="#000000", cornerRadius=10, stroke="#000000", strokeWeight=1, strokeAlign="inside")
        bind(chip, "fill", "color/" + name)
        bind(chip, "stroke", "color/border")
        bind(chip, "cornerRadius", "radius/default")
        t = text(s, name, "Name", size=12, style="Medium", fill="#000000")
        bind(t, "fill", "color/foreground")
    return outer

swatches("Light", "Light", 0)
swatches("Dark", "Dark", 380)

# ------------------------------------------------------------------ components page
c("page.select", id=comp_page)

# ---- Icon: Phosphor Icons (MIT), Regular weight, 256-unit viewBox scaled proportionally into the frame
PHOSPHOR = json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "phosphor_regular.json")))["icons"]
ICON_NAMES = ["caret-down", "caret-up", "caret-right", "plus", "minus", "x", "check", "arrow-right", "info", "warning"]
ICON_SIZES = [16, 20, 24]


def icon_variant(name, size, x, y):
    comp = c("frame.create", x=x, y=y, width=size, height=size, name=f"Name={name}, Size={size}", select=False, props={"fill": None, "clip": False})["id"]
    v = c("node.create", type="vector", parent=comp, path=PHOSPHOR[name], x=0, y=0, name=name, select=False)["id"]
    g = c("node.get", id=v, fields=["x", "y", "width", "height"])  # at the origin the box sits at the path's own offset
    k = size / 256
    c("node.set", ids=[v], props={"width": g["width"] * k, "height": g["height"] * k, "x": g["x"] * k, "y": g["y"] * k})
    c("node.set", ids=[v], props={"fill": "#090B0C"})
    bind(v, "fill", "color/foreground")
    cid = c("component.create", ids=[comp])["id"]
    # the glyph must sit inside its frame
    fb = c("node.get", id=cid, fields=["absoluteBounds"])["absoluteBounds"]
    vb = c("node.get", id=v, fields=["absoluteBounds"])["absoluteBounds"]
    eps = 0.01
    assert vb["x"] >= fb["x"] - eps and vb["y"] >= fb["y"] - eps and vb["x"] + vb["width"] <= fb["x"] + fb["width"] + eps and vb["y"] + vb["height"] <= fb["y"] + fb["height"] + eps, (name, size, fb, vb)
    return cid


ICON = {}
for ni, name in enumerate(ICON_NAMES):
    for si, size in enumerate(ICON_SIZES):
        ICON[(name, size)] = icon_variant(name, size, si * 40, ni * 40)
icon_set = c("component.combineVariants", ids=list(ICON.values()))["id"]
c("node.set", ids=[icon_set], props={"name": "Icon", "x": 0, "y": 0})
c("component.editProperty", id=icon_set, name="Size", default="16")

# ---- Button
TYPES = ["Primary", "Secondary", "Outline", "Destructive"]
SIZES = ["Small", "Medium", "Large"]
STATES = ["Enabled", "Hover", "Pressed", "Disabled", "Focus"]

# token per type: (fill token, text token, hover opacity)
BTN = {
    "Primary": ("color/primary", "color/primary-foreground", 0.9),
    "Secondary": ("color/secondary", "color/secondary-foreground", 0.8),
    "Outline": (None, "color/foreground", None),
    "Destructive": ("color/destructive", "color/background", 0.9),  # light label on red in Light, dark label on the lighter red in Dark
}
SZ = {  # padV, padH, gap, icon, label style, radius token?
    "Small": dict(padV=4, padH=12, gap=6, icon=16, style="text/label-s", bind_gap=False, bind_radius=False, radius=8, bind_padV=False),
    "Medium": dict(padV=8, padH=16, gap=8, icon=16, style="text/label-m", bind_gap=True, bind_radius=True, radius=RADIUS, bind_padV=True),
    "Large": dict(padV=12, padH=24, gap=8, icon=20, style="text/label-l", bind_gap=True, bind_radius=True, radius=RADIUS, bind_padV=True),
}
LABEL_PROPS = {"text/label-s": dict(size=12, style="Medium", line="16px"), "text/label-m": dict(size=14, style="Medium", line="20px"), "text/label-l": dict(size=16, style="Medium", line="24px")}


def set_paint_opacity(node, token, opacity, key="fills"):
    # opacity of the first paint, keeping the variable binding
    cur = c("node.get", id=node, fields=[key])[key][0]
    cur = dict(cur); cur["opacity"] = opacity
    c("node.set", ids=[node], props={key: [cur]})


def build_button(typ, size, state, x, y):
    s = SZ[size]
    fill_tok, text_tok, hov = BTN[typ]
    name = f"Type={typ}, Size={size}, State={state}"
    props = dict(gap=s["gap"], padding=[s["padV"], s["padH"], s["padV"], s["padH"]], mainAlign="center", crossAlign="center", cornerRadius=s["radius"])
    props["fill"] = "#C6005C" if fill_tok else None
    comp = frame(None, name, "horizontal", x=x, y=y, w=120, h=36, **props)
    # layers: Leading icon, Label, Trailing icon (hidden)
    left = c("instance.create", component=ICON[("plus", s["icon"])], parent=comp)["id"]
    c("node.set", ids=[left], props={"name": "Leading icon"})
    label = frame(comp, "Label", "horizontal", crossAlign="center")
    fg = "#FFFFFF" if text_tok is None else "#090B0C"
    t = text(label, "Button", "Text", fill=fg, **LABEL_PROPS[s["style"]])
    right = c("instance.create", component=ICON[("arrow-right", s["icon"])], parent=comp)["id"]
    c("node.set", ids=[right], props={"name": "Trailing icon", "visible": False})
    # text style, then colour
    if size != "Small":  # Small keeps raw type values: a mix of style and loose values
        c("style.apply", ids=[t], style=s["style"])
    if text_tok:
        bind(t, "fill", text_tok)
    # icons follow the text colour (an override on the glyph)
    for ic, gname in ((left, "plus"), (right, "arrow-right")):
        mark = by_name(ic, gname)
        if text_tok:
            bind(mark, "fill", text_tok)
        else:
            c("node.set", ids=[mark], props={"fill": "#FFFFFF"})
    # fill
    if fill_tok:
        bind(comp, "fill", fill_tok)
    if s["bind_radius"]:
        bind(comp, "cornerRadius", "radius/default")
    bind(comp, "paddingHorizontal", f"space/{s['padH']}")
    if s["bind_padV"]:
        bind(comp, "paddingVertical", f"space/{s['padV']}")
    if s["bind_gap"]:
        bind(comp, "gap", f"space/{s['gap']}")
    # outline: a border
    if typ == "Outline":
        c("node.set", ids=[comp], props={"stroke": "#E3E7E8", "strokeWeight": 1, "strokeAlign": "inside"})
        bind(comp, "stroke", "color/border")
    # states: hover and pressed have colours of their own (no opacity); Disabled keeps 50% opacity (the alpha case)
    if state == "Hover":
        hov = {"Primary": "color/primary-hover", "Secondary": "color/secondary-hover", "Outline": "color/accent", "Destructive": "color/destructive-hover"}[typ]
        c("node.set", ids=[comp], props={"fill": "#F1F3F3"}); bind(comp, "fill", hov)
        if typ == "Outline":
            bind(t, "fill", "color/accent-foreground")
            for ic, gname in ((left, "plus"), (right, "arrow-right")):
                bind(by_name(ic, gname), "fill", "color/accent-foreground")
        c("style.apply", ids=[comp], style="shadow/sm")
    elif state == "Pressed":
        prs = {"Primary": "color/primary-pressed", "Secondary": "color/secondary-pressed", "Outline": "color/secondary-pressed", "Destructive": "color/destructive-pressed"}[typ]
        c("node.set", ids=[comp], props={"fill": "#F1F3F3"}); bind(comp, "fill", prs)
    elif state == "Disabled":
        c("node.set", ids=[comp], props={"opacity": 0.5})
    elif state == "Focus":
        if typ == "Outline":
            c("node.set", ids=[comp], props={"strokeWeight": 2})
        else:
            c("node.set", ids=[comp], props={"stroke": "#9CA8AB", "strokeWeight": 2, "strokeAlign": "outside"})
        bind(comp, "stroke", "color/ring")
    return c("component.create", ids=[comp])["id"]


BUTTON = {}
y = 0
for ti, typ in enumerate(TYPES):
    for si, size in enumerate(SIZES):
        for sti, state in enumerate(STATES):
            x = 420 + sti * 220
            yy = 400 + (ti * 3 + si) * 80
            BUTTON[(typ, size, state)] = build_button(typ, size, state, x, yy)
btn_set = c("component.combineVariants", ids=list(BUTTON.values()))["id"]
c("node.set", ids=[btn_set], props={"name": "Button"})

c("component.editProperty", id=btn_set, name="Size", default="Medium")  # Primary / Medium / Enabled is the default variant

# boolean properties for the two icons (default: left shown, right hidden)
for pname, lname in [("Leading icon", "Leading icon"), ("Trailing icon", "Trailing icon")]:
    c("component.addProperty", id=btn_set, name=pname, type="boolean", default=(pname == "Leading icon"))
for key, comp in BUTTON.items():
    for pname in ("Leading icon", "Trailing icon"):
        c("component.bindProperty", layer=by_name(comp, pname), property=pname, field="visible")
# the icons are swappable instances (default: plus on the left, arrow-right on the right)
SWAP = {}
for pname, default in [("Leading glyph", ICON[("plus", 16)]), ("Trailing glyph", ICON[("arrow-right", 16)])]:
    SWAP[pname] = c("component.addProperty", id=btn_set, name=pname, type="instanceSwap", default=default, preferred=[icon_set])
for key, comp in BUTTON.items():
    for pname, layer in [("Leading glyph", "Leading icon"), ("Trailing glyph", "Trailing icon")]:
        c("component.bindProperty", layer=by_name(comp, layer), property=pname, field="mainComponent")

# ------------------------------------------------------------------ Card
BTN_SEC = BUTTON[("Secondary", "Medium", "Enabled")]
BTN_PRI = BUTTON[("Primary", "Medium", "Enabled")]
LONG = ("Design systems earn their keep when the people who build with them can see exactly what a component is made of: "
        "which tokens it reads, how much room it keeps around its content, and what changes between one variant and the next. "
        "This card is deliberately wordy so that long text, wrapping and a fixed width all show up in the report.")
CARD_SIZES = {"Compact": (320, False, 16), "Default": (384, True, 24), "Wide": (448, True, 24)}


def build_card(size, x, y):
    w, features, pad = CARD_SIZES[size]
    comp = frame(None, f"Size={size}", "vertical", x=x, y=y, w=w, sizingH="fixed", gap=16, padding=pad, fill="#FFFFFF", cornerRadius=RADIUS,
                 stroke="#E3E7E8", strokeWeight=1, strokeAlign="inside")
    bind(comp, "fill", "color/card"); bind(comp, "stroke", "color/border"); bind(comp, "cornerRadius", "radius/default")
    bind(comp, "gap", "space/16")
    bind(comp, "paddingHorizontal", f"space/{pad}"); bind(comp, "paddingVertical", f"space/{pad}")
    c("style.apply", ids=[comp], style="shadow/sm")
    header = frame(comp, "Header", "vertical", gap=4, sizingH="fill")
    title_row = frame(header, "Title row", "horizontal", sizingH="fill", mainAlign="spaceBetween", crossAlign="center")
    title = text(title_row, "Project overview", "Title", size=24, style="SemiBold", line="32px", spacing="-1%")
    info = c("instance.create", component=ICON[("info", 20)], parent=title_row)["id"]
    c("node.set", ids=[info], props={"name": "Info icon"})
    bind(by_name(info, "info"), "fill", "color/muted-foreground")
    desc = text(header, "A short line that says what this card is for.", "Description", size=14, line="20px", width=w - 2 * pad)
    if size != "Compact":
        c("style.apply", ids=[title], style="text/h3")
    c("style.apply", ids=[desc], style="text/muted")
    bind(title, "fill", "color/foreground"); bind(desc, "fill", "color/muted-foreground")
    content = frame(comp, "Content", "vertical", gap=12, sizingH="fill")
    c("node.set", ids=[content], props={"gap": 12}); bind(content, "gap", "space/12")
    para = text(content, LONG if size != "Compact" else LONG[:140] + "…", "Paragraph", size=14, line="20px", width=w - 2 * pad)
    c("style.apply", ids=[para], style="text/body"); bind(para, "fill", "color/foreground")
    feats = frame(content, "Features", "vertical", gap=8, sizingH="fill")
    for i in range(3):  # three identical rows: the x3 case
        row = frame(feats, "Feature", "horizontal", gap=8, crossAlign="center")
        bind(row, "gap", "space/8")
        ic = c("instance.create", component=ICON[("check", 16)], parent=row)["id"]
        bind(by_name(ic, "check"), "fill", "color/foreground")
        ft = text(row, "Reads every colour from a token", "Text", size=14, line="20px")
        c("style.apply", ids=[ft], style="text/body"); bind(ft, "fill", "color/foreground")
    if not features:
        c("node.set", ids=[feats], props={"visible": False})  # a hidden layer
    bar = frame(comp, "Footer actions", "horizontal", gap=8, sizingH="fill", mainAlign="max", crossAlign="center")
    bind(bar, "gap", "space/8")
    c("instance.create", component=BTN_SEC, parent=bar)
    c("instance.create", component=BTN_PRI, parent=bar)
    return c("component.create", ids=[comp])["id"]


cards = [build_card(s, 420 + i * 520, 2200) for i, s in enumerate(CARD_SIZES)]
card_set = c("component.combineVariants", ids=cards)["id"]
c("node.set", ids=[card_set], props={"name": "Card"})
c("component.editProperty", id=card_set, name="Size", default="Default")

# ------------------------------------------------------------------ Accordion
ACC_BODY = "A short answer that opens under its question. It wraps to a second line so the expanded state is clearly taller."


def build_accordion(state, x, y):
    comp = frame(None, f"State={state}", "vertical", x=x, y=y, w=384, sizingH="fixed", gap=0, fill=None, cornerRadius=RADIUS,
                 stroke="#E3E7E8", strokeWeight=1, strokeAlign="inside", clip=True)
    bind(comp, "stroke", "color/border"); bind(comp, "cornerRadius", "radius/default")
    trig = frame(comp, "Trigger", "horizontal", sizingH="fill", padding=[16, 16, 16, 16], mainAlign="spaceBetween", crossAlign="center")
    bind(trig, "paddingHorizontal", "space/16"); bind(trig, "paddingVertical", "space/16")
    q = text(trig, "Is it built with tokens?", "Question", size=14, style="Medium", line="20px")
    c("style.apply", ids=[q], style="text/label-m"); bind(q, "fill", "color/foreground")
    gname = "caret-up" if state == "Expanded" else "caret-down"  # no rotation: the glyph itself changes
    chev = c("instance.create", component=ICON[(gname, 16)], parent=trig)["id"]
    c("node.set", ids=[chev], props={"name": "Chevron"})
    bind(by_name(chev, gname), "fill", "color/muted-foreground")
    body = frame(comp, "Answer", "vertical", sizingH="fill", padding=[0, 16, 16, 16])
    bind(body, "paddingHorizontal", "space/16")
    a = text(body, ACC_BODY, "Text", size=14, line="20px", width=384 - 32)
    c("style.apply", ids=[a], style="text/muted"); bind(a, "fill", "color/muted-foreground")
    if state == "Collapsed":
        c("node.set", ids=[body], props={"visible": False})
    if state == "Locked":
        c("node.set", ids=[comp], props={"opacity": 0.5}); c("node.set", ids=[body], props={"visible": False})
    return c("component.create", ids=[comp])["id"]


acc = [build_accordion(s, 420 + i * 440, 3100) for i, s in enumerate(["Collapsed", "Expanded", "Locked"])]
acc_set = c("component.combineVariants", ids=acc)["id"]
c("node.set", ids=[acc_set], props={"name": "Accordion"})
c("component.editProperty", id=acc_set, name="State", default="Expanded")

# ------------------------------------------------------------------ Code (a single component, no variants)
code = frame(None, "Code", "vertical", x=420, y=3500, w=420, sizingH="fixed", gap=8, padding=[12, 16, 12, 16], fill="#F1F3F3", cornerRadius=RADIUS)
c("style.apply", ids=[code], style="surface/code", target="fill"); bind(code, "fill", "color/muted"); bind(code, "cornerRadius", "radius/default"); bind(code, "paddingHorizontal", "space/16")
for line in ["$ npm install demo-ds", "$ npx demo-ds init --tokens", "# 15 colour tokens, 5 spacings, 1 radius"]:
    t = text(code, line, "Line", size=13, family="JetBrains Mono", line="20px")
    c("style.apply", ids=[t], style="text/code"); bind(t, "fill", "color/foreground")
code_comp = c("component.create", ids=[code])["id"]

# tidy: stack the sets in a column with a fixed gap
yy = 0
for node in [icon_set, btn_set, card_set, acc_set, code_comp]:
    c("node.set", ids=[node], props={"x": 0, "y": yy})
    b = c("node.get", id=node, fields=["absoluteBounds"])["absoluteBounds"]
    yy += b["height"] + 120

# ------------------------------------------------------------------ Dark artboards (instances only: every colour comes from the mode)
DARK_X = 1800


def dark_board(name, y, parts, width=1000):
    board = frame(None, f"{name} / Dark", "horizontal", x=DARK_X, y=y, w=width, sizingH="fixed", gap=16, padding=24, fill="#FFFFFF", cornerRadius=RADIUS, wrap=True)
    bind(board, "fill", "color/background"); bind(board, "gap", "space/16"); bind(board, "cornerRadius", "radius/default")
    bind(board, "paddingHorizontal", "space/24"); bind(board, "paddingVertical", "space/24")
    c("variable.setMode", ids=[board], collection="Colors", mode="Dark")
    for comp in parts:
        c("instance.create", component=comp, parent=board)
    return board


def set_y(node):
    return c("node.get", id=node, fields=["absoluteBounds"])["absoluteBounds"]["y"]


DARK = {}
DARK["Button"] = dark_board("Button", set_y(btn_set), [BUTTON[("Primary", "Medium", st)] for st in ("Enabled", "Hover", "Pressed", "Disabled", "Focus")]
                            + [BUTTON[(t, "Medium", "Enabled")] for t in ("Secondary", "Outline", "Destructive")])
DARK["Card"] = dark_board("Card", set_y(card_set), [cards[1]], width=460)
DARK["Accordion"] = dark_board("Accordion", set_y(acc_set), [acc[1], acc[0], acc[2]], width=1240)  # Expanded, Collapsed, Locked
DARK["Code"] = dark_board("Code", set_y(code_comp), [code_comp], width=480)
DARK["Icon"] = dark_board("Icon", set_y(icon_set), [ICON[(n, 24)] for n in ("caret-down", "caret-up", "plus", "x", "check", "info", "warning", "arrow-right")], width=420)

# ------------------------------------------------------------------ dark preview on Foundations
c("page.select", id=found_page)
prev = frame(None, "Dark preview", "vertical", x=1100, y=0, gap=24, padding=24, fill="#090B0C", cornerRadius=RADIUS)
bind(prev, "fill", "color/background")
c("variable.setMode", ids=[prev], collection="Colors", mode="Dark")
c("instance.create", component=card_set, parent=prev)
c("instance.create", component=BUTTON[("Primary", "Medium", "Hover")], parent=prev)
c("page.select", id=comp_page)

res = fc.tool("save_file", {"path": OUT})
print("saved", res)
