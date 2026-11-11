//! The "gerar" command: a report frame for one component, to the right of everything on the page:
//! a header, the anatomy (the instance on a stage with its padding measures and numbered markers,
//! and a table of the numbered properties) and a token audit. Every text it shows comes from
//! `crate::i18n`.

use crate::clock;
use crate::audit::{self, Group, Info, Line, Prop, Row, State, Styles, Vars};
use crate::glyphs;
use crate::i18n::{Key, Lang, override_name, plural, t, tf};
use crate::contrast::{self, Rgba};
use crate::delta;
use crate::store::{Meta, REPORT_KEY, Rerun, Settings, StageMode};
use crate::theme;
use crate::variants::{self, Pairs, SetInfo, Variant};
use figcraft_plugin_sdk::{Input, Result, Value, call, json, notify};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Host calls made through `host` so far (the legend's cost is measured with it).
static HOST_CALLS: AtomicU64 = AtomicU64::new(0);

/// Most top-level layers looked at when finding the free spot.
const MAX_TOP_LEVEL: usize = 2_000;
/// Most layers audited, and most legend lines drawn.
const MAX_AUDIT_LAYERS: usize = 500;
const MAX_AUDIT_LINES: usize = 250;
/// Gap between the existing content and the report.
const GAP_RIGHT: f64 = 512.0;
/// Where the stage is built before it moves into the report (below anything the report covers).
const STAGING_Y: f64 = 4_000.0;
/// The report's margin, its least width and the space between its sections.
const FRAME_PAD: f64 = 64.0;
const MIN_FRAME_W: f64 = 1280.0;
const SECTION_GAP: f64 = 56.0;
/// The legend's width, the space between it and the stage, and the stage's room around the
/// instance (in all, per axis) and least height.
const LEGEND_W: f64 = 520.0;
const ANATOMY_GAP: f64 = 32.0;
const STAGE_ROOM: f64 = 192.0;
const MIN_STAGE_H: f64 = 240.0;
const STAGE_RADIUS: f64 = 12.0;
const FONT: f64 = theme::MIN_SIZE;
/// Markers: diameter, least distance from the component to the marker's edge, least distance
/// between two markers' centres, and the margin kept to the stage's edge.
const MARKER_D: f64 = 28.0;
const MARKER_CLEAR: f64 = 24.0;
const MARKER_GAP: f64 = 8.0;
/// Least distance between two markers' centres: the circles, the least gap and the ring.
const MARKER_STEP: f64 = MARKER_D + MARKER_GAP + 2.0;
const STAGE_EDGE: f64 = 32.0;
/// Rough width of a character at 14 px, to centre texts.
const CHAR_W: f64 = 8.5;
const LABEL_H: f64 = 18.0;
/// Padding of the patch behind a value of the overlay.
const LABEL_PAD_X: f64 = 4.0;
const LABEL_PAD_Y: f64 = 1.0;
/// The longest line of a token inside a chip, in characters.
const CHIP_CHARS: usize = 36;
/// The legend: indent of a layer's lines under its glyph, space between blocks, sizes of a block's
/// header and lines, most badges on the anatomy stage and the tallest legend the stage follows.
const LEGEND_INDENT: f64 = 38.0;
const BLOCK_GAP: f64 = 36.0;
const HEADER_SIZE: f64 = 18.0;
const LINE_SIZE: f64 = 15.0;
const MAX_BADGES: usize = 12;
/// The Properties section's name and type columns, and the most variant rows the axes draw.
const PROP_NAME_W: f64 = 180.0;
const PROP_TYPE_W: f64 = 120.0;
const MAX_AXIS_ROWS: usize = 24;
/// A report taller than this exports at 1x: at 2x it would pass 16,384 px, where images break.
const EXPORT_2X_MAX_H: f64 = 8_000.0;
/// Axis rows: room around the instance (in all) and least stage height.
const AXIS_ROOM: f64 = 64.0;
const AXIS_MIN_H: f64 = 120.0;
const STAGE_MAX_LEGEND_H: f64 = 480.0;
/// How far below the anatomy stage the spacing stage is built.
const SPACING_STAGING_DY: f64 = 3_000.0;

/// What `node.get` is asked for when auditing a layer.
const AUDIT_FIELDS: [&str; 25] = ["name", "type", "characters", "mainComponent", "mainComponentName", "visible", "opacity", "sizing", "boundVariables", "cornerRadius", "layout", "fills", "strokes", "children", "absoluteBounds", "styles", "textStyleRanges", "fontFamily", "fontSize", "fontStyle", "lineHeight", "letterSpacing", "textDecoration", "effects", "strokeWeight"];

/// A box on the page.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn right(self) -> f64 {
        self.x + self.w
    }

    fn bottom(self) -> f64 {
        self.y + self.h
    }

    fn grow(self, by: f64) -> Rect {
        Rect { x: self.x - by, y: self.y - by, w: self.w + 2.0 * by, h: self.h + 2.0 * by }
    }

    fn hits(self, o: Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}

fn num(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|f| f.is_finite())
}

fn bounds(node: &Value) -> Option<Rect> {
    let b = node.get("absoluteBounds")?;
    Some(Rect { x: num(b, "x")?, y: num(b, "y")?, w: num(b, "width")?, h: num(b, "height")? })
}

/// A host call whose error comes back with the command's name, in the report's language.
pub(crate) fn host(lang: Lang, cmd: &str, params: &Value) -> Result<Value> {
    HOST_CALLS.fetch_add(1, Ordering::Relaxed);
    call(cmd, params).map_err(|e| tf(lang, Key::ErrCall, &[("cmd", cmd), ("reason", &e)]))
}

fn id_of(lang: Lang, v: &Value, what: &str) -> Result<u64> {
    v.get("id").and_then(Value::as_u64).ok_or_else(|| tf(lang, Key::ErrNoId, &[("what", what)]))
}

/// The line colour, or the default.
fn line_colour(text: Option<&str>) -> String {
    match text.map(str::trim) {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => theme::REDLINE.to_string(),
    }
}

/// The fill colour: the one given, or the line colour at 30% (0x4D).
pub(crate) fn fill_colour(text: Option<&str>, line: &str) -> String {
    if let Some(t) = text.map(str::trim).filter(|t| !t.is_empty()) {
        return t.to_string();
    }
    // `#RRGGBB` (or `#RRGGBBAA`: its alpha is replaced) at 30% (0x4D).
    match line.get(..7) {
        Some(rgb) if line.starts_with('#') && line.len() >= 7 => format!("{rgb}4D"),
        _ => match line.strip_prefix('#').filter(|h| h.len() == 3) {
            Some(h) => h.chars().flat_map(|c| [c, c]).fold(String::from("#"), |mut s, c| {
                s.push(c);
                s
            }) + "4D",
            None => theme::REDLINE_FILL.to_string(),
        },
    }
}

/// Make a text layer inside `parent` (page position given when the parent has no auto layout).
#[allow(clippy::too_many_arguments)]
fn text(lang: Lang, parent: u64, s: &str, x: f64, y: f64, size: f64, fill: &str, width: Option<f64>, name: &str) -> Result<u64> {
    let mut p = json!({"text": s, "x": x, "y": y, "parent": parent, "name": name, "select": false,
        "props": {"fontSize": size.max(theme::MIN_SIZE), "fill": fill, "fontFamily": theme::FONT_FAMILY}});
    if let (Some(w), Some(o)) = (width, p.as_object_mut()) {
        o.insert("width".into(), json!(w));
    }
    let r = host(lang, "text.create", &p)?;
    id_of(lang, &r, "text.create")
}

/// A frame with auto layout inside `parent`.
struct FlowSpec<'a> {
    name: &'a str,
    width: f64,
    /// `None`: it hugs its height.
    height: Option<f64>,
    direction: &'a str,
    gap: f64,
    padding: Value,
    fill: Option<&'a str>,
    /// More properties, merged over the usual ones.
    extra: Value,
}

impl<'a> FlowSpec<'a> {
    fn new(name: &'a str, width: f64, direction: &'a str) -> FlowSpec<'a> {
        FlowSpec { name, width, height: None, direction, gap: 0.0, padding: json!(0), fill: None, extra: json!({}) }
    }
}

fn flow_frame(lang: Lang, parent: u64, spec: FlowSpec) -> Result<u64> {
    let mut props = json!({"fill": spec.fill, "layout": spec.direction, "gap": spec.gap, "padding": spec.padding, "crossAlign": "min",
        "sizingH": "fixed", "sizingV": if spec.height.is_some() { "fixed" } else { "hug" }, "clip": false});
    if let (Some(p), Some(e)) = (props.as_object_mut(), spec.extra.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    let r = host(
        lang,
        "frame.create",
        &json!({"parent": parent, "width": spec.width, "height": spec.height.unwrap_or(20.0), "name": spec.name, "select": false, "props": props}),
    )?;
    id_of(lang, &r, "frame.create")
}

fn prop_key(p: Prop) -> Key {
    match p {
        Prop::Radius => Key::PropRadius,
        Prop::Gap => Key::PropGap,
        Prop::PadH => Key::PropPadH,
        Prop::PadV => Key::PropPadV,
        Prop::Fill => Key::PropFill,
        Prop::Stroke => Key::PropStroke,
        Prop::Typography => Key::PropTypography,
    }
}

/// Break `s` into lines of at most `max_chars`, only at spaces and after a slash: a token comes
/// apart at its "/" and at its "→", never in the middle of a word.
fn wrap(s: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut line = 0usize;
    for (w, word) in s.split(' ').enumerate() {
        let pieces: Vec<&str> = word.split_inclusive('/').collect();
        for (p, piece) in pieces.iter().enumerate() {
            let len = piece.chars().count();
            // A space before the first piece of a word; none inside a word.
            let space = usize::from(p == 0 && w > 0);
            if line > 0 && line + space + len > max_chars {
                out.push('\n');
                line = 0;
            } else if space == 1 && line > 0 {
                out.push(' ');
                line += 1;
            }
            out.push_str(piece);
            line += len;
        }
    }
    out
}

/// The layers of the instance, for working out where a guide can go.
struct Tree<'a> {
    nodes: &'a [Value],
    depths: Vec<usize>,
    parent: Vec<Option<usize>>,
}

impl<'a> Tree<'a> {
    fn new(nodes: &'a [Value], depths: Vec<usize>) -> Tree<'a> {
        let by_id: HashMap<u64, usize> = nodes.iter().enumerate().filter_map(|(i, n)| Some((n.get("id")?.as_u64()?, i))).collect();
        let mut parent = vec![None; nodes.len()];
        for (i, n) in nodes.iter().enumerate() {
            for c in n.get("children").and_then(Value::as_array).into_iter().flatten() {
                if let Some(&ci) = c.get("id").and_then(Value::as_u64).and_then(|id| by_id.get(&id))
                    && let Some(p) = parent.get_mut(ci)
                {
                    *p = Some(i);
                }
            }
        }
        Tree { nodes, depths, parent }
    }

    fn bounds(&self, i: usize) -> Option<Rect> {
        bounds(self.nodes.get(i)?)
    }

    /// The boxes a guide to layer `owner` must not cross: every layer inside the component except
    /// `owner` and what it sits in.
    fn obstacles(&self, owner: usize) -> Vec<Rect> {
        let mut skip = vec![owner];
        let mut at = owner;
        while let Some(Some(p)) = self.parent.get(at) {
            skip.push(*p);
            at = *p;
        }
        (0..self.nodes.len()).filter(|i| self.depths.get(*i).is_some_and(|d| *d >= 1) && !skip.contains(i)).filter_map(|i| self.bounds(i)).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// Whether the straight guide from `a` to `b` crosses `r` (shrunk a hair, so touching is fine).
fn crosses(a: (f64, f64), b: (f64, f64), r: Rect) -> bool {
    let r = r.grow(-0.5);
    if r.w <= 0.0 || r.h <= 0.0 {
        return false;
    }
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, d, lo, hi) in [(a.0, b.0 - a.0, r.x, r.right()), (a.1, b.1 - a.1, r.y, r.bottom())] {
        if d.abs() < 1e-9 {
            if p < lo || p > hi {
                return false;
            }
        } else {
            let (mut ta, mut tb) = ((lo - p) / d, (hi - p) / d);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

/// Where a marker's guide goes: the side the marker sits on and the point the guide ends at.
#[derive(Clone, Copy, Debug)]
struct Target {
    side: Side,
    end: (f64, f64),
}

/// Where the guide of the badge of layer `owner` goes: the side it comes from and the point on the
/// layer's border. The root is reached from the left; a layer inside it from the side it is
/// nearest to, the first whose way crosses no other layer.
fn layer_target(owner: usize, tree: &Tree, root: Rect, used: &[(f64, f64)], skip: &[Side]) -> Option<Target> {
    let b = tree.bounds(owner)?;
    let t = |side, end| Target { side, end };
    let mut cands: Vec<(f64, Target)> = if owner == 0 {
        vec![
            (0.0, t(Side::Left, (b.x, b.y + b.h / 2.0))),
            (1.0, t(Side::Top, (b.x + b.w / 2.0, b.y))),
            (2.0, t(Side::Bottom, (b.x + b.w / 2.0, b.bottom()))),
            (3.0, t(Side::Right, (b.right(), b.y + b.h / 2.0))),
        ]
    } else {
        vec![
            (b.y - root.y, t(Side::Top, (b.x + b.w / 2.0, b.y))),
            (root.bottom() - b.bottom(), t(Side::Bottom, (b.x + b.w / 2.0, b.bottom()))),
            (b.x - root.x, t(Side::Left, (b.x, b.y + b.h / 2.0))),
            (root.right() - b.right(), t(Side::Right, (b.right(), b.y + b.h / 2.0))),
        ]
    };
    // A way that would end where another layer's guide already ends (a text inside its frame, say)
    // comes last: coincident layers get guides on different sides.
    // A way whose guide would pass through another guide's end comes last as well, and so does a side
    // already tried and given up on.
    for c in &mut cands {
        let start = side_start(c.1.side, c.1.end, root);
        if used.iter().any(|u| (u.0 - c.1.end.0).hypot(u.1 - c.1.end.1) < MARKER_GAP || point_to_segment(*u, start, c.1.end) < 2.0) {
            c.0 += 10_000.0;
        }
        if skip.contains(&c.1.side) {
            c.0 += 100_000.0;
        }
    }
    cands.sort_by(|p, q| p.0.total_cmp(&q.0));
    let obstacles = tree.obstacles(owner);
    cands
        .iter()
        .map(|c| c.1)
        .find(|c| {
            let start = side_start(c.side, c.end, root);
            !obstacles.iter().any(|o| crosses(start, c.end, *o))
        })
        .or_else(|| cands.first().map(|c| c.1))
}

/// Where a guide from `side` starts: on the component's border, in line with its end.
fn side_start(side: Side, end: (f64, f64), root: Rect) -> (f64, f64) {
    match side {
        Side::Top => (end.0, root.y),
        Side::Bottom => (end.0, root.bottom()),
        Side::Left => (root.x, end.1),
        Side::Right => (root.right(), end.1),
    }
}

/// Whether the guide ends on the component's own border (so its marker may move along the side).
fn on_border(end: (f64, f64), root: Rect) -> bool {
    let near = |a: f64, b: f64| (a - b).abs() < 0.75;
    let inside_x = end.0 >= root.x - 0.75 && end.0 <= root.right() + 0.75;
    let inside_y = end.1 >= root.y - 0.75 && end.1 <= root.bottom() + 0.75;
    (inside_x && (near(end.1, root.y) || near(end.1, root.bottom()))) || (inside_y && (near(end.0, root.x) || near(end.0, root.right())))
}

/// A marker to draw: its number, the centre of its circle and the point its guide ends at.
struct Marker {
    /// The place of its item in the list given to `place_markers`.
    item: usize,
    number: usize,
    centre: (f64, f64),
    end: (f64, f64),
}

/// Place the markers around `ib` inside `stage` (all in page coordinates): each on its target's
/// side, `MARKER_CLEAR` away from the component, in line with its guide's end when it can be (the
/// guide then runs straight), `MARKER_STEP` from the others and clear of `avoid` (the values).
/// A guide that doesn't end on the component's border is kept in line first and the others give way.
fn place_markers(ib: Rect, stage: Rect, items: &[(usize, Target)], avoid: &[Rect]) -> Vec<Marker> {
    let r = MARKER_D / 2.0;
    let off = MARKER_CLEAR + r;
    let mut out: Vec<Marker> = Vec::new();
    for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
        let horizontal = matches!(side, Side::Top | Side::Bottom);
        let along = |p: (f64, f64)| if horizontal { p.0 } else { p.1 };
        let (lo, hi) = if horizontal { (stage.x + STAGE_EDGE + r, stage.right() - STAGE_EDGE - r) } else { (stage.y + STAGE_EDGE + r, stage.bottom() - STAGE_EDGE - r) };
        let centre_at = |a: f64| match side {
            Side::Top => (a, ib.y - off),
            Side::Bottom => (a, ib.bottom() + off),
            Side::Left => (ib.x - off, a),
            Side::Right => (ib.right() + off, a),
        };
        let mut group: Vec<(usize, &(usize, Target))> = items.iter().enumerate().filter(|(_, (_, t))| t.side == side).collect();
        // Guides that end inside the component first, then by position.
        group.sort_by(|a, b| {
            let pin = |t: &Target| !on_border(t.end, ib);
            pin(&b.1 .1).cmp(&pin(&a.1 .1)).then(along(a.1 .1.end).total_cmp(&along(b.1 .1.end)))
        });
        for (item, (number, target)) in group {
            let wanted = along(target.end);
            let mut chosen = None;
            for k in 0..14 {
                let offset = MARKER_STEP * ((k + 1) / 2) as f64 * if k % 2 == 1 { 1.0 } else { -1.0 };
                let a = (wanted + offset).clamp(lo, hi);
                let c = centre_at(a);
                let circle = Rect { x: c.0 - r, y: c.1 - r, w: MARKER_D, h: MARKER_D };
                // Clear of every marker already placed, on any side (not only this one's).
                if out.iter().all(|m| (m.centre.0 - c.0).hypot(m.centre.1 - c.1) >= MARKER_STEP - 0.01) && !avoid.iter().any(|v| circle.grow(2.0).hits(*v)) {
                    chosen = Some(a);
                    break;
                }
            }
            // No room that is clear: the marker is left out rather than crowded in.
            if let Some(a) = chosen {
                out.push(Marker { item, number: *number, centre: centre_at(a), end: target.end });
            }
        }
    }
    out.sort_by_key(|m| m.number);
    out
}

/// Whether the segments `a`-`b` and `c`-`d` cross (touching at a shared end doesn't count).
fn segments_cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2, d3, d4) = (side(c, d, a), side(c, d, b), side(a, b, c), side(a, b, d));
    let eps = 1e-6;
    ((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps)) && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps))
}

/// The distance from `p` to the segment `a`-`b`.
fn point_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 < 1e-12 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0) };
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}

/// A marker's guide: from its circle's edge to the end.
fn guide_of(m: &Marker) -> ((f64, f64), (f64, f64)) {
    let (dx, dy) = (m.end.0 - m.centre.0, m.end.1 - m.centre.1);
    let d = dx.hypot(dy).max(0.001);
    ((m.centre.0 + dx / d * MARKER_D / 2.0, m.centre.1 + dy / d * MARKER_D / 2.0), m.end)
}

/// Keep the markers whose guide crosses no kept guide and passes through no kept circle (and whose
/// circle no kept guide passes through, and no guide ends on another); the rest are left out and returned.
fn drop_tangled(markers: Vec<Marker>) -> (Vec<Marker>, Vec<Marker>) {
    let mut kept: Vec<Marker> = Vec::new();
    let mut dropped = Vec::new();
    for m in markers {
        let (a, b) = guide_of(&m);
        let tangled = kept.iter().any(|k| {
            let (c, d) = guide_of(k);
            segments_cross(a, b, c, d)
                || point_to_segment(k.centre, a, b) < MARKER_D / 2.0 + MARKER_GAP / 2.0
                || point_to_segment(m.centre, c, d) < MARKER_D / 2.0 + MARKER_GAP / 2.0
                || point_to_segment(k.end, a, b) < 2.0
                || point_to_segment(m.end, c, d) < 2.0
        });
        if tangled {
            dropped.push(m);
        } else {
            kept.push(m);
        }
    }
    (kept, dropped)
}

/// A line from `a` to `b` as a `shape.line` (rotated about its middle; positive turns counter-clockwise).
fn leader(lang: Lang, parent: u64, origin: (f64, f64), a: (f64, f64), b: (f64, f64), stroke: (&str, f64), name: &str) -> Result<Option<u64>> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx.hypot(dy);
    if len < 0.5 {
        return Ok(None);
    }
    let mut angle = (-dy).atan2(dx).to_degrees();
    if angle > 90.0 {
        angle -= 180.0;
    } else if angle <= -90.0 {
        angle += 180.0;
    }
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let r = host(
        lang,
        "shape.line",
        &json!({"x": mid.0 - len / 2.0 - origin.0, "y": mid.1 - origin.1, "width": len, "parent": parent, "name": name, "select": false,
            "props": {"rotation": angle, "stroke": stroke.0, "strokeWeight": stroke.1}}),
    )?;
    Ok(Some(id_of(lang, &r, "shape.line")?))
}

/// The glyph of a property.
fn prop_glyph(p: Prop) -> &'static str {
    match p {
        Prop::Radius => glyphs::RADIUS,
        Prop::Gap => glyphs::GAP,
        Prop::PadH | Prop::PadV => glyphs::PADDING,
        Prop::Fill => glyphs::FILL,
        Prop::Stroke => glyphs::STROKE,
        Prop::Typography => glyphs::TEXT,
    }
}

/// Whether a property belongs to the Spacing section (padding and gap).
fn is_spacing(p: Prop) -> bool {
    matches!(p, Prop::PadH | Prop::PadV | Prop::Gap)
}

/// The title of a layer: its glyph and name, with "↳" in front when it sits inside another.
fn layer_title(node: &Value, depth: usize, variant: bool, name: &str) -> String {
    let base = match glyphs::layer(node, depth == 0, variant) {
        Some(g) => glyphs::with_word(g, name),
        None => name.to_string(),
    };
    if depth > 0 { glyphs::with_word(glyphs::NESTED, &base) } else { base }
}

/// A state's text, and whether its glyph is in the accent colour.
fn state_text(lang: Lang, state: State) -> (String, bool) {
    match state {
        State::Token => (glyphs::with_word(glyphs::OK, t(lang, Key::LegendToken)), false),
        State::Style => (glyphs::with_word(glyphs::OK, t(lang, Key::LegendStyle)), false),
        State::Check => (glyphs::with_word(glyphs::OK, t(lang, Key::LegendCheck)), false),
        State::Suspect => (glyphs::with_word(glyphs::SUSPECT, t(lang, Key::LegendSuspect)), true),
        State::Missing => (glyphs::with_word(glyphs::MISSING, t(lang, Key::LegendMissing)), true),
        State::Unbound => (t(lang, Key::LegendNone).to_string(), false),
    }
}

/// A text layer with more properties (bold, say) than `text` gives.
#[allow(clippy::too_many_arguments)]
fn text_with(lang: Lang, parent: u64, s: &str, size: f64, fill: &str, name: &str, extra: Value) -> Result<u64> {
    let mut props = json!({"fontSize": size.max(theme::MIN_SIZE), "fill": fill, "fontFamily": theme::FONT_FAMILY});
    if let (Some(p), Some(e)) = (props.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    let r = host(lang, "text.create", &json!({"text": s, "x": 0, "y": 0, "parent": parent, "name": name, "select": false, "props": props}))?;
    id_of(lang, &r, "text.create")
}

/// One property of a layer, in the legend's body. With a token: two lines glued together, "glyph
/// Label : [chip]" and under it, dimmer, "value · state" (and "· ×N layers" when it is shared).
/// Without: one line, "glyph Label : value · no token".
fn draw_prop(lang: Lang, body: u64, width: f64, line: &Line, accent: &str) -> Result<()> {
    let label = t(lang, prop_key(line.property));
    let head = glyphs::with_word(prop_glyph(line.property), label);
    let (state, accented) = state_text(lang, line.state);
    if line.state == State::Unbound {
        let state = if line.property == Prop::Typography { t(lang, Key::LegendNoStyle).to_string() } else { state };
        text_with(lang, body, &format!("{head} : {} · {state}", line.value), LINE_SIZE, theme::TEXT, label, json!({}))?;
        return Ok(());
    }
    let mut pair = FlowSpec::new(label, width, "vertical");
    pair.gap = 2.0;
    let pair = flow_frame(lang, body, pair)?;
    let mut a = FlowSpec::new(label, 20.0, "horizontal");
    a.gap = 6.0;
    a.extra = json!({"sizingH": "hug", "crossAlign": "center"});
    let a = flow_frame(lang, pair, a)?;
    text_with(lang, a, &format!("{head} :"), LINE_SIZE, theme::TEXT, label, json!({}))?;
    let chip_name = t(lang, Key::LayerChip);
    let mut c = FlowSpec::new(chip_name, 20.0, "horizontal");
    c.padding = json!([2, 8, 2, 8]);
    c.fill = Some(theme::PANEL);
    c.extra = json!({"sizingH": "hug", "cornerRadius": 12, "stroke": theme::BORDER, "strokeWeight": 1, "strokeAlign": "inside"});
    let chip = flow_frame(lang, a, c)?;
    text_with(lang, chip, &wrap(&line.token, CHIP_CHARS), LINE_SIZE, theme::TEXT, chip_name, json!({}))?;
    let b = format!("{} · {state}", line.value);
    let id = text_with(lang, pair, &b, LINE_SIZE, theme::MUTED_70, label, json!({}))?;
    if accented {
        // Only the glyph is in the accent colour.
        let start = line.value.chars().count() + 3;
        host(lang, "text.style", &json!({"id": id, "start": start, "end": start + 1, "textFills": [{"type": "solid", "color": accent}]}))?;
    }
    Ok(())
}

/// What to draw for a block of the legend: its first layer's place in the walk, every layer it stands
/// for, its lines, the line under its title and, when it has one, the number of its badge.
struct Block<'a> {
    node: usize,
    nodes: Vec<usize>,
    lines: Vec<&'a Line>,
    sub: Option<String>,
    badge: Option<usize>,
}

/// The blocks of the anatomy: every part, and every other layer group that has a line to show
/// (`keep` picks the lines: the anatomy's or the spacing's).
fn blocks<'a>(groups: &'a [Group], keep: impl Fn(&Line) -> bool, parts: bool) -> Vec<Block<'a>> {
    groups
        .iter()
        .filter_map(|g| {
            let lines: Vec<&Line> = g.lines.iter().filter(|l| keep(l)).collect();
            let is_part = parts && g.part;
            (!lines.is_empty() || is_part).then(|| Block { node: g.node, nodes: g.nodes.clone(), lines, sub: if parts { g.sub.clone() } else { None }, badge: None })
        })
        .collect()
}

/// For each layer of the walk: whether it is drawn (it and everything it sits in are visible).
fn rendered(walked: &[(Value, usize)]) -> Vec<bool> {
    let mut stack: Vec<(usize, bool)> = Vec::new();
    walked
        .iter()
        .map(|(node, depth)| {
            while stack.last().is_some_and(|(d, _)| d >= depth) {
                stack.pop();
            }
            let shown = stack.last().is_none_or(|(_, v)| *v) && node.get("visible").and_then(Value::as_bool).unwrap_or(true);
            stack.push((*depth, shown));
            shown
        })
        .collect()
}

/// Number the blocks of the anatomy that are drawn, in the order of the legend (the tree's order),
/// at most `MAX_BADGES`. Returns how many layers are left without a number.
fn number_blocks(blocks: &mut [Block], rendered: &[bool]) -> usize {
    let mut next = 1;
    let mut left = 0;
    for b in blocks.iter_mut() {
        if !rendered.get(b.node).copied().unwrap_or(false) {
            continue;
        }
        if next <= MAX_BADGES {
            b.badge = Some(next);
            next += 1;
        } else {
            left += b.nodes.iter().filter(|n| rendered.get(**n).copied().unwrap_or(false)).count();
        }
    }
    left
}

/// A text layer's content when it differs from the layer's name, in quotes and cut at 24 characters.
fn text_content(node: &Value, name: &str) -> Option<String> {
    if node.get("type").and_then(Value::as_str) != Some("TEXT") {
        return None;
    }
    let chars = node.get("characters").and_then(Value::as_str)?;
    let flat = chars.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() || flat == name {
        return None;
    }
    let cut = if flat.chars().count() > 24 { format!("{}…", flat.chars().take(24).collect::<String>()) } else { flat };
    Some(format!("\"{cut}\""))
}

/// The content to show for a group of text layers: the same in all of them, else none.
fn group_content(walked: &[(Value, usize)], nodes: &[usize], name: &str) -> Option<String> {
    let mut all = nodes.iter().map(|i| walked.get(*i).and_then(|(n, _)| text_content(n, name)));
    let first = all.next()??;
    all.all(|c| c.as_deref() == Some(first.as_str())).then_some(first)
}

/// The name a block's header shows: the layer's name, "×N" when the block stands for N layers, and
/// the content of its text layers.
fn header_name(name: &str, layers: usize, content: Option<&str>) -> String {
    let mut out = name.to_string();
    if layers > 1 {
        out = format!("{out} ×{layers}");
    }
    if let Some(c) = content {
        out = format!("{out} {c}");
    }
    out
}

/// A layer type in words: "Frame", "Text", "Group"… A type the dictionary doesn't know is the host's
/// name in title case.
fn kind_word(lang: Lang, kind: &str) -> String {
    kind_key(kind).map_or_else(|| title_case(kind), |key| t(lang, key).to_string())
}

/// The dictionary key of a host layer type, when it has one.
fn kind_key(kind: &str) -> Option<Key> {
    let key = match kind {
        "TEXT" => Key::TypeText,
        "FRAME" => Key::KindFrame,
        "GROUP" => Key::KindGroup,
        "RECTANGLE" => Key::KindRectangle,
        "ELLIPSE" => Key::KindEllipse,
        "VECTOR" => Key::KindVector,
        "LINE" => Key::KindLine,
        "STAR" => Key::KindStar,
        "POLYGON" => Key::KindPolygon,
        "BOOLEAN_OPERATION" => Key::KindBooleanOperation,
        "COMPONENT" => Key::KindComponent,
        "COMPONENT_SET" => Key::KindComponentSet,
        "INSTANCE" => Key::KindInstance,
        "SECTION" => Key::KindSection,
        "SLICE" => Key::KindSlice,
        _ => return None,
    };
    Some(key)
}

/// The layer type inside the "not a component" error. English keeps the host's own name (`FRAME`,
/// as it always has); another language words the types the dictionary knows and keeps the host's
/// name for the rest.
fn kind_in_error(lang: Lang, kind: &str) -> String {
    match (lang, kind_key(kind)) {
        (Lang::En, _) | (_, None) => kind.to_string(),
        (_, Some(key)) => t(lang, key).to_string(),
    }
}

/// `BOOLEAN_OPERATION` as "Boolean Operation".
fn title_case(kind: &str) -> String {
    kind.split('_').map(|w| w.chars().next().map(|c| c.to_uppercase().collect::<String>() + &w[c.len_utf8()..].to_lowercase()).unwrap_or_default()).collect::<Vec<_>>().join(" ")
}

/// The component sets of the variants in `wanted`, by variant id (one pass over the file's sets).
fn set_names(lang: Lang, wanted: &[u64]) -> Result<HashMap<u64, String>> {
    let mut out = HashMap::new();
    if wanted.is_empty() {
        return Ok(out);
    }
    let sets = host(lang, "node.find", &json!({"type": "component_set", "limit": 1000}))?;
    for set in sets.get("ids").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_u64) {
        let node = host(lang, "node.get", &json!({"id": set, "fields": ["name", "children"]}))?;
        let name = node.get("name").and_then(Value::as_str).unwrap_or_default();
        for c in node.get("children").and_then(Value::as_array).into_iter().flatten() {
            if let Some(id) = c.get("id").and_then(Value::as_u64)
                && wanted.contains(&id)
            {
                out.insert(id, name.to_string());
            }
        }
        if out.len() == wanted.len() {
            break;
        }
    }
    Ok(out)
}

/// The root's only drawn direct child when it has the root's box (a wrapper): the legend sees through
/// it, so the parts are the wrapper's children.
fn anatomy_wrapper(walked: &[(Value, usize)], drawn: &[bool]) -> Option<usize> {
    let root = bounds(&walked.first()?.0)?;
    let mut kids = walked.iter().enumerate().filter(|(i, (_, d))| *d == 1 && drawn.get(*i).copied().unwrap_or(false));
    let (i, (n, _)) = kids.next()?;
    let same = bounds(n).is_some_and(|b| (b.x - root.x).abs() < 0.5 && (b.y - root.y).abs() < 0.5 && (b.w - root.w).abs() < 0.5 && (b.h - root.h).abs() < 0.5);
    (kids.next().is_none() && same).then_some(i)
}

/// Which layers of the walk are parts: the root and its drawn direct children (the wrapper's, when
/// there is a wrapper).
fn part_flags(walked: &[(Value, usize)], drawn: &[bool]) -> Vec<bool> {
    let wrapper = anatomy_wrapper(walked, drawn).is_some();
    let level = if wrapper { 2 } else { 1 };
    walked.iter().enumerate().map(|(i, (_, d))| drawn.get(i).copied().unwrap_or(false) && (*d == 0 || *d == level)).collect()
}

/// The line under a part's title: "Instance of <component>" for an instance (`set` is its component
/// set's name, when the main component is a variant) or the layer's type. The root has none: it would
/// repeat the report's title.
fn sub_line(lang: Lang, node: &Value, depth: usize, set: Option<String>) -> Option<String> {
    if depth == 0 {
        return None;
    }
    let kind = node.get("type").and_then(Value::as_str).unwrap_or_default();
    if kind != "INSTANCE" {
        return Some(kind_word(lang, kind));
    }
    let main = set.or_else(|| node.get("mainComponentName").and_then(Value::as_str).map(ToString::to_string)).unwrap_or_default();
    Some(if main.is_empty() { kind_word(lang, "INSTANCE") } else { tf(lang, Key::InstanceOf, &[("name", &main)]) })
}

/// What the legend knows of each layer of the walk: name, type and, for a part (the root and its
/// visible direct children), the line under its title: "Instance of <component>" or its type.
fn layer_infos(lang: Lang, walked: &[(Value, usize)], drawn: &[bool]) -> Result<Vec<Info>> {
    let flags = part_flags(walked, drawn);
    let is_part = |i: usize, _d: usize| flags.get(i).copied().unwrap_or(false);
    let variant_mains = |n: &Value| n.get("mainComponentName").and_then(Value::as_str).is_some_and(|m| m.contains('='));
    let wanted: Vec<u64> = walked
        .iter()
        .enumerate()
        .filter(|(i, (n, d))| is_part(*i, *d) && *d > 0 && n.get("type").and_then(Value::as_str) == Some("INSTANCE") && variant_mains(n))
        .filter_map(|(_, (n, _))| n.get("mainComponent").and_then(Value::as_u64))
        .collect();
    let sets = set_names(lang, &wanted)?;
    Ok(walked
        .iter()
        .enumerate()
        .map(|(i, (n, d))| {
            let str_of = |k: &str| n.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
            let kind = str_of("type");
            let part = is_part(i, *d);
            let set = n.get("mainComponent").and_then(Value::as_u64).and_then(|id| sets.get(&id).cloned());
            let sub = if part { sub_line(lang, n, *d, set) } else { None };
            Info { name: str_of("name"), kind, sub, part }
        })
        .collect())
}

/// The walk's index of the direct child that stands in for a root with no spacing of its own: drawn,
/// with spacing rows and the same box as the root.
fn spacing_wrapper(walked: &[(Value, usize)], drawn: &[bool], rows: &[Row]) -> Option<usize> {
    let root = bounds(&walked.first()?.0)?;
    walked.iter().enumerate().find_map(|(i, (n, d))| {
        let same = bounds(n).is_some_and(|b| (b.x - root.x).abs() < 0.5 && (b.y - root.y).abs() < 0.5 && (b.w - root.w).abs() < 0.5 && (b.h - root.h).abs() < 0.5);
        (*d == 1 && drawn.get(i).copied().unwrap_or(false) && same && rows.iter().any(|r| r.node == i && is_spacing(r.property))).then_some(i)
    })
}

/// The groups with at most `max` lines in all, and how many lines are left out. The cap is spent on
/// the lines of the first kind (fill, stroke, radius…) in tree order; a Typography line rides along
/// with its layer's other lines without spending it, and a layer that has only a Typography line gets
/// what the cap has left over once every layer with another line is covered.
fn limit_groups(groups: &[Group], max: usize) -> (Vec<Group>, usize) {
    let typo = |l: &Line| l.property == Prop::Typography;
    let mut left = max;
    let mut kept: Vec<usize> = Vec::with_capacity(groups.len());
    for g in groups {
        let others = g.lines.iter().filter(|l| !typo(l)).count();
        let take = others.min(left);
        left -= take;
        kept.push(take);
    }
    let mut hidden = 0;
    let shown = groups
        .iter()
        .zip(&kept)
        .map(|(g, take)| {
            let others_shown = *take;
            let has_others = g.lines.iter().any(|l| !typo(l));
            let mut lines: Vec<Line> = Vec::new();
            let mut seen = 0;
            for l in &g.lines {
                if typo(l) {
                    // With its layer's other lines; alone, only from what is left over.
                    if others_shown > 0 || (!has_others && left > 0) {
                        lines.push(l.clone());
                        if !has_others {
                            left -= 1;
                        }
                    }
                } else if seen < others_shown {
                    seen += 1;
                    lines.push(l.clone());
                }
            }
            hidden += g.lines.len() - lines.len();
            Group { lines, ..g.clone() }
        })
        .collect();
    (shown, hidden)
}

/// A block of the legend: the layer's header (a badge when it has one, "↳" when it sits inside
/// another layer) and under it its lines, indented under the glyph. `extra` are plain lines
/// ("Label : value") that follow.
#[allow(clippy::too_many_arguments)]
fn block_name(walked: &[(Value, usize)], block: &Block) -> String {
    let layer_name = walked.get(block.node).and_then(|(v, _)| v.get("name")).and_then(Value::as_str).unwrap_or_default();
    let content = group_content(walked, &block.nodes, layer_name);
    header_name(layer_name, block.nodes.len(), content.as_deref())
}

fn draw_block(lang: Lang, legend: u64, walked: &[(Value, usize)], variant: bool, block: &Block, extra: &[(String, String)], accent: &str) -> Result<()> {
    let (node, depth) = walked.get(block.node).map(|(v, d)| (v.clone(), *d)).unwrap_or((Value::Null, 0));
    let layer_name = node.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let name = block_name(walked, block);
    let title = layer_title(&node, depth, variant, &name);
    let gname = tf(lang, Key::LayerGroup, &[("name", &layer_name)]);
    let mut g = FlowSpec::new(&gname, LEGEND_W, "vertical");
    g.gap = 12.0;
    let group = flow_frame(lang, legend, g)?;
    let bold = json!({"fontStyle": "Bold"});
    match block.badge {
        Some(n) => {
            let mut h = FlowSpec::new(t(lang, Key::LayerBlockHeader), LEGEND_W, "horizontal");
            h.gap = 10.0;
            h.extra = json!({"crossAlign": "center"});
            let header = flow_frame(lang, group, h)?;
            let badge_name = tf(lang, Key::LayerBadge, &[("n", &n.to_string())]);
            let mut b = FlowSpec::new(&badge_name, MARKER_D, "horizontal");
            b.height = Some(MARKER_D);
            b.fill = Some(accent);
            b.extra = json!({"cornerRadius": MARKER_D / 2.0, "mainAlign": "center", "crossAlign": "center"});
            let badge = flow_frame(lang, header, b)?;
            text_with(lang, badge, &n.to_string(), FONT, theme::ON_REDLINE, &badge_name, json!({}))?;
            // The badge already says where it sits: the title drops the "↳".
            let plain = match glyphs::layer(&node, depth == 0, variant) {
                Some(g) => glyphs::with_word(g, &name),
                None => name.clone(),
            };
            text_with(lang, header, &plain, HEADER_SIZE, theme::TEXT, &gname, bold)?;
        }
        None => {
            text_with(lang, group, &title, HEADER_SIZE, theme::TEXT, &gname, bold)?;
        }
    }
    let width = LEGEND_W - LEGEND_INDENT;
    let mut b = FlowSpec::new(t(lang, Key::LayerBody), LEGEND_W, "vertical");
    b.gap = 10.0;
    b.padding = json!([0, 0, 0, LEGEND_INDENT]);
    let body = flow_frame(lang, group, b)?;
    if let Some(sub) = &block.sub {
        text_with(lang, body, sub, LINE_SIZE, theme::MUTED_70, t(lang, Key::LayerKind), json!({}))?;
    }
    for line in &block.lines {
        draw_prop(lang, body, width, line, accent)?;
    }
    for (label, value) in extra {
        text_with(lang, body, &format!("{label} : {value}"), LINE_SIZE, theme::TEXT, label, json!({}))?;
    }
    Ok(())
}

/// A word of an alignment, from the host's `min`, `center`, `max`, `spaceBetween`… along the vertical
/// or the horizontal axis.
fn align_word(lang: Lang, value: &str, vertical: bool) -> String {
    let key = match (value, vertical) {
        ("min", true) => Key::AlignTop,
        ("center", true) => Key::AlignMiddle,
        ("max", true) => Key::AlignBottom,
        ("min", false) => Key::AlignLeft,
        ("center", false) => Key::AlignCenter,
        ("max", false) => Key::AlignRight,
        ("spaceBetween", _) => Key::AlignSpaceBetween,
        ("spaceAround", _) => Key::AlignSpaceAround,
        ("spaceEvenly", _) => Key::AlignSpaceEvenly,
        _ => return value.to_string(),
    };
    t(lang, key).to_string()
}

/// "Middle center": the vertical part first, then the horizontal one. A horizontal layout's main axis
/// is the horizontal one, a vertical layout's the vertical one. The nine combinations of top, middle
/// and bottom with left, center and right each have a text of their own (the order of words and the
/// small words between them differ from language to language); with a distribution ("space between")
/// or a value the dictionary doesn't know, the two words are joined by the `AlignCompose` text.
fn alignment_text(lang: Lang, mode: &str, main: &str, cross: &str) -> String {
    let (v, h) = if mode == "vertical" { (main, cross) } else { (cross, main) };
    let key = match (v, h) {
        ("min", "min") => Some(Key::AlignTopLeft),
        ("min", "center") => Some(Key::AlignTopCenter),
        ("min", "max") => Some(Key::AlignTopRight),
        ("center", "min") => Some(Key::AlignMiddleLeft),
        ("center", "center") => Some(Key::AlignMiddleCenter),
        ("center", "max") => Some(Key::AlignMiddleRight),
        ("max", "min") => Some(Key::AlignBottomLeft),
        ("max", "center") => Some(Key::AlignBottomCenter),
        ("max", "max") => Some(Key::AlignBottomRight),
        _ => None,
    };
    match key {
        Some(key) => t(lang, key).to_string(),
        None => tf(lang, Key::AlignCompose, &[("vertical", &align_word(lang, v, true)), ("horizontal", &align_word(lang, h, false).to_lowercase())]),
    }
}

/// A resizing value in words: Hug, Fixed or Fill.
fn resize_word(lang: Lang, value: &str) -> String {
    match value {
        "hug" => t(lang, Key::ResizeHug).to_string(),
        "fixed" => t(lang, Key::ResizeFixed).to_string(),
        "fill" => t(lang, Key::ResizeFill).to_string(),
        other => other.to_string(),
    }
}

/// The auto layout lines of a layer, as the host gives them: direction, alignment and resizing.
fn layout_lines(lang: Lang, node: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(layout) = node.get("layout") else { return out };
    let mode_text = layout.get("mode").and_then(Value::as_str).unwrap_or_default();
    let mode = match mode_text {
        "horizontal" => Key::DirHorizontal,
        "vertical" => Key::DirVertical,
        "grid" => Key::DirGrid,
        _ => return out,
    };
    out.push((t(lang, Key::SpDirection).to_string(), t(lang, mode).to_string()));
    if let (Some(main), Some(cross)) = (layout.get("mainAlign").and_then(Value::as_str), layout.get("crossAlign").and_then(Value::as_str)) {
        out.push((t(lang, Key::SpAlignment).to_string(), alignment_text(lang, mode_text, main, cross)));
    }
    let sizing = |k: &str| node.get("sizing").and_then(|s| s.get(k)).and_then(Value::as_str);
    if let Some(h) = sizing("horizontal") {
        out.push((t(lang, Key::SpHResizing).to_string(), resize_word(lang, h)));
    }
    if let Some(v) = sizing("vertical") {
        out.push((t(lang, Key::SpVResizing).to_string(), resize_word(lang, v)));
    }
    out
}

/// A value of the padding overlay: its text on a solid patch in the report's background colour,
/// so a guide passing close by doesn't get in the way of reading it. `x`, `y` are the text's own
/// place on the page; `origin` is the stage's.
#[allow(clippy::too_many_arguments)]
fn value_label(lang: Lang, stage: u64, origin: (f64, f64), label: &str, x: f64, y: f64, colour: &str, patch_fill: &str, name: &str) -> Result<u64> {
    let patch = host(
        lang,
        "frame.create",
        &json!({"parent": stage, "x": x - LABEL_PAD_X - origin.0, "y": y - LABEL_PAD_Y - origin.1, "width": 20, "height": LABEL_H, "name": name, "select": false,
            "props": {"fill": patch_fill, "layout": "horizontal", "padding": [LABEL_PAD_Y, LABEL_PAD_X, LABEL_PAD_Y, LABEL_PAD_X], "crossAlign": "center",
                "sizingH": "hug", "sizingV": "hug", "clip": false}}),
    )?;
    let patch = id_of(lang, &patch, "frame.create")?;
    text(lang, patch, label, 0.0, 0.0, FONT, colour, None, name)?;
    Ok(patch)
}

/// The note under the audit that says how many lines were left out, if any.
fn hidden_note(lang: Lang, hidden: usize) -> Option<String> {
    (hidden > 0).then(|| tf(lang, plural(hidden, Key::MoreLinesOne, Key::MoreLines), &[("n", &hidden.to_string())]))
}

/// The layers of the instance as `node.get` gives them for the audit, depth first, each with its
/// depth (at most `MAX_AUDIT_LAYERS`).
fn walk_layers(lang: Lang, root: u64) -> Result<Vec<(Value, usize)>> {
    let mut nodes = Vec::new();
    let mut stack = vec![(root, 0usize)];
    while let Some((id, depth)) = stack.pop() {
        if nodes.len() >= MAX_AUDIT_LAYERS {
            break;
        }
        let node = host(lang, "node.get", &json!({"id": id, "fields": AUDIT_FIELDS}))?;
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            stack.extend(children.iter().rev().filter_map(|c| c.get("id").and_then(Value::as_u64)).map(|c| (c, depth + 1)));
        }
        nodes.push((node, depth));
    }
    Ok(nodes)
}

/// Whether any layer has a style applied (otherwise `style.list` isn't worth asking).
fn any_styled(nodes: &[Value]) -> bool {
    nodes.iter().any(|n| {
        n.get("styles").and_then(Value::as_object).is_some_and(|s| ["fill", "stroke", "text"].iter().any(|k| s.get(*k).is_some_and(|v| !v.is_null())))
            || n.get("textStyleRanges").and_then(Value::as_array).is_some_and(|r| !r.is_empty())
    })
}

/// Whether any layer has a variable bound (otherwise `variable.list` isn't worth asking).
fn any_bound(nodes: &[Value]) -> bool {
    nodes.iter().any(|n| n.get("boundVariables").and_then(Value::as_object).is_some_and(|b| !b.is_empty()))
}

/// What one read of the current page gives: its id, its settings and its top-level layers (each
/// with name, type, box and `pluginData`).
pub struct PageInfo {
    pub id: u64,
    pub settings: Settings,
    pub children: Vec<Value>,
}

/// Read the current page. Its errors are in English: the language isn't known until its settings are read.
pub fn page_info() -> Result<PageInfo> {
    let lang = Lang::En;
    let pages = host(lang, "page.list", &json!({}))?;
    let list = pages.as_array().ok_or_else(|| t(lang, Key::ErrNotList).to_string())?;
    let id = list
        .iter()
        .find(|p| p.get("current").and_then(Value::as_bool).unwrap_or(false))
        .or_else(|| list.first())
        .and_then(|p| p.get("id").and_then(Value::as_u64))
        .ok_or_else(|| t(lang, Key::ErrNoPage).to_string())?;
    // One call: the page's own data, and its children with their names, boxes and data.
    let top = host(lang, "node.get", &json!({"id": id, "fields": ["children", "name", "type", "absoluteBounds", "pluginData"]}))?;
    let settings = Settings::from_plugin_data(top.get("pluginData").unwrap_or(&Value::Null));
    let children = top.get("children").and_then(Value::as_array).map(|a| a.iter().take(MAX_TOP_LEVEL).cloned().collect()).unwrap_or_default();
    Ok(PageInfo { id, settings, children })
}

/// The component a selection stands for, where its sheet is kept, and what to tell about the choice.
pub struct Picked {
    /// The component the report shows (a variant when the selection is one).
    pub component: u64,
    /// The layer that holds the sheet: the component set for a variant (so all variants share it),
    /// else the component.
    pub holder: u64,
    /// The component as `node.get` gives it (name, type, box).
    pub node: Value,
    pub notes: Vec<String>,
    /// The name the report carries: the set's for a set, else the component's.
    pub title: String,
    /// What marks the report for a rerun: the set's id for a set, else the component's.
    pub key: u64,
    /// The set, when a set was selected.
    pub set: Option<SetPick>,
}

/// A selected component set: its variants and which one is the default.
pub struct SetPick {
    pub info: SetInfo,
    /// The pairs of the default variant (complete).
    pub default: Pairs,
    /// Where they come from.
    pub source: DefaultSource,
}

/// Where the default variant of a report comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefaultSource {
    /// The one saved in the settings.
    Saved,
    /// The one the component itself says (`component.properties`).
    Component,
    /// The first variant of the set.
    First,
}

/// The default variant the component says (`component.properties` on the set): the default of each
/// of its variant properties, when they name a variant of the set.
fn component_default(lang: Lang, set: u64, info: &SetInfo) -> Option<Pairs> {
    let d = host(lang, "component.properties", &json!({"id": set})).ok()?;
    let typed: Pairs = d
        .get("properties")
        .and_then(Value::as_array)?
        .iter()
        .filter(|p| p.get("type").and_then(Value::as_str) == Some("variant"))
        .filter_map(|p| Some((clean_name(p.get("name")?.as_str()?).to_string(), p.get("default")?.as_str()?.to_string())))
        .collect();
    if typed.is_empty() { None } else { variants::complete(info, &typed).ok() }
}

/// The variants of a component set, as `node.get` gives its children (name and box).
pub fn variants_of(node: &Value) -> Vec<Variant> {
    node.get("children")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    let b = bounds(c)?;
                    Some(Variant { id: c.get("id")?.as_u64()?, name: c.get("name")?.as_str()?.to_string(), w: b.w, h: b.h })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Load a component set: its variants and what they spell.
pub fn load_set(lang: Lang, id: u64) -> Result<(SetInfo, Meta)> {
    let node = host(lang, "node.get", &json!({"id": id, "fields": ["name", "children", "absoluteBounds", "pluginData"]}))?;
    let name = node.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let meta = Meta::from_plugin_data(node.get("pluginData").unwrap_or(&Value::Null));
    Ok((SetInfo::new(id, name, variants_of(&node)), meta))
}

/// The component set that holds `component`, if one does.
fn set_of(lang: Lang, component: u64) -> Result<Option<u64>> {
    let sets = host(lang, "node.find", &json!({"type": "component_set", "limit": 1000}))?;
    for set in sets.get("ids").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_u64) {
        let node = host(lang, "node.get", &json!({"id": set, "fields": ["children"]}))?;
        if node.get("children").and_then(Value::as_array).is_some_and(|c| c.iter().any(|v| v.get("id").and_then(Value::as_u64) == Some(component))) {
            return Ok(Some(set));
        }
    }
    Ok(None)
}

/// Work out the component of a selection: an instance gives its main component, a component set its
/// first variant (said in a note, as for several layers selected).
pub fn resolve(lang: Lang, selection: &[u64]) -> Result<Picked> {
    let mut notes: Vec<String> = Vec::new();
    let mut id = selection.first().copied().ok_or_else(|| t(lang, Key::ErrNoSelection).to_string())?;
    if selection.len() > 1 {
        notes.push(tf(lang, Key::Variants, &[("n", &selection.len().to_string())]));
    }
    let mut node = host(lang, "node.get", &json!({"id": id, "fields": ["type", "name", "mainComponent", "overrides", "children", "absoluteBounds", "pluginData"]}))?;
    let mut set: Option<u64> = None;
    let mut set_pick: Option<SetPick> = None;
    let mut set_name = String::new();
    if node.get("type").and_then(Value::as_str) == Some("COMPONENT_SET") {
        let name = node.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
        let info = SetInfo::new(id, name.clone(), variants_of(&node));
        if info.variants.is_empty() {
            return Err(tf(lang, Key::ErrEmptySet, &[("id", &id.to_string())]));
        }
        // The saved default, when it still names a variant; else the first one.
        let saved = Meta::from_plugin_data(node.get("pluginData").unwrap_or(&Value::Null)).default;
        let from_saved = saved.as_deref().and_then(|t| variants::parse_default(t).ok()).and_then(|typed| variants::complete(&info, &typed).ok());
        // Saved default, then the component's own, then the first variant.
        let (pairs, source) = match from_saved {
            Some(p) => (p, DefaultSource::Saved),
            None => match component_default(lang, id, &info) {
                Some(p) => (p, DefaultSource::Component),
                None => (info.first_pairs(), DefaultSource::First),
            },
        };
        let chosen = info.find(&pairs).or(info.variants.first()).cloned();
        let chosen = chosen.ok_or_else(|| tf(lang, Key::ErrEmptySet, &[("id", &id.to_string())]))?;
        set = Some(id);
        set_name = name;
        node = json!({"id": chosen.id, "type": "COMPONENT", "name": chosen.name, "absoluteBounds": {"x": 0, "y": 0, "width": chosen.w, "height": chosen.h}});
        id = chosen.id;
        set_pick = Some(SetPick { info, default: pairs, source });
    }
    // An instance: its main component (with a note about overrides); a variant stays itself.
    let kind = node.get("type").and_then(Value::as_str).unwrap_or_default().to_string();
    let component = match kind.as_str() {
        "INSTANCE" => {
            let main = node.get("mainComponent").and_then(Value::as_u64).ok_or_else(|| tf(lang, Key::ErrNoMain, &[("id", &id.to_string())]))?;
            let overrides: Vec<String> = node
                .get("overrides")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).map(|o| override_name(lang, o)).collect())
                .unwrap_or_default();
            if !overrides.is_empty() {
                notes.push(tf(lang, Key::OverrideNote, &[("list", &overrides.join(", "))]));
            }
            main
        }
        "COMPONENT" => id,
        other => return Err(tf(lang, Key::ErrNotComponent, &[("id", &id.to_string()), ("kind", &kind_in_error(lang, other))])),
    };
    // The component's own name and box: the layer itself when it is the component.
    let comp = if component == id { node.clone() } else { host(lang, "node.get", &json!({"id": component, "fields": ["name", "type", "absoluteBounds"]}))? };
    // A variant is named like "Property=Value, …": look for its set only then.
    let variant_like = comp.get("name").and_then(Value::as_str).is_some_and(|n| n.contains('='));
    let holder = match set {
        Some(s) => s,
        None if variant_like => set_of(lang, component)?.unwrap_or(component),
        None => component,
    };
    let title = if set_pick.is_some() { set_name } else { comp.get("name").and_then(Value::as_str).unwrap_or_default().to_string() };
    let key = if set_pick.is_some() { holder } else { component };
    Ok(Picked { component, holder, node: comp, notes, title, key, set: set_pick })
}

/// A stage: a top-level frame, away from everything, that the instance is placed on. (instance.place
/// only puts an instance in a top-level frame, and a frame with auto layout would flow it.) It
/// moves into the report once it is finished.
fn make_stage(lang: Lang, at: Rect, dark: bool) -> Result<u64> {
    // Dark: the navy with a faint border. Light: Alabaster, no border.
    let props = if dark {
        json!({"fill": theme::PANEL, "cornerRadius": STAGE_RADIUS, "stroke": theme::BORDER, "strokeWeight": 1, "strokeAlign": "inside"})
    } else {
        json!({"fill": theme::STAGE_LIGHT, "cornerRadius": STAGE_RADIUS})
    };
    let r = host(lang, "frame.create", &json!({"x": at.x, "y": at.y, "width": at.w, "height": at.h, "name": t(lang, Key::LayerStage), "select": false, "props": props}))?;
    id_of(lang, &r, "frame.create")
}

/// The colours and weights of the redlines (badges, guides, measures and overlay) on a stage. On the
/// light stage with the default line colour the guides, the overlay's outline and the badges are the
/// line colour at full strength (2 px guides, 1.5 px outline), with Prussian blue for the measure values (the line
/// colour is too pale to read on the light stage) and a Prussian ring on the badge. On the dark
/// stage, or with a line colour set in the settings, everything is the line colour, 1 px, no ring.
struct RedStyle {
    accent: String,
    ring: Option<&'static str>,
    guide: String,
    guide_weight: f64,
    overlay_fill: String,
    overlay_stroke: String,
    overlay_weight: f64,
    /// The colour of the measure values.
    value: String,
}

fn red_style(dark: bool, custom_line: bool, line: &str, fill: &str) -> RedStyle {
    if dark || custom_line {
        RedStyle {
            accent: line.to_string(),
            ring: None,
            guide: line.to_string(),
            guide_weight: 1.0,
            overlay_fill: fill.to_string(),
            overlay_stroke: line.to_string(),
            overlay_weight: 1.0,
            value: line.to_string(),
        }
    } else {
        RedStyle {
            accent: line.to_string(),
            ring: Some(theme::PRUSSIAN),
            guide: line.to_string(),
            guide_weight: 2.0,
            overlay_fill: fill.to_string(),
            overlay_stroke: line.to_string(),
            overlay_weight: 1.5,
            value: theme::PRUSSIAN.to_string(),
        }
    }
}

/// The patch behind a measure's value: the stage's own colour.
fn stage_fill(dark: bool) -> &'static str {
    if dark { theme::PANEL } else { theme::STAGE_LIGHT }
}

/// The first solid paint of `node` that is drawn, as a colour (layer and paint opacity in its alpha).
fn solid_fill(node: &Value) -> Option<Rgba> {
    let layer = node.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
    node.get("fills").and_then(Value::as_array)?.iter().find(|p| p.get("visible").and_then(Value::as_bool) != Some(false) && p.get("opacity").and_then(Value::as_f64).is_none_or(|o| o > 0.0)).and_then(|paint| {
        if paint.get("type").and_then(Value::as_str) != Some("solid") {
            return None;
        }
        let own = paint.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
        paint.get("color").and_then(Value::as_str).and_then(|c| contrast::parse_hex(c, own * layer))
    })
}

/// The colours of the visible elements under the root (fills, strokes and text), as drawn.
fn element_colours(nodes: &[Value]) -> Vec<Rgba> {
    let mut out = Vec::new();
    for n in nodes.iter().skip(1) {
        if n.get("visible").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let layer = n.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
        for key in ["fills", "strokes"] {
            for paint in n.get(key).and_then(Value::as_array).into_iter().flatten() {
                if paint.get("visible").and_then(Value::as_bool) == Some(false) || paint.get("type").and_then(Value::as_str) != Some("solid") {
                    continue;
                }
                let own = paint.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
                if let Some(c) = paint.get("color").and_then(Value::as_str).and_then(|c| contrast::parse_hex(c, own * layer)) {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// The component's effective background: its root's solid fill, else the one of the first ancestor
/// that has one (`ancestor`: the component set's, for a variant), else none (transparent).
fn effective_backdrop(nodes: &[Value], ancestor: Option<Rgba>) -> Option<Rgba> {
    nodes.first().and_then(solid_fill).or(ancestor)
}

/// Whether a stage is dark: always for `Dark`, never for `Light`, and for `Auto` the one of the
/// two stages with the higher lowest contrast (see `contrast::pick_stage`; a tie is light). Also
/// gives that contrast.
fn is_dark(mode: StageMode, nodes: &[Value], ancestor: Option<Rgba>) -> (bool, Option<f64>) {
    match mode {
        StageMode::Light => (false, None),
        StageMode::Dark => (true, None),
        StageMode::Auto => {
            let pick = contrast::pick_stage(effective_backdrop(nodes, ancestor), &element_colours(nodes));
            (pick.dark, pick.lowest)
        }
    }
}

/// The solid fill of the component set a variant belongs to, for the auto stage (a variant's first
/// ancestor with a fill; the host's `node.get` gives no parent, so a set is the only ancestor known).
fn set_backdrop(lang: Lang, target: &Picked) -> Result<Option<Rgba>> {
    if target.holder == target.component {
        return Ok(None);
    }
    Ok(solid_fill(&host(lang, "node.get", &json!({"id": target.holder, "fields": ["fills", "opacity"]}))?))
}

/// Place an instance of `component` in the middle of `stage` (whose box is `at`); returns it and its
/// box (it has the component's size).
fn place_instance(lang: Lang, component: u64, stage: u64, at: Rect, size: Rect) -> Result<(u64, Rect)> {
    let placed = host(lang, "instance.place", &json!({"component": component, "x": at.x + at.w / 2.0, "y": at.y + at.h / 2.0}))?;
    let instance = id_of(lang, &placed, "instance.place")?;
    let parent = placed.get("parent").and_then(Value::as_u64);
    if parent != Some(stage) {
        return Err(tf(lang, Key::ErrParent, &[("parent", &format!("{parent:?}")), ("stage", &stage.to_string())]));
    }
    Ok((instance, Rect { x: at.x + (at.w - size.w) / 2.0, y: at.y + (at.h - size.h) / 2.0, w: size.w, h: size.h }))
}

/// The padding and gap measures on the stage: a rectangle with an outline for each line of
/// `measure.padding`, and its value on a patch over it. Returns the rectangles' and values' ids.
#[allow(clippy::too_many_arguments)]
fn draw_overlay(lang: Lang, stage: u64, sb: Rect, ib: Rect, lines: &[Value], red: &RedStyle, patch: &str, notes: &mut Vec<String>) -> Result<(Vec<u64>, Vec<u64>)> {
    let mut rects = Vec::new();
    let mut labels = Vec::new();
    let mut pending: Vec<(String, f64, f64)> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let (Some(x0), Some(x1), Some(y0), Some(y1)) = (num(l, "x0"), num(l, "x1"), num(l, "y0"), num(l, "y1")) else {
            notes.push(tf(lang, Key::NoteLineNoCoords, &[("i", &i.to_string())]));
            continue;
        };
        let label = l.get("label").and_then(Value::as_str).unwrap_or_default().to_string();
        let (rx, ry, rw, rh);
        let (lx, ly);
        let tw = CHAR_W * label.chars().count() as f64;
        if (y0 - y1).abs() < f64::EPSILON && x1 > x0 {
            // A horizontal measure: the band spans the instance's height.
            (rx, ry, rw, rh) = (x0, ib.y, x1 - x0, ib.h);
            let edge = (x0 - ib.x).abs() < 0.5 || (x1 - ib.right()).abs() < 0.5;
            lx = (x0 + x1) / 2.0 - tw / 2.0;
            ly = if edge { ib.y - LABEL_H - 4.0 } else { ib.bottom() + 6.0 };
        } else if (x0 - x1).abs() < f64::EPSILON && y1 > y0 {
            // A vertical measure: the band spans the instance's width.
            (rx, ry, rw, rh) = (ib.x, y0, ib.w, y1 - y0);
            let edge = (y0 - ib.y).abs() < 0.5 || (y1 - ib.bottom()).abs() < 0.5;
            ly = (y0 + y1) / 2.0 - LABEL_H / 2.0;
            lx = if edge { ib.right() + 10.0 } else { ib.x - 10.0 - tw };
        } else {
            notes.push(tf(lang, Key::NoteLineOblique, &[("i", &i.to_string()), ("label", &label)]));
            continue;
        }
        let r = host(
            lang,
            "shape.rectangle",
            &json!({"x": rx - sb.x, "y": ry - sb.y, "width": rw, "height": rh, "parent": stage, "name": tf(lang, Key::LayerMeasure, &[("value", &label)]), "select": false,
                "props": {"fill": red.overlay_fill, "stroke": red.overlay_stroke, "strokeWeight": red.overlay_weight, "strokeAlign": "inside"}}),
        )?;
        rects.push(id_of(lang, &r, "shape.rectangle")?);
        pending.push((label, lx, ly));
    }
    // The values go on last, so they end up on top.
    for (label, lx, ly) in &pending {
        labels.push(value_label(lang, stage, (sb.x, sb.y), label, *lx, *ly, &red.value, patch, &tf(lang, Key::LayerValue, &[("value", label)]))?);
    }
    Ok((rects, labels))
}

/// The badge's circle: the accent colour, with a ring when the style has one.
fn badge_props(red: &RedStyle) -> Value {
    match red.ring {
        Some(ring) => json!({"fill": red.accent, "stroke": ring, "strokeWeight": theme::RING_WEIGHT, "strokeAlign": "outside"}),
        None => json!({"fill": red.accent}),
    }
}

/// What `draw_badges` made.
struct Badges {
    ids: Vec<u64>,
    info: Vec<Value>,
    /// Numbers with no marker on the stage: no room that is clear and untangled.
    dropped: Vec<usize>,
}

/// The layers' badges on the stage: outside the component, on the free side nearest to the layer,
/// each with a guide to the layer's border.
#[allow(clippy::too_many_arguments)]
fn draw_badges(lang: Lang, stage: u64, sb: Rect, ib: Rect, tree: &Tree, numbered: &[(usize, usize)], red: &RedStyle) -> Result<Badges> {
    // Each round places what is left; a marker left out (no clear room, or a tangled guide) tries
    // another side of its layer in the next, up to two more times.
    let mut skips: Vec<Vec<Side>> = vec![Vec::new(); numbered.len()];
    let mut markers: Vec<Marker> = Vec::new();
    for round in 0..3 {
        let mut items: Vec<(usize, Target)> = Vec::new();
        let mut owner: Vec<usize> = Vec::new();
        let mut used: Vec<(f64, f64)> = Vec::new();
        for (idx, (n, node)) in numbered.iter().enumerate() {
            if let Some(t) = layer_target(*node, tree, ib, &used, &skips[idx]) {
                used.push(t.end);
                items.push((*n, t));
                owner.push(idx);
            }
        }
        let (kept, _) = drop_tangled(place_markers(ib, sb, &items, &[]));
        let missing: Vec<usize> = (0..items.len()).filter(|i| !kept.iter().any(|m| m.item == *i)).collect();
        markers = kept;
        if missing.is_empty() || round == 2 {
            break;
        }
        for i in missing {
            if let (Some(idx), Some((_, t))) = (owner.get(i), items.get(i)) {
                skips[*idx].push(t.side);
            }
        }
    }
    // A number is left out of the stage only when none of its markers is left.
    let mut dropped: Vec<usize> = numbered.iter().map(|(n, _)| *n).filter(|n| !markers.iter().any(|m| m.number == *n)).collect();
    dropped.dedup();
    let origin = (sb.x, sb.y);
    let mut ids = Vec::new();
    for m in &markers {
        let (dx, dy) = (m.end.0 - m.centre.0, m.end.1 - m.centre.1);
        let d = dx.hypot(dy).max(0.001);
        // The guide starts at the circle's edge.
        let start = (m.centre.0 + dx / d * MARKER_D / 2.0, m.centre.1 + dy / d * MARKER_D / 2.0);
        leader(lang, stage, origin, start, m.end, (red.guide.as_str(), red.guide_weight), &tf(lang, Key::LayerLeader, &[("n", &m.number.to_string())]))?;
        let name = tf(lang, Key::LayerMarker, &[("n", &m.number.to_string())]);
        let c = host(
            lang,
            "shape.ellipse",
            &json!({"x": m.centre.0 - MARKER_D / 2.0 - origin.0, "y": m.centre.1 - MARKER_D / 2.0 - origin.1, "width": MARKER_D, "height": MARKER_D, "parent": stage,
                "name": name, "select": false, "props": badge_props(red)}),
        )?;
        let digits = m.number.to_string();
        let tw = CHAR_W * digits.chars().count() as f64;
        text(lang, stage, &digits, m.centre.0 - tw / 2.0 - origin.0, m.centre.1 - LABEL_H / 2.0 - origin.1, FONT, theme::ON_REDLINE, None, &name)?;
        ids.push(id_of(lang, &c, "shape.ellipse")?);
    }
    let info = markers.iter().map(|m| json!({"n": m.number, "centre": [m.centre.0, m.centre.1], "end": [m.end.0, m.end.1]})).collect();
    Ok(Badges { ids, info, dropped })
}

/// A section of the report: its frame, a title and a divider under it, and the row that holds the
/// legend (left) and the stage (right).
fn section(lang: Lang, frame: u64, inner: f64, name: &str, title: &str) -> Result<(u64, u64)> {
    let section = section_frame(lang, frame, inner, name, title)?;
    let mut ar = FlowSpec::new(t(lang, Key::LayerAnatomyRow), inner, "horizontal");
    ar.gap = ANATOMY_GAP;
    let row = flow_frame(lang, section, ar)?;
    Ok((section, row))
}

/// A legend frame in `row`.
fn legend_frame(lang: Lang, row: u64) -> Result<u64> {
    let mut lg = FlowSpec::new(t(lang, Key::LayerLegend), LEGEND_W, "vertical");
    lg.gap = BLOCK_GAP;
    flow_frame(lang, row, lg)
}

/// A frame's height (0 when it can't be read).
fn height_of(lang: Lang, id: u64) -> Result<f64> {
    Ok(bounds(&host(lang, "node.get", &json!({"id": id, "fields": ["absoluteBounds"]}))?).map_or(0.0, |b| b.h))
}

/// A component property as the report shows it: its name, type and options (the default marked).
struct PropView {
    name: String,
    kind: String,
    options: Vec<(String, bool)>,
}

/// The name of a property without the "#id" suffix the host adds to booleans and the like.
fn clean_name(name: &str) -> &str {
    name.split('#').next().unwrap_or(name).trim()
}

/// The properties of a component as `component.properties` gives them. `default` holds the pairs
/// of the default variant, which say the default option of each variant property.
fn property_views(lang: Lang, defs: &Value, default: Option<&Pairs>) -> Vec<PropView> {
    let list = defs.get("properties").and_then(Value::as_array).cloned().unwrap_or_default();
    list.iter()
        .filter_map(|p| {
            let name = clean_name(p.get("name").and_then(Value::as_str)?).to_string();
            let kind = p.get("type").and_then(Value::as_str).unwrap_or_default().to_string();
            let options: Vec<(String, bool)> = match kind.as_str() {
                "variant" => {
                    let chosen = default.and_then(|d| d.iter().find(|(k, _)| *k == name).map(|(_, v)| v.clone())).or_else(|| p.get("default").and_then(Value::as_str).map(ToString::to_string));
                    p.get("options").and_then(Value::as_array).map(|o| o.iter().filter_map(Value::as_str).map(|o| (o.to_string(), chosen.as_deref() == Some(o))).collect()).unwrap_or_default()
                }
                "boolean" => {
                    let on = p.get("default").and_then(Value::as_bool).unwrap_or(false);
                    vec![(t(lang, Key::ValueTrue).to_string(), on), (t(lang, Key::ValueFalse).to_string(), !on)]
                }
                _ => p.get("default").and_then(Value::as_str).map(|d| vec![(d.to_string(), true)]).unwrap_or_default(),
            };
            Some(PropView { name, kind, options })
        })
        .collect()
}

/// The glyph and word of a property's type, in the report's words; unknown types keep their own name.
fn type_text(lang: Lang, kind: &str) -> String {
    match kind {
        "variant" => glyphs::with_word(glyphs::VARIANT, t(lang, Key::TypeVariant)),
        "boolean" => glyphs::with_word(glyphs::BOOLEAN, t(lang, Key::TypeBoolean)),
        "text" => glyphs::with_word(glyphs::TEXT, t(lang, Key::TypeText)),
        "instanceSwap" => glyphs::with_word(glyphs::INSTANCE, t(lang, Key::TypeInstance)),
        other => other.to_string(),
    }
}

/// A section's frame, with its title and divider.
fn section_frame(lang: Lang, frame: u64, inner: f64, name: &str, title: &str) -> Result<u64> {
    let mut sec = FlowSpec::new(name, inner, "vertical");
    sec.gap = 16.0;
    let section = flow_frame(lang, frame, sec)?;
    text(lang, section, title, 0.0, 0.0, theme::SECTION_SIZE, theme::TEXT, Some(inner), title)?;
    host(
        lang,
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": inner, "height": 1, "parent": section, "name": t(lang, Key::LayerDivider), "select": false, "props": {"fill": theme::BORDER}}),
    )?;
    Ok(section)
}

/// The Properties section: a row for each property, with its type and its options as chips. The
/// default option is bold with a white 1.5 px border; the others are dimmer with a faint border.
/// The chips wrap inside their own column, so a second line stays under the first.
fn draw_properties(lang: Lang, frame: u64, inner: f64, views: &[PropView]) -> Result<()> {
    let section = section_frame(lang, frame, inner, t(lang, Key::LayerProperties), t(lang, Key::PropertiesTitle))?;
    let mut rows = FlowSpec::new(t(lang, Key::LayerProperties), inner, "vertical");
    rows.gap = 16.0;
    let rows = flow_frame(lang, section, rows)?;
    let chips_w = inner - PROP_NAME_W - PROP_TYPE_W - 2.0 * 12.0;
    for v in views {
        let name = tf(lang, Key::LayerPropertyRow, &[("name", &v.name)]);
        let mut r = FlowSpec::new(&name, inner, "horizontal");
        r.gap = 12.0;
        let row = flow_frame(lang, rows, r)?;
        text_with(lang, row, &v.name, LINE_SIZE, theme::TEXT, &name, json!({"width": PROP_NAME_W, "fontStyle": "Bold"}))?;
        text_with(lang, row, &type_text(lang, &v.kind), LINE_SIZE, theme::MUTED, &name, json!({"width": PROP_TYPE_W}))?;
        let mut c = FlowSpec::new(t(lang, Key::LayerChip), chips_w, "horizontal");
        c.gap = 8.0;
        c.extra = json!({"wrap": true, "crossGap": 8});
        let chips = flow_frame(lang, row, c)?;
        for (option, is_default) in &v.options {
            let mut chip = FlowSpec::new(t(lang, Key::LayerChip), 20.0, "horizontal");
            chip.padding = json!([3, 10, 3, 10]);
            chip.fill = Some(theme::PANEL);
            let (stroke, weight) = if *is_default { (theme::TEXT, 1.5) } else { (theme::BORDER, 1.0) };
            chip.extra = json!({"sizingH": "hug", "cornerRadius": 12, "stroke": stroke, "strokeWeight": weight, "strokeAlign": "inside"});
            let chip = flow_frame(lang, chips, chip)?;
            let extra = if *is_default { json!({"fontStyle": "Bold"}) } else { json!({}) };
            text_with(lang, chip, option, LINE_SIZE, if *is_default { theme::TEXT } else { theme::MUTED_70 }, t(lang, Key::LayerChip), extra)?;
        }
    }
    Ok(())
}

/// What the axes drew.
struct AxesDrawn {
    rows: usize,
    skipped: usize,
    undefined: Vec<String>,
    /// For each row: its name, the lowest contrast found (in auto) and whether its stage is dark.
    info: Vec<Value>,
}

/// The variables and styles of the file, read the first time a layer needs them (the default's walk may
/// not have needed them, and a variant's may).
struct Caches {
    vars: Vars,
    vars_loaded: bool,
    styles: Styles,
    styles_loaded: bool,
}

impl Caches {
    /// Load what the layers of `walked` are bound to and the caches don't have yet.
    fn load_for(&mut self, lang: Lang, walked: &[(Value, usize)]) -> Result<()> {
        let nodes: Vec<Value> = walked.iter().map(|(n, _)| n.clone()).collect();
        if !self.vars_loaded && any_bound(&nodes) {
            self.vars = Vars::parse(&host(lang, "variable.list", &json!({}))?);
            self.vars_loaded = true;
        }
        if !self.styles_loaded && any_styled(&nodes) {
            self.styles = Styles::parse(&host(lang, "style.list", &json!({}))?);
            self.styles_loaded = true;
        }
        Ok(())
    }
}

/// A text of the legend's width (it wraps), with more properties than `text` gives.
fn legend_text(lang: Lang, parent: u64, s: &str, fill: &str, name: &str, bold: bool) -> Result<u64> {
    let mut props = json!({"fontSize": LINE_SIZE.max(theme::MIN_SIZE), "fill": fill, "fontFamily": theme::FONT_FAMILY});
    if let (true, Some(p)) = (bold, props.as_object_mut()) {
        p.insert("fontStyle".into(), json!("Bold"));
    }
    let r = host(lang, "text.create", &json!({"text": s, "x": 0, "y": 0, "width": LEGEND_W, "parent": parent, "name": name, "select": false, "props": props}))?;
    id_of(lang, &r, "text.create")
}

/// The variant axes: a section for each variant property, with a row for each option. On the left of
/// a row, what that variant changes from the default (the default says "Default" and nothing else); on
/// the right, a new instance of the variant. The other properties stay at the default. Missing
/// combinations are listed at the end of their section. At most `limit` rows.
#[allow(clippy::too_many_arguments)]
fn draw_axes(
    lang: Lang,
    frame: u64,
    inner: f64,
    stage_w: f64,
    x: f64,
    axes: &[variants::Axis],
    limit: usize,
    mode: StageMode,
    ancestor: Option<Rgba>,
    base_walked: &[(Value, usize)],
    caches: &mut Caches,
) -> Result<AxesDrawn> {
    let mut drawn = AxesDrawn { rows: 0, skipped: 0, undefined: Vec::new(), info: Vec::new() };
    let mut last_section: Option<u64> = None;
    let base_nodes: Vec<Value> = base_walked.iter().map(|(n, _)| n.clone()).collect();
    let base_snap = delta::snapshot(lang, base_walked, &caches.vars, &caches.styles);
    for axis in axes {
        let mut section: Option<u64> = None;
        let mut missing: Vec<String> = Vec::new();
        for opt in &axis.options {
            let Some(variant) = &opt.variant else {
                missing.push(format!("{}={}", axis.prop, opt.option));
                continue;
            };
            if drawn.rows >= limit {
                drawn.skipped += 1;
                continue;
            }
            let sec = match section {
                Some(s) => s,
                None => {
                    let s = section_frame(lang, frame, inner, &tf(lang, Key::LayerAxis, &[("name", &axis.prop)]), &axis.prop)?;
                    section = Some(s);
                    s
                }
            };
            last_section = Some(sec);
            let rname = tf(lang, Key::LayerAxisRow, &[("name", &opt.name)]);
            let mut r = FlowSpec::new(&rname, inner, "horizontal");
            r.gap = ANATOMY_GAP;
            let row = flow_frame(lang, sec, r)?;
            let mut lg = FlowSpec::new(t(lang, Key::LayerLegend), LEGEND_W, "vertical");
            lg.gap = 8.0;
            let legend = flow_frame(lang, row, lg)?;
            let title = glyphs::with_word(glyphs::VARIANT, &format!("{}={}", axis.prop, opt.option));
            text_with(lang, legend, &title, HEADER_SIZE, theme::TEXT, &rname, json!({"fontStyle": "Bold"}))?;
            // The stage first with the room the instance needs; it grows when the list is taller.
            let h0 = (variant.h + AXIS_ROOM).max(AXIS_MIN_H);
            let sb = Rect { x, y: STAGING_Y, w: stage_w, h: h0 };
            let stage = make_stage(lang, sb, mode == StageMode::Dark)?;
            let (instance, _) = place_instance(lang, variant.id, stage, sb, Rect { x: 0.0, y: 0.0, w: variant.w, h: variant.h })?;
            let mut listed: Vec<String> = Vec::new();
            let mut h = h0;
            let nodes: Vec<Value> = if opt.is_default {
                text_with(lang, legend, t(lang, Key::LegendDefault), LINE_SIZE, theme::MUTED, &rname, json!({}))?;
                base_nodes.clone()
            } else {
                let walked = walk_layers(lang, instance)?;
                caches.load_for(lang, &walked)?;
                let diff = delta::compare(&base_snap, &delta::snapshot(lang, &walked, &caches.vars, &caches.styles));
                let listing = delta::list(lang, &diff);
                for l in &listing.lines {
                    legend_text(lang, legend, &l.text, theme::TEXT, t(lang, Key::LayerDeltas), l.heading)?;
                }
                if listing.more > 0 {
                    legend_text(lang, legend, &tf(lang, plural(listing.more, Key::DeltaMoreOne, Key::DeltaMore), &[("n", &listing.more.to_string())]), theme::MUTED, t(lang, Key::LayerMore), false)?;
                }
                if let Some(none) = &listing.none {
                    legend_text(lang, legend, none, theme::MUTED, t(lang, Key::LayerDeltas), false)?;
                }
                listed = listing.texts(Lang::En);
                // The row is as tall as its list when that is taller than the instance needs.
                let legend_h = height_of(lang, legend)?;
                if legend_h > h0 + 0.5 {
                    h = legend_h;
                    host(lang, "node.set", &json!({"ids": [stage], "props": {"height": h}}))?;
                    host(lang, "node.set", &json!({"ids": [instance], "props": {"y": (h - variant.h) / 2.0}}))?;
                }
                walked.into_iter().map(|(n, _)| n).collect()
            };
            let (dark, lowest) = is_dark(mode, &nodes, ancestor);
            if dark && mode == StageMode::Auto {
                host(lang, "node.set", &json!({"ids": [stage], "props": {"fill": theme::PANEL, "stroke": theme::BORDER, "strokeWeight": 1, "strokeAlign": "inside"}}))?;
            }
            drawn.info.push(json!({"name": opt.name, "lowest_contrast": lowest, "dark": dark, "height": h, "deltas": listed}));
            host(lang, "node.reparent", &json!({"ids": [stage], "parent": row}))?;
            drawn.rows += 1;
        }
        if let (Some(sec), false) = (section, missing.is_empty()) {
            text(lang, sec, &tf(lang, Key::NotDefined, &[("list", &missing.join(", "))]), 0.0, 0.0, FONT, theme::MUTED, Some(inner), t(lang, Key::LayerAxisNote))?;
        }
        drawn.undefined.extend(missing);
    }
    if let (Some(sec), true) = (last_section, drawn.skipped > 0) {
        text(lang, sec, &tf(lang, plural(drawn.skipped, Key::MoreVariantsOne, Key::MoreVariants), &[("n", &drawn.skipped.to_string())]), 0.0, 0.0, FONT, theme::MUTED, Some(inner), t(lang, Key::LayerAxisNote))?;
    }
    Ok(drawn)
}

/// The counts of the header's summary line.
struct SummaryCounts {
    bound: usize,
    check: usize,
    unbound: usize,
    missing: usize,
    suspect: usize,
    total: usize,
    vars: usize,
    styles: usize,
}

/// The summary line. Its text depends on whether some bindings are "to check" and whether some are
/// styles; its nouns (variables, styles, properties) are said in the singular when their count is
/// exactly 1, each noun with a text of its own. With any other count the line is what it always was.
fn summary_text(lang: Lang, c: &SummaryCounts) -> String {
    let key = match (c.check > 0, c.styles > 0) {
        (false, false) => Key::Summary,
        (true, false) => Key::SummaryChecks,
        (false, true) => Key::SummaryStyles,
        (true, true) => Key::SummaryStylesChecks,
    };
    let noun = |n: usize, one: Key, many: Key| t(lang, plural(n, one, many));
    let values: Vec<(&str, String)> = vec![
        ("bound", c.bound.to_string()),
        ("check", c.check.to_string()),
        ("unbound", c.unbound.to_string()),
        ("missing", c.missing.to_string()),
        ("suspect", c.suspect.to_string()),
        ("total", c.total.to_string()),
        ("vars", c.vars.to_string()),
        ("styles", c.styles.to_string()),
        ("vars_word", noun(c.vars, Key::NounVariable, Key::NounVariables).to_string()),
        ("styles_word", noun(c.styles, Key::NounStyle, Key::NounStyles).to_string()),
        ("total_word", noun(c.total, Key::NounProperty, Key::NounProperties).to_string()),
    ];
    let pairs: Vec<(&str, &str)> = values.iter().map(|(k, v)| (*k, v.as_str())).collect();
    tf(lang, key, &pairs)
}

/// A band with a warning, in the line colour: inside `parent`, `inner` wide.
fn warning_band(lang: Lang, parent: u64, inner: f64, message: &str, line: &str) -> Result<()> {
    let mut band = FlowSpec::new(t(lang, Key::LayerWarning), inner, "horizontal");
    band.padding = json!([10, 12, 10, 12]);
    band.fill = Some(theme::WARN_BG);
    band.extra = json!({"stroke": line, "strokeWeight": 1, "strokeAlign": "inside", "cornerRadius": 8});
    let band = flow_frame(lang, parent, band)?;
    text(lang, band, message, 0.0, 0.0, FONT, theme::TEXT, Some(inner - 24.0), t(lang, Key::LayerWarning))?;
    Ok(())
}

pub fn generate(input: Input) -> Result<Value> {
    // 1. The page's settings come first: they give the language, even for an error.
    let page = page_info()?;
    let lang = page.settings.lang.as_deref().map_or(Lang::En, Lang::parse);
    if input.selection.is_empty() {
        return Err(t(lang, Key::ErrNoSelection).to_string());
    }
    let id_source = "selection: Input.selection (the `selection` field of the run's input; not a FigCraft command)";

    // 2. The component, and its sheet.
    let target = resolve(lang, &input.selection)?;
    let component = target.component;
    let mut notes = target.notes.clone();
    let sheet = host(lang, "node.get", &json!({"id": target.holder, "fields": ["pluginData"]}))?;
    let meta = Meta::from_plugin_data(sheet.get("pluginData").unwrap_or(&Value::Null));
    let versao = meta.version.as_deref().unwrap_or_default();
    let status = meta.status.as_deref().unwrap_or_default();
    let cor_linha = line_colour(page.settings.line.as_deref());
    let cor_fill = fill_colour(page.settings.fill.as_deref(), &cor_linha);
    let comp = &target.node;
    let name = target.title.clone();
    let size = bounds(comp).unwrap_or(Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 });
    let variant = target.holder != component;

    // 2b. The date: read from the app's clock, afresh on every run (never from a saved date). It can't
    // fail the run: without it the report says so.
    let stamp = clock::read(page.id, &mut |cmd, params| call(cmd, params));
    let generated = stamp.created.map(|created| clock::date_text(created, page.settings.tz_hours(), lang));

    // 3. Where: in place of the report of this component that is on the page (when replacing), else
    // to the right of everything. The old report goes first.
    let rerun = page.settings.rerun();
    let key = target.key.to_string();
    let old = page.children.iter().rev().find(|c| c.get("pluginData").and_then(|d| d.get(REPORT_KEY)).and_then(Value::as_str) == Some(key.as_str()));
    let top_level = page.children.len();
    let mut replaced = None;
    let mut at = None;
    if let (Some(old), Rerun::Replace) = (old, rerun)
        && let (Some(b), Some(old_id)) = (bounds(old), old.get("id").and_then(Value::as_u64))
    {
        host(lang, "edit.delete", &json!({"ids": [old_id]}))?;
        replaced = Some(old_id);
        at = Some((b.x, b.y));
    }
    let (x, y) = at.unwrap_or_else(|| {
        let right = page.children.iter().filter_map(bounds).map(Rect::right).fold(None, |r: Option<f64>, v| Some(r.map_or(v, |r| r.max(v))));
        (right.map_or(0.0, |r| r + GAP_RIGHT), 0.0)
    });

    // 3b. The component's properties, and the variant axes of a set.
    let defs = host(lang, "component.properties", &json!({"id": component})).ok();
    let views = defs.as_ref().map(|d| property_views(lang, d, target.set.as_ref().map(|sp| &sp.default))).unwrap_or_default();
    let axes_list: Vec<variants::Axis> = match &target.set {
        Some(sp) => {
            let order: Vec<(String, Vec<String>)> =
                views.iter().filter(|v| v.kind == "variant").map(|v| (v.name.clone(), v.options.iter().map(|(o, _)| o.clone()).collect())).collect();
            variants::axes(&sp.info, &sp.default, &order)
        }
        None => Vec::new(),
    };
    let widest = axes_list.iter().flat_map(|a| &a.options).filter_map(|o| o.variant.as_ref()).take(MAX_AXIS_ROWS).map(|v| v.w).fold(size.w, f64::max);

    // 4. The report's frame and its header.
    let frame_w = MIN_FRAME_W.max(widest + STAGE_ROOM + LEGEND_W + ANATOMY_GAP + 2.0 * FRAME_PAD);
    let inner = frame_w - 2.0 * FRAME_PAD;
    let stage_w = inner - LEGEND_W - ANATOMY_GAP;
    let frame_name = tf(lang, Key::FrameName, &[("name", &name)]);
    let frame = host(
        lang,
        "frame.create",
        &json!({"x": x, "y": y, "width": frame_w, "height": 200, "name": frame_name, "select": false,
            "props": {"fill": theme::BG, "layout": "vertical", "gap": SECTION_GAP, "padding": FRAME_PAD, "sizingH": "fixed", "sizingV": "hug", "cornerRadius": 12,
                "pluginData": {REPORT_KEY: key}}}),
    )?;
    let frame = id_of(lang, &frame, "frame.create")?;
    let mut hd = FlowSpec::new(t(lang, Key::LayerHeaderBlock), inner, "vertical");
    hd.gap = 8.0;
    let head = flow_frame(lang, frame, hd)?;
    text(lang, head, &name, 0.0, 0.0, theme::TITLE_SIZE, theme::TEXT, Some(inner), t(lang, Key::LayerTitle))?;
    let sub: Vec<&str> = [versao, status].into_iter().filter(|s| !s.is_empty()).collect();
    if !sub.is_empty() {
        text(lang, head, &sub.join(" · "), 0.0, 0.0, FONT, theme::MUTED, Some(inner), t(lang, Key::LayerSubtitle))?;
    }
    text(lang, head, &clock::generated_line(stamp.created, page.settings.tz_hours(), lang), 0.0, 0.0, FONT, theme::MUTED, Some(inner), t(lang, Key::LayerGenerated))?;
    // What went wrong with the date shows at the top of the report.
    if let Some(reason) = &stamp.failure {
        warning_band(lang, head, inner, &tf(lang, Key::WarnNoDate, &[("reason", reason)]), &cor_linha)?;
    }
    if let Some(id) = stamp.leftover {
        warning_band(lang, head, inner, &tf(lang, Key::WarnTmpComment, &[("id", &id.to_string())]), &cor_linha)?;
    }
    // A set says which of its variants the report is about.
    if let Some(sp) = &target.set {
        let source = t(lang, match sp.source { DefaultSource::Saved => Key::SourceSaved, DefaultSource::Component => Key::SourceComponent, DefaultSource::First => Key::SourceFirst });
        text(lang, head, &tf(lang, Key::DefaultLine, &[("pairs", &variants::join(&sp.default)), ("source", source)]), 0.0, 0.0, FONT, theme::MUTED, Some(inner), t(lang, Key::LayerSubtitle))?;
    }
    // The Properties section follows the header, before the anatomy.
    if !views.is_empty() {
        draw_properties(lang, frame, inner, &views)?;
    }

    // 5. The anatomy stage, first with the least height: the instance placed on it is what is audited.
    let stage_x = x;
    let h0 = (size.h + STAGE_ROOM).max(MIN_STAGE_H);
    let sb0 = Rect { x: stage_x, y: STAGING_Y, w: stage_w, h: h0 };
    let mode = page.settings.stage_mode();
    let custom_line = page.settings.line.is_some();
    let stage = make_stage(lang, sb0, mode == StageMode::Dark)?;
    let (instance, ib0) = place_instance(lang, component, stage, sb0, size)?;
    let walked = walk_layers(lang, instance)?;
    let mut nodes: Vec<Value> = walked.iter().map(|(n, _)| n.clone()).collect();
    let audited = nodes.len();
    let vars = if any_bound(&nodes) { Vars::parse(&host(lang, "variable.list", &json!({}))?) } else { Vars::default() };
    // The styles are read once per run, and only when a layer has one applied.
    let styles = if any_styled(&nodes) { Styles::parse(&host(lang, "style.list", &json!({}))?) } else { Styles::default() };
    let (vars_loaded, styles_loaded) = (any_bound(&nodes), any_styled(&nodes));
    let mut all_rows: Vec<Row> = Vec::new();
    for (i, (n, depth)) in walked.iter().enumerate() {
        for mut r in audit::rows_for(lang, n, &vars, &styles) {
            r.node = i;
            r.depth = *depth;
            all_rows.push(r);
        }
    }
    // Only what is drawn counts: a hidden layer (or one under a hidden ancestor) has no block in the legend.
    let drawn = rendered(&walked);
    let all_rows = audit::keep_shown(all_rows, &drawn);
    let all_count = all_rows.len();
    let mut rows = audit::drop_zero_unbound(all_rows);
    audit::mark_checks(&mut rows);
    let suspect = audit::suspects(&rows);
    let (bound, unbound, missing, suspect_all) = audit::counts(&rows);
    let (vars_bound, styles_bound) = audit::bound_split(&rows);
    let to_check = rows.iter().filter(|r| r.state == State::Check).count();
    let summary = summary_text(
        lang,
        &SummaryCounts { bound, check: to_check, unbound, missing, suspect: suspect_all, total: rows.len(), vars: vars_bound, styles: styles_bound },
    );
    let warning = (suspect > 0).then(|| tf(lang, plural(suspect, Key::CollisionOne, Key::Collision), &[("n", &suspect.to_string())]));
    let infos = layer_infos(lang, &walked, &drawn)?;
    let grouped = audit::group_layers(&rows, &infos);
    let line_total: usize = grouped.iter().map(|g| g.lines.len()).sum();
    let (shown, hidden) = limit_groups(&grouped, MAX_AUDIT_LINES);

    // The summary sits under the header, and the collision warning with it.
    text(lang, head, &summary, 0.0, 0.0, FONT, theme::MUTED, Some(inner), t(lang, Key::LayerSummary))?;
    for n in &notes {
        text(lang, head, n, 0.0, 0.0, FONT, theme::MUTED, Some(inner), t(lang, Key::LayerNote))?;
    }
    if let Some(w) = &warning {
        warning_band(lang, head, inner, w, &cor_linha)?;
    }

    // 6. The anatomy section: a block for each layer, and the stage to its right.
    let (_, row) = section(lang, frame, inner, t(lang, Key::LayerAnatomySection), t(lang, Key::Anatomy))?;
    let legend = legend_frame(lang, row)?;
    let calls_before = HOST_CALLS.load(Ordering::Relaxed);
    let mut anatomy_blocks = blocks(&shown, |l| !is_spacing(l.property), true);
    let unnumbered = number_blocks(&mut anatomy_blocks, &drawn);
    for b in &anatomy_blocks {
        draw_block(lang, legend, &walked, variant, b, &[], &cor_linha)?;
    }
    if let Some(note) = hidden_note(lang, hidden) {
        text(lang, legend, &note, 0.0, 0.0, FONT, theme::MUTED, Some(LEGEND_W), t(lang, Key::LayerMore))?;
    }
    if unnumbered > 0 {
        text(lang, legend, &tf(lang, plural(unnumbered, Key::UnnumberedOne, Key::Unnumbered), &[("n", &unnumbered.to_string())]), 0.0, 0.0, FONT, theme::MUTED, Some(LEGEND_W), t(lang, Key::LayerMore))?;
    }
    let legend_h = height_of(lang, legend)?;

    // The stage is as tall as the instance needs and, up to a limit, as the legend: when it grows,
    // the instance moves down to stay in the middle (so do the audited boxes).
    let h1 = h0.max(legend_h.min(STAGE_MAX_LEGEND_H));
    let (sb, ib, dy) = if (h1 - h0).abs() > 0.5 {
        let dy = (h1 - h0) / 2.0;
        host(lang, "node.set", &json!({"ids": [stage], "props": {"height": h1}}))?;
        host(lang, "node.set", &json!({"ids": [instance], "props": {"y": (h1 - size.h) / 2.0}}))?;
        (Rect { h: h1, ..sb0 }, Rect { y: ib0.y + dy, ..ib0 }, dy)
    } else {
        (sb0, ib0, 0.0)
    };
    for n in &mut nodes {
        if let Some(b) = n.get_mut("absoluteBounds").and_then(Value::as_object_mut)
            && let Some(v) = b.get("y").and_then(Value::as_f64)
        {
            b.insert("y".into(), json!(v + dy));
        }
    }
    // The stage's colour: dark or light, by the setting or, in auto, by contrast. The stage was
    // made for light and dark settings; auto may turn it dark now.
    let set_fill = if mode == StageMode::Auto { set_backdrop(lang, &target)? } else { None };
    let (anatomy_dark, anatomy_ratio) = is_dark(mode, &nodes, set_fill);
    if mode == StageMode::Auto && anatomy_dark {
        host(lang, "node.set", &json!({"ids": [stage], "props": {"fill": theme::PANEL, "stroke": theme::BORDER, "strokeWeight": 1, "strokeAlign": "inside"}}))?;
    }
    let red = red_style(anatomy_dark, custom_line, &cor_linha, &cor_fill);
    let tree = Tree::new(&nodes, walked.iter().map(|(_, d)| *d).collect());
    let numbered: Vec<(usize, usize)> = anatomy_blocks.iter().filter_map(|b| b.badge.map(|n| b.nodes.iter().filter(|i| drawn.get(**i).copied().unwrap_or(false)).map(move |i| (n, *i)))).flatten().collect();
    let badges = draw_badges(lang, stage, sb, ib, &tree, &numbered, &red)?;
    host(lang, "node.reparent", &json!({"ids": [stage], "parent": row}))?;

    // 7. The spacing section, when the root has padding or a gap: a block for each layer that has
    // some, and a second instance with the measures on it.
    let root_spaced = rows.iter().any(|r| r.node == 0 && is_spacing(r.property));
    // A component whose own box has no spacing but whose only wrapper does (a "Container" with the
    // same box as the root) gets the section too, measured on the wrapper.
    let wrapper = if root_spaced { None } else { spacing_wrapper(&walked, &drawn, &rows) };
    let mut spacing_info = Value::Null;
    let mut rects = Vec::new();
    let mut labels = Vec::new();
    if root_spaced || wrapper.is_some() {
        let (_, row2) = section(lang, frame, inner, t(lang, Key::LayerSpacingSection), t(lang, Key::SpacingTitle))?;
        let legend2 = legend_frame(lang, row2)?;
        for b in blocks(&shown, |l| is_spacing(l.property), false) {
            let extra = walked.get(b.node).map(|(n, _)| layout_lines(lang, n)).unwrap_or_default();
            draw_block(lang, legend2, &walked, variant, &b, &extra, &cor_linha)?;
        }
        let legend2_h = height_of(lang, legend2)?;
        let h2 = h0.max(legend2_h.min(STAGE_MAX_LEGEND_H));
        let sb2 = Rect { x: stage_x, y: STAGING_Y + SPACING_STAGING_DY, w: stage_w, h: h2 };
        let stage2 = make_stage(lang, sb2, anatomy_dark)?;
        let (instance2, ib2) = place_instance(lang, component, stage2, sb2, size)?;
        let measured_id = match wrapper {
            Some(w) => {
                let k = walked[..w].iter().filter(|(_, d)| *d == 1).count();
                let kids = host(lang, "node.get", &json!({"id": instance2, "fields": ["children"]}))?;
                kids.get("children").and_then(Value::as_array).and_then(|c| c.get(k)).and_then(|c| c.get("id")).and_then(Value::as_u64).unwrap_or(instance2)
            }
            None => instance2,
        };
        let measured = host(lang, "measure.padding", &json!({"id": measured_id}))?;
        let measure_lines = measured.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
        (rects, labels) = draw_overlay(lang, stage2, sb2, ib2, &measure_lines, &red, stage_fill(anatomy_dark), &mut notes)?;
        host(lang, "node.reparent", &json!({"ids": [stage2], "parent": row2}))?;
        spacing_info = json!({"stage": stage2, "instance": instance2, "stage_size": {"w": stage_w, "h": h2}, "legend_height": legend2_h});
    }
    let calls_legends = HOST_CALLS.load(Ordering::Relaxed) - calls_before;

    // The variant axes close the report.
    let mut caches = Caches { vars, vars_loaded, styles, styles_loaded };
    let axes_drawn = if axes_list.is_empty() { None } else { Some(draw_axes(lang, frame, inner, stage_w, x, &axes_list, MAX_AXIS_ROWS, mode, set_fill, &walked, &mut caches)?) };

    // The report is born with PNG export on, as the Export section of the panel would set it: 2x with no
    // suffix, or 1x (said in a note) when it is too tall for 2x.
    let frame_h = height_of(lang, frame)?;
    let too_tall = frame_h > EXPORT_2X_MAX_H;
    if too_tall {
        text(lang, head, t(lang, Key::NoteExport1x), 0.0, 0.0, FONT, theme::MUTED, Some(inner), t(lang, Key::LayerNote))?;
    }
    let export_result = match call("export.add", &json!({"ids": [frame], "setting": {"size": if too_tall { "1x" } else { "2x" }, "suffix": "", "format": "png"}})) {
        Ok(_) => "ok".to_string(),
        Err(e) => e,
    };

    // The report becomes the selection, so the user can zoom to it. A refusal doesn't fail the run.
    let selected = match call("select.set", &json!({"ids": [frame]})) {
        Ok(_) => "ok".to_string(),
        Err(e) => e,
    };
    notify(&tf(lang, Key::Done, &[("name", &name)]));

    let table: Vec<Value> = grouped
        .iter()
        .flat_map(|g| g.lines.iter().map(move |l| json!({"layer": g.name, "type": g.kind, "layers": g.nodes.len(), "property": t(Lang::En, prop_key(l.property)), "token": l.token, "value": l.value, "state": format!("{:?}", l.state), "count": g.nodes.len()})))
        .collect();
    let badge_layers: Vec<Value> = numbered.iter().map(|(n, node)| json!({"n": n, "layer": walked.get(*node).and_then(|(v, _)| v.get("name")).cloned().unwrap_or(Value::Null)})).collect();
    Ok(json!({
        "frame": frame, "stage": stage, "anatomy_row": row, "legend": legend, "instance": instance, "component": component, "rects": rects, "labels": labels, "markers": badges.ids,
        "x": x, "y": y, "replaced": replaced, "rerun": rerun.name(), "top_level_layers": top_level, "id_source": id_source,
        "settings": {"lang": lang.code(), "line": cor_linha, "fill": cor_fill}, "meta": {"version": versao, "status": status},
        "generated": {"date": generated, "created": stamp.created, "tz": page.settings.tz_hours(), "failure": stamp.failure, "leftover_comment": stamp.leftover},
        "notes": notes, "lang": lang.code(), "selected": selected,
        "instance_size": {"w": ib.w, "h": ib.h}, "stage_size": {"w": stage_w, "h": h1}, "frame_size": {"w": frame_w}, "legend_height": legend_h,
        "audited_layers": audited,
        "rows": {"all_properties": all_count, "in_table": rows.len(), "lines_after_grouping": line_total, "hidden": hidden},
        "legend_cost": {"lines": line_total - hidden, "blocks": anatomy_blocks.len(), "host_calls": calls_legends},
        "spacing": spacing_info, "spacing_wrapper": wrapper, "export": {"size": if too_tall { "1x" } else { "2x" }, "result": export_result, "frame_height": frame_h}, "badges": badge_layers,
        "properties": views.iter().map(|v| json!({"name": v.name, "type": v.kind, "options": v.options.iter().map(|(o, d)| json!({"option": o, "default": d})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "default_variant": target.set.as_ref().map(|sp| json!({"pairs": variants::join(&sp.default), "source": format!("{:?}", sp.source)})),
        "axes": axes_drawn.as_ref().map(|a| json!({"rows": a.rows, "skipped": a.skipped, "undefined": a.undefined, "planned": variants::within_limit(&axes_list, MAX_AXIS_ROWS).0, "stages": a.info})),
        "stage_mode": mode.name(), "anatomy_stage": {"dark": anatomy_dark, "lowest_contrast": anatomy_ratio}, "host_calls": HOST_CALLS.load(Ordering::Relaxed), "badge_info": badges.info, "badges_dropped": badges.dropped, "unnumbered_layers": unnumbered,
        "anatomy_blocks": anatomy_blocks.iter().map(|b| json!({"title": block_name(&walked, b), "badge": b.badge, "count": b.nodes.len()})).collect::<Vec<_>>(),
        "hidden_paints": nodes.iter().map(audit::hidden_paints).sum::<usize>(),
        "summary": summary, "warning": warning, "table": table
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn tokens_break_at_slashes_and_arrows_only() {
        let s = "Semantico/color/action/bg → Tokens/color/fill/primary";
        let w = wrap(s, 30);
        for line in w.lines() {
            assert!(line.chars().count() <= 30, "{line}");
        }
        assert_eq!(w.replace(['\n', ' '], ""), s.replace(' ', ""));
        // A short token stays on one line.
        assert_eq!(wrap("Tokens/radius/12", 50), "Tokens/radius/12");
    }

    #[test]
    fn fill_colour_is_the_line_colour_at_thirty_percent() {
        assert_eq!(fill_colour(None, "#FF4D6D"), "#FF4D6D4D");
        assert_eq!(fill_colour(Some(""), "#3B5BDB"), "#3B5BDB4D");
        assert_eq!(fill_colour(Some("#00FF0080"), "#3B5BDB"), "#00FF0080");
        assert_eq!(fill_colour(None, "#F0A"), "#FF00AA4D");
    }

    fn node(id: u64, x: f64, y: f64, w: f64, h: f64, children: &[u64]) -> Value {
        json!({"id": id, "absoluteBounds": {"x": x, "y": y, "width": w, "height": h}, "children": children.iter().map(|c| json!({"id": c})).collect::<Vec<_>>()})
    }

    #[test]
    fn a_guide_is_stopped_by_what_is_in_its_way() {
        let r = Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 };
        assert!(crosses((20.0, 0.0), (20.0, 50.0), r));
        assert!(!crosses((20.0, 0.0), (20.0, 10.0), r), "touching an edge is not crossing");
        assert!(!crosses((0.0, 0.0), (0.0, 50.0), r));
        assert!(crosses((0.0, 20.0), (50.0, 20.0), r));
    }

    #[test]
    fn badge_guides_end_on_the_layer_border_and_cross_no_other_layer() {
        // A button 100 x 40 at (100, 100) with an icon and a label, 16 px from the edges.
        let nodes = vec![
            node(1, 100.0, 100.0, 100.0, 40.0, &[2, 3]),
            node(2, 116.0, 112.0, 16.0, 16.0, &[]),
            node(3, 140.0, 112.0, 44.0, 16.0, &[]),
        ];
        let tree = Tree::new(&nodes, vec![0, 1, 1]);
        let root = Rect { x: 100.0, y: 100.0, w: 100.0, h: 40.0 };
        // The root is reached from the left, on its own border.
        let r = layer_target(0, &tree, root, &[], &[]).unwrap();
        assert_eq!((r.side, r.end), (Side::Left, (100.0, 120.0)));
        assert!(on_border(r.end, root));
        // The label: top and bottom tie at 12 px; the top comes first and is clear.
        let l = layer_target(2, &tree, root, &[], &[]).unwrap();
        assert_eq!((l.side, l.end), (Side::Top, (162.0, 112.0)));
        // The icon: top and bottom tie at 12 px (the left is 16); the top comes first and is clear.
        let i = layer_target(1, &tree, root, &[], &[]).unwrap();
        assert_eq!(i.side, Side::Top);
        assert_eq!(i.end, (124.0, 112.0));
        for (owner, t) in [(1, i), (2, l)] {
            for o in tree.obstacles(owner) {
                let start = match t.side {
                    Side::Top => (t.end.0, root.y),
                    Side::Bottom => (t.end.0, root.bottom()),
                    Side::Left => (root.x, t.end.1),
                    Side::Right => (root.right(), t.end.1),
                };
                assert!(!crosses(start, t.end, o));
            }
        }
    }

    #[test]
    fn a_blocked_side_gives_way_to_a_free_one() {
        // A small layer sits right above the icon: the icon's top way is blocked, the bottom is free.
        let nodes = vec![
            node(1, 0.0, 0.0, 100.0, 40.0, &[2, 3]),
            node(2, 30.0, 12.0, 16.0, 16.0, &[]),
            node(3, 30.0, 2.0, 16.0, 8.0, &[]),
        ];
        let tree = Tree::new(&nodes, vec![0, 1, 1]);
        let root = Rect { x: 0.0, y: 0.0, w: 100.0, h: 40.0 };
        let t = layer_target(1, &tree, root, &[], &[]).unwrap();
        assert_eq!(t.side, Side::Bottom);
    }

    #[test]
    fn markers_are_clear_of_the_component_and_of_each_other() {
        let ib = Rect { x: 145.0, y: 100.0, w: 70.0, h: 33.0 };
        let stage = Rect { x: 0.0, y: 0.0, w: 360.0, h: 240.0 };
        let top = |x: f64| Target { side: Side::Top, end: (x, 100.0) };
        let items = [(1, top(150.0)), (2, top(152.0)), (3, top(154.0)), (4, Target { side: Side::Left, end: (145.0, 116.0) })];
        let ms = place_markers(ib, stage, &items, &[]);
        assert_eq!(ms.len(), 4);
        for m in &ms {
            let c = Rect { x: m.centre.0 - 12.0, y: m.centre.1 - 12.0, w: 24.0, h: 24.0 };
            assert!(!c.grow(MARKER_CLEAR - 1.0).hits(ib), "marker {} is too close", m.number);
        }
        for (i, a) in ms.iter().enumerate() {
            for b in &ms[i + 1..] {
                let d = (a.centre.0 - b.centre.0).hypot(a.centre.1 - b.centre.1);
                assert!(d >= MARKER_D, "markers {} and {} overlap", a.number, b.number);
            }
        }
    }

    #[test]
    fn a_marker_in_line_with_its_guide_end_gives_a_straight_guide() {
        let ib = Rect { x: 145.0, y: 100.0, w: 70.0, h: 33.0 };
        let stage = Rect { x: 0.0, y: 0.0, w: 360.0, h: 240.0 };
        let items = [(1, Target { side: Side::Top, end: (180.0, 100.0) })];
        let ms = place_markers(ib, stage, &items, &[]);
        assert!((ms[0].centre.0 - 180.0).abs() < 0.01);
    }

    fn block_of<'a>(node: usize, nodes: &[usize]) -> Block<'a> {
        Block { node, nodes: nodes.to_vec(), lines: Vec::new(), sub: None, badge: None }
    }

    #[test]
    fn every_visible_block_is_numbered_in_order_up_to_the_cap() {
        let mut bs: Vec<Block> = (0..15).map(|i| block_of(i, &[i])).collect();
        let shown = vec![true; 15];
        let left = number_blocks(&mut bs, &shown);
        assert_eq!(bs.iter().map(|b| b.badge).collect::<Vec<_>>(), [Some(1), Some(2), Some(3), Some(4), Some(5), Some(6), Some(7), Some(8), Some(9), Some(10), Some(11), Some(12), None, None, None]);
        assert_eq!(left, 3);
        assert_eq!(tf(Lang::En, Key::Unnumbered, &[("n", "3")]), "+3 layers without a number");
        assert_eq!(tf(Lang::Pt, Key::Unnumbered, &[("n", "3")]), "+3 camadas sem número");
    }

    #[test]
    fn deep_blocks_are_numbered_and_hidden_ones_are_not() {
        // A block of two layers counts its layers when it falls past the cap; a hidden layer isn't numbered.
        let mut bs = vec![block_of(0, &[0]), block_of(1, &[1]), block_of(2, &[2])];
        assert_eq!(number_blocks(&mut bs, &[true, false, true]), 0);
        assert_eq!(bs.iter().map(|b| b.badge).collect::<Vec<_>>(), [Some(1), None, Some(2)]);
        let mut many: Vec<Block> = (0..MAX_BADGES).map(|i| block_of(i, &[i])).collect();
        many.push(block_of(MAX_BADGES, &[MAX_BADGES, MAX_BADGES + 1]));
        assert_eq!(number_blocks(&mut many, &[true; MAX_BADGES + 2]), 2);
        let walked = vec![(json!({"visible": true}), 0), (json!({"visible": false}), 1), (json!({}), 2), (json!({}), 1)];
        assert_eq!(rendered(&walked), [true, false, false, true]);
    }

    #[test]
    fn text_shows_its_content_when_the_name_differs() {
        let t = |name: &str, chars: &str| (json!({"type": "TEXT", "name": name, "characters": chars}), 2);
        let content = |walked: &[(Value, usize)], name: &str| group_content(walked, &(0..walked.len()).collect::<Vec<_>>(), name);
        let one = [t("Lorem ipsum", "Button")];
        assert_eq!(header_name("Lorem ipsum", 1, content(&one, "Lorem ipsum").as_deref()), "Lorem ipsum \"Button\"");
        assert_eq!(content(&[t("Button", "Button")], "Button"), None);
        let long = [t("Title", "A very long heading that goes on and on")];
        assert_eq!(content(&long, "Title").as_deref(), Some("\"A very long heading that…\""));
        assert_eq!(header_name("Cube", 2, None), "Cube ×2");
    }

    #[test]
    fn grouped_texts_show_quotes_only_when_the_content_is_the_same() {
        let t = |chars: &str| (json!({"type": "TEXT", "name": "Label", "characters": chars}), 2);
        let same = [t("Save"), t("Save")];
        assert_eq!(header_name("Label", 2, group_content(&same, &[0, 1], "Label").as_deref()), "Label ×2 \"Save\"");
        let differ = [t("Save"), t("Cancel")];
        assert_eq!(header_name("Label", 2, group_content(&differ, &[0, 1], "Label").as_deref()), "Label ×2");
    }

    #[test]
    fn parts_are_named_by_type_and_instances_by_their_component() {
        assert_eq!(kind_word(Lang::En, "FRAME"), "Frame");
        assert_eq!(kind_word(Lang::En, "TEXT"), "Text");
        assert_eq!(kind_word(Lang::En, "BOOLEAN_OPERATION"), "Boolean Operation");
        assert_eq!(tf(Lang::En, Key::InstanceOf, &[("name", "Icon")]), "Instance of Icon");
    }

    /// The way 0.9.5 composed an alignment (before the nine texts): the oracle for the English one.
    fn old_alignment_text(mode: &str, main: &str, cross: &str) -> String {
        let (vertical, horizontal) = if mode == "vertical" { (align_word(Lang::En, main, true), align_word(Lang::En, cross, false)) } else { (align_word(Lang::En, cross, true), align_word(Lang::En, main, false)) };
        format!("{vertical} {}", horizontal.to_lowercase())
    }

    #[test]
    fn the_english_alignment_is_what_it_was_for_every_combination() {
        let values = ["min", "center", "max", "spaceBetween", "spaceAround", "spaceEvenly", "baseline", "somethingNew"];
        let mut n = 0;
        for mode in ["horizontal", "vertical", "grid"] {
            for main in values {
                for cross in values {
                    assert_eq!(alignment_text(Lang::En, mode, main, cross), old_alignment_text(mode, main, cross), "{mode} {main} {cross}");
                    n += 1;
                }
            }
        }
        assert_eq!(n, 192);
    }

    fn counts(check: usize, vars: usize, styles: usize, total: usize) -> SummaryCounts {
        SummaryCounts { bound: 8, check, unbound: 3, missing: 0, suspect: 2, total, vars, styles }
    }

    /// The four summary texts as they were before the singular (0.9.6 up to 02613e0): the oracle for any
    /// count other than 1.
    fn old_summary(lang: Lang, c: &SummaryCounts) -> String {
        let (plain, checks, styles, both) = match lang {
            Lang::En => (
                "{bound} bound · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties",
                "{bound} bound ({check} to check) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties",
                "{bound} bound ({vars} variables, {styles} styles) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties",
                "{bound} bound ({vars} variables, {styles} styles; {check} to check) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties",
            ),
            Lang::Pt => (
                "{bound} vinculadas · {unbound} sem token · {missing} ausentes · {suspect} suspeitas · de {total} propriedades",
                "{bound} vinculadas ({check} a verificar) · {unbound} sem token · {missing} ausentes · {suspect} suspeitas · de {total} propriedades",
                "{bound} vinculadas ({vars} variáveis, {styles} estilos) · {unbound} sem token · {missing} ausentes · {suspect} suspeitas · de {total} propriedades",
                "{bound} vinculadas ({vars} variáveis, {styles} estilos; {check} a verificar) · {unbound} sem token · {missing} ausentes · {suspect} suspeitas · de {total} propriedades",
            ),
        };
        let template = match (c.check > 0, c.styles > 0) {
            (false, false) => plain,
            (true, false) => checks,
            (false, true) => styles,
            (true, true) => both,
        };
        [("bound", c.bound), ("check", c.check), ("unbound", c.unbound), ("missing", c.missing), ("suspect", c.suspect), ("total", c.total), ("vars", c.vars), ("styles", c.styles)]
            .iter()
            .fold(template.to_string(), |acc, (k, v)| acc.replace(&format!("{{{k}}}"), &v.to_string()))
    }

    #[test]
    fn the_summary_is_what_it_was_for_every_count_but_one() {
        let mut n = 0;
        for lang in [Lang::En, Lang::Pt] {
            for check in [0, 2] {
                for vars in [0, 2, 7, 35] {
                    for styles in [0, 2, 8] {
                        for total in [0, 2, 45] {
                            let c = counts(check, vars, styles, total);
                            assert_eq!(summary_text(lang, &c), old_summary(lang, &c), "{lang:?} check {check} vars {vars} styles {styles} total {total}");
                            n += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(n, 2 * 2 * 4 * 3 * 3);
    }

    #[test]
    fn the_summary_says_variable_style_and_property_in_the_singular_for_exactly_one() {
        // Each noun on its own, with the others in the plural.
        assert_eq!(summary_text(Lang::En, &counts(0, 1, 2, 45)), "8 bound (1 variable, 2 styles) · 3 no token · 0 missing · 2 suspect · of 45 properties");
        assert_eq!(summary_text(Lang::En, &counts(0, 7, 1, 45)), "8 bound (7 variables, 1 style) · 3 no token · 0 missing · 2 suspect · of 45 properties");
        assert_eq!(summary_text(Lang::En, &counts(0, 7, 2, 1)), "8 bound (7 variables, 2 styles) · 3 no token · 0 missing · 2 suspect · of 1 property");
        assert_eq!(summary_text(Lang::Pt, &counts(0, 1, 2, 45)), "8 vinculadas (1 variável, 2 estilos) · 3 sem token · 0 ausentes · 2 suspeitas · de 45 propriedades");
        assert_eq!(summary_text(Lang::Pt, &counts(0, 7, 1, 45)), "8 vinculadas (7 variáveis, 1 estilo) · 3 sem token · 0 ausentes · 2 suspeitas · de 45 propriedades");
        assert_eq!(summary_text(Lang::Pt, &counts(0, 7, 2, 1)), "8 vinculadas (7 variáveis, 2 estilos) · 3 sem token · 0 ausentes · 2 suspeitas · de 1 propriedade");
        // The three at once, with and without "to check".
        assert_eq!(summary_text(Lang::En, &counts(0, 1, 1, 1)), "8 bound (1 variable, 1 style) · 3 no token · 0 missing · 2 suspect · of 1 property");
        assert_eq!(summary_text(Lang::En, &counts(1, 1, 1, 1)), "8 bound (1 variable, 1 style; 1 to check) · 3 no token · 0 missing · 2 suspect · of 1 property");
        assert_eq!(summary_text(Lang::Pt, &counts(0, 1, 1, 1)), "8 vinculadas (1 variável, 1 estilo) · 3 sem token · 0 ausentes · 2 suspeitas · de 1 propriedade");
        assert_eq!(summary_text(Lang::Pt, &counts(1, 1, 1, 1)), "8 vinculadas (1 variável, 1 estilo; 1 a verificar) · 3 sem token · 0 ausentes · 2 suspeitas · de 1 propriedade");
        // Only the total (no styles, so no parentheses): the Icon of the demo.
        assert_eq!(summary_text(Lang::En, &counts(0, 0, 0, 1)), "8 bound · 3 no token · 0 missing · 2 suspect · of 1 property");
        assert_eq!(summary_text(Lang::Pt, &counts(0, 0, 0, 1)), "8 vinculadas · 3 sem token · 0 ausentes · 2 suspeitas · de 1 propriedade");
        assert_eq!(summary_text(Lang::En, &counts(2, 0, 0, 1)), "8 bound (2 to check) · 3 no token · 0 missing · 2 suspect · of 1 property");
        // 0 is the plural.
        assert_eq!(summary_text(Lang::En, &counts(0, 0, 1, 0)), "8 bound (0 variables, 1 style) · 3 no token · 0 missing · 2 suspect · of 0 properties");
    }

    #[test]
    fn the_nine_alignments_in_portuguese_and_a_distribution_joined_by_a_comma() {
        let pt = |mode: &str, main: &str, cross: &str| alignment_text(Lang::Pt, mode, main, cross);
        // A horizontal layout: main is the horizontal axis.
        assert_eq!(pt("horizontal", "min", "min"), "Topo à esquerda");
        assert_eq!(pt("horizontal", "center", "min"), "Topo ao centro");
        assert_eq!(pt("horizontal", "max", "min"), "Topo à direita");
        assert_eq!(pt("horizontal", "min", "center"), "Meio à esquerda");
        assert_eq!(pt("horizontal", "center", "center"), "Meio ao centro");
        assert_eq!(pt("horizontal", "max", "center"), "Meio à direita");
        assert_eq!(pt("horizontal", "min", "max"), "Base à esquerda");
        assert_eq!(pt("horizontal", "center", "max"), "Base ao centro");
        assert_eq!(pt("horizontal", "max", "max"), "Base à direita");
        // A vertical layout: main is the vertical axis.
        assert_eq!(pt("vertical", "min", "max"), "Topo à direita");
        // A distribution has no nine-way text: the words of the app, joined.
        assert_eq!(pt("horizontal", "spaceBetween", "center"), "Meio, espaço entre");
        assert_eq!(pt("vertical", "spaceEvenly", "min"), "Espaço uniforme, esquerda");
        assert_eq!(pt("horizontal", "spaceAround", "max"), "Base, espaço ao redor");
    }

    #[test]
    fn layer_kinds_in_portuguese_and_the_ones_kept_in_english() {
        assert_eq!(kind_word(Lang::Pt, "TEXT"), "Texto");
        assert_eq!(kind_word(Lang::Pt, "INSTANCE"), "Instância");
        assert_eq!(kind_word(Lang::Pt, "COMPONENT"), "Componente");
        assert_eq!(kind_word(Lang::Pt, "COMPONENT_SET"), "Conjunto de componentes");
        let nine = [("FRAME", "Quadro"), ("GROUP", "Grupo"), ("RECTANGLE", "Retângulo"), ("ELLIPSE", "Elipse"), ("VECTOR", "Vetor"), ("LINE", "Linha"), ("STAR", "Estrela"), ("POLYGON", "Polígono"), ("BOOLEAN_OPERATION", "Operação booleana")];
        for (host, pt) in nine {
            assert_eq!(kind_word(Lang::Pt, host), pt, "{host}");
        }
        // Section and Slice: English as it always was, Portuguese from the dictionary.
        assert_eq!(kind_word(Lang::En, "SLICE"), "Slice");
        assert_eq!(kind_word(Lang::En, "SECTION"), "Section");
        assert_eq!(kind_word(Lang::Pt, "SLICE"), "Fatia");
        assert_eq!(kind_word(Lang::Pt, "SECTION"), "Seção");
        // A type the dictionary doesn't know is the host's name, as before.
        for l in [Lang::En, Lang::Pt] {
            assert_eq!(kind_word(l, "TABLE_CELL"), "Table Cell");
        }
        for k in ["FRAME", "GROUP", "RECTANGLE", "ELLIPSE", "VECTOR", "LINE", "STAR", "POLYGON", "BOOLEAN_OPERATION", "COMPONENT", "COMPONENT_SET", "INSTANCE", "TEXT"] {
            let old: String = k.split('_').map(|w| w[..1].to_string() + &w[1..].to_lowercase()).collect::<Vec<_>>().join(" ");
            assert_eq!(kind_word(Lang::En, k), if k == "TEXT" { "Text".to_string() } else { old }, "{k}");
        }
    }

    #[test]
    fn the_not_a_component_error_names_the_type_in_portuguese_and_keeps_english_as_it_was() {
        let say = |lang: Lang, kind: &str| tf(lang, Key::ErrNotComponent, &[("id", "50"), ("kind", &kind_in_error(lang, kind))]);
        assert_eq!(say(Lang::En, "FRAME"), "layer 50 is a FRAME, not a component or an instance");
        assert_eq!(say(Lang::En, "SECTION"), "layer 50 is a SECTION, not a component or an instance");
        assert_eq!(say(Lang::Pt, "FRAME"), "a camada 50 é Quadro, não um componente nem uma instância");
        assert_eq!(say(Lang::Pt, "TEXT"), "a camada 50 é Texto, não um componente nem uma instância");
        // Section and Slice: raw in English, worded in Portuguese.
        assert_eq!(say(Lang::En, "SLICE"), "layer 50 is a SLICE, not a component or an instance");
        assert_eq!(say(Lang::Pt, "SECTION"), "a camada 50 é Seção, não um componente nem uma instância");
        assert_eq!(say(Lang::Pt, "SLICE"), "a camada 50 é Fatia, não um componente nem uma instância");
        // A type the dictionary doesn't know stays as the host names it.
        assert_eq!(say(Lang::Pt, "TABLE_CELL"), "a camada 50 é TABLE_CELL, não um componente nem uma instância");
    }

    #[test]
    fn blocks_include_parts_without_lines_and_skip_deeper_layers_without_lines() {
        let mk = |name: &str, part: bool, props: usize| Group {
            name: name.into(),
            kind: "FRAME".into(),
            sub: part.then(|| "Frame".to_string()),
            part,
            node: 0,
            nodes: vec![0],
            lines: (0..props).map(|_| Line { property: Prop::Fill, token: String::new(), value: "#FFFFFF".into(), state: State::Unbound, nodes: vec![0] }).collect(),
        };
        let gs = [mk("Root", true, 1), mk("Part", true, 0), mk("Deep", false, 0), mk("Deep with fill", false, 1)];
        let bs = blocks(&gs, |_| true, true);
        assert_eq!(bs.len(), 3);
        assert_eq!(bs[1].lines.len(), 0);
        assert_eq!(bs[1].sub.as_deref(), Some("Frame"));
    }

    fn boxed(x: f64, w: f64, visible: bool) -> Value {
        json!({"visible": visible, "absoluteBounds": {"x": x, "y": 0.0, "width": w, "height": 40.0}})
    }

    #[test]
    fn a_wrapper_with_the_roots_box_is_seen_through() {
        // Root > Container (same box) > Icon, Title: the parts are the container's children.
        let walked = vec![(boxed(0.0, 100.0, true), 0), (boxed(0.0, 100.0, true), 1), (boxed(10.0, 20.0, true), 2), (boxed(40.0, 20.0, true), 2), (boxed(40.0, 5.0, true), 3)];
        let drawn = rendered(&walked);
        assert_eq!(anatomy_wrapper(&walked, &drawn), Some(1));
        assert_eq!(part_flags(&walked, &drawn), [true, false, true, true, false]);
        // Two direct children, or a smaller one: no wrapper, the direct children are the parts.
        let two = vec![(boxed(0.0, 100.0, true), 0), (boxed(0.0, 100.0, true), 1), (boxed(0.0, 50.0, true), 1)];
        assert_eq!(anatomy_wrapper(&two, &rendered(&two)), None);
        assert_eq!(part_flags(&two, &rendered(&two)), [true, true, true]);
        let small = vec![(boxed(0.0, 100.0, true), 0), (boxed(0.0, 60.0, true), 1), (boxed(0.0, 10.0, true), 2)];
        assert_eq!(part_flags(&small, &rendered(&small)), [true, true, false]);
        // A hidden sibling doesn't count: the one drawn child is still the wrapper.
        let hid = vec![(boxed(0.0, 100.0, true), 0), (boxed(0.0, 100.0, true), 1), (boxed(0.0, 100.0, false), 1)];
        assert_eq!(anatomy_wrapper(&hid, &rendered(&hid)), Some(1));
    }

    #[test]
    fn hidden_layers_are_not_parts_and_have_no_block() {
        let walked = vec![(boxed(0.0, 100.0, true), 0), (boxed(0.0, 10.0, false), 1), (boxed(10.0, 10.0, true), 1), (boxed(10.0, 5.0, true), 2)];
        let drawn = rendered(&walked);
        assert_eq!(drawn, [true, false, true, true]);
        assert_eq!(part_flags(&walked, &drawn), [true, false, true, false]);
        // A layer under a hidden one is out as well.
        let nested = vec![(boxed(0.0, 100.0, true), 0), (boxed(0.0, 10.0, false), 1), (boxed(0.0, 5.0, true), 2)];
        assert_eq!(rendered(&nested), [true, false, false]);
    }

    #[test]
    fn the_root_has_no_instance_of_line_but_the_parts_do() {
        let inst = json!({"type": "INSTANCE", "mainComponentName": "Size=Small"});
        assert_eq!(sub_line(Lang::En, &inst, 0, Some("Button".into())), None);
        assert_eq!(sub_line(Lang::En, &inst, 1, Some("Icon".into())).as_deref(), Some("Instance of Icon"));
        assert_eq!(sub_line(Lang::En, &inst, 1, None).as_deref(), Some("Instance of Size=Small"));
        assert_eq!(sub_line(Lang::En, &json!({"type": "FRAME"}), 1, None).as_deref(), Some("Frame"));
    }

    #[test]
    fn up_to_twelve_badges() {
        let mut bs: Vec<Block> = (0..14).map(|i| block_of(i, &[i])).collect();
        let left = number_blocks(&mut bs, &[true; 14]);
        assert_eq!(bs.iter().filter(|b| b.badge.is_some()).count(), 12);
        assert_eq!(bs[11].badge, Some(12));
        assert_eq!(bs[12].badge, None);
        assert_eq!(left, 2);
    }

    #[test]
    fn a_spaced_wrapper_with_the_roots_box_stands_in_for_the_root() {
        let n = |x: f64, w: f64| json!({"absoluteBounds": {"x": x, "y": 0.0, "width": w, "height": 40.0}});
        let walked = vec![(n(0.0, 100.0), 0), (n(0.0, 100.0), 1), (n(10.0, 50.0), 2)];
        let row = |node| Row { property: Prop::PadH, token: String::new(), value: "24".into(), state: State::Unbound, zero: false, node, depth: 1, style: false };
        assert_eq!(spacing_wrapper(&walked, &[true; 3], &[row(1)]), Some(1));
        // Spacing deeper than the direct children, a smaller child or a hidden one: none.
        assert_eq!(spacing_wrapper(&walked, &[true; 3], &[row(2)]), None);
        let small = vec![(n(0.0, 100.0), 0), (n(0.0, 60.0), 1)];
        assert_eq!(spacing_wrapper(&small, &[true; 2], &[row(1)]), None);
        assert_eq!(spacing_wrapper(&walked, &[true, false, true], &[row(1)]), None);
    }

    #[test]
    fn coincident_layers_get_guides_on_different_sides() {
        // A label and the text inside it share top and bottom: the second takes the other side.
        let nodes = vec![node(1, 100.0, 100.0, 100.0, 40.0, &[2]), node(2, 120.0, 110.0, 60.0, 20.0, &[3]), node(3, 120.0, 110.0, 60.0, 20.0, &[])];
        let tree = Tree::new(&nodes, vec![0, 1, 2]);
        let root = Rect { x: 100.0, y: 100.0, w: 100.0, h: 40.0 };
        let a = layer_target(1, &tree, root, &[], &[]).unwrap();
        let b = layer_target(2, &tree, root, &[a.end], &[]).unwrap();
        assert_ne!(a.side, b.side);
    }

    #[test]
    fn markers_keep_eight_pixels_apart_on_every_side() {
        let ib = Rect { x: 145.0, y: 100.0, w: 70.0, h: 33.0 };
        let stage = Rect { x: 0.0, y: 0.0, w: 360.0, h: 240.0 };
        // Top and left guides close to the corner: the markers must not meet there.
        let items = [(1, Target { side: Side::Top, end: (146.0, 100.0) }), (2, Target { side: Side::Left, end: (145.0, 101.0) }), (3, Target { side: Side::Top, end: (150.0, 100.0) })];
        let ms = place_markers(ib, stage, &items, &[]);
        for (i, a) in ms.iter().enumerate() {
            for b in &ms[i + 1..] {
                let d = (a.centre.0 - b.centre.0).hypot(a.centre.1 - b.centre.1);
                assert!(d - MARKER_D >= MARKER_GAP, "markers {} and {} are {} apart", a.number, b.number, d - MARKER_D);
            }
        }
    }

    #[test]
    fn a_guide_never_ends_on_another_guide() {
        let m = |number, c: (f64, f64), end: (f64, f64)| Marker { item: 0, number, centre: c, end };
        // The second guide ends on the first one's line.
        let (kept, dropped) = drop_tangled(vec![m(1, (0.0, 50.0), (100.0, 50.0)), m(2, (50.0, 0.0), (50.0, 50.0))]);
        assert_eq!((kept.len(), dropped.len()), (1, 1));
        // A way whose guide would pass through another guide's end gives way to another side.
        let nodes = vec![node(1, 100.0, 100.0, 100.0, 40.0, &[2]), node(2, 120.0, 110.0, 20.0, 20.0, &[])];
        let tree = Tree::new(&nodes, vec![0, 1]);
        let root = Rect { x: 100.0, y: 100.0, w: 100.0, h: 40.0 };
        let t = layer_target(1, &tree, root, &[(100.0, 120.0)], &[]).unwrap();
        assert_ne!(t.side, Side::Left);
        // A side given up on comes last.
        let t = layer_target(1, &tree, root, &[], &[Side::Top]).unwrap();
        assert_ne!(t.side, Side::Top);
    }

    #[test]
    fn crossing_guides_leave_the_later_marker_out() {
        let m = |number, c: (f64, f64), end: (f64, f64)| Marker { item: 0, number, centre: c, end };
        // Two guides that cross as an X.
        let (kept, dropped) = drop_tangled(vec![m(1, (0.0, 0.0), (100.0, 100.0)), m(2, (100.0, 0.0), (0.0, 100.0))]);
        assert_eq!((kept.len(), dropped.iter().map(|m| m.number).collect::<Vec<_>>()), (1, vec![2]));
        // Parallel guides stay.
        let (kept, dropped) = drop_tangled(vec![m(1, (0.0, 0.0), (0.0, 100.0)), m(2, (40.0, 0.0), (40.0, 100.0))]);
        assert_eq!((kept.len(), dropped.len()), (2, 0));
    }

    fn groups(n: usize) -> Vec<Group> {
        (0..n).map(|i| Group { name: format!("L{i}"), kind: "FRAME".into(), sub: None, part: false, node: i, nodes: vec![i], lines: vec![Line { property: Prop::Fill, token: String::new(), value: i.to_string(), state: State::Unbound, nodes: vec![i] }] }).collect()
    }

    #[test]
    fn typography_lines_ride_along_without_spending_the_cap() {
        let line = |property| Line { property, token: String::new(), value: "v".into(), state: State::Unbound, nodes: vec![0] };
        let group = |i: usize, props: &[Prop]| Group { name: format!("L{i}"), kind: "TEXT".into(), sub: None, part: false, node: i, nodes: vec![i], lines: props.iter().map(|p| line(*p)).collect() };
        // Three layers with a fill and a typography line each, a cap of 2 fills.
        let all = vec![group(0, &[Prop::Fill, Prop::Typography]), group(1, &[Prop::Fill, Prop::Typography]), group(2, &[Prop::Fill, Prop::Typography])];
        let (shown, hidden) = limit_groups(&all, 2);
        assert_eq!(shown.iter().map(|g| g.lines.len()).collect::<Vec<_>>(), [2, 2, 0]);
        assert_eq!(hidden, 2);
        // A layer with only a typography line takes what the cap has left over.
        let all = vec![group(0, &[Prop::Fill]), group(1, &[Prop::Typography]), group(2, &[Prop::Typography])];
        let (shown, hidden) = limit_groups(&all, 2);
        assert_eq!(shown.iter().map(|g| g.lines.len()).collect::<Vec<_>>(), [1, 1, 0]);
        assert_eq!(hidden, 1);
    }

    #[test]
    fn the_audit_stops_at_the_line_limit_and_says_how_many_are_left_out() {
        let all = groups(MAX_AUDIT_LINES + 37);
        let (shown, hidden) = limit_groups(&all, MAX_AUDIT_LINES);
        assert_eq!(shown.iter().map(|g| g.lines.len()).sum::<usize>(), MAX_AUDIT_LINES);
        assert_eq!(hidden, 37);
        assert_eq!(hidden_note(Lang::En, hidden).as_deref(), Some("+37 lines not shown"));
        assert_eq!(hidden_note(Lang::Pt, hidden).as_deref(), Some("+37 linhas não exibidas"));
        // Under the limit nothing is left out and there is no note.
        let (shown, hidden) = limit_groups(&groups(3), MAX_AUDIT_LINES);
        assert_eq!((shown.iter().map(|g| g.lines.len()).sum::<usize>(), hidden), (3, 0));
        assert_eq!(hidden_note(Lang::En, 0), None);
        // Exactly the limit: nothing left out either.
        assert_eq!(limit_groups(&groups(MAX_AUDIT_LINES), MAX_AUDIT_LINES).1, 0);
    }

    #[test]
    fn alignment_is_said_like_the_design_tool() {
        // Horizontal layout: the cross axis is vertical, the main axis horizontal.
        assert_eq!(alignment_text(Lang::En, "horizontal", "center", "center"), "Middle center");
        assert_eq!(alignment_text(Lang::En, "horizontal", "min", "min"), "Top left");
        assert_eq!(alignment_text(Lang::En, "horizontal", "max", "max"), "Bottom right");
        assert_eq!(alignment_text(Lang::En, "horizontal", "spaceBetween", "center"), "Middle space between");
        // Vertical layout: the axes swap.
        assert_eq!(alignment_text(Lang::En, "vertical", "min", "center"), "Top center");
        assert_eq!(alignment_text(Lang::En, "vertical", "max", "min"), "Bottom left");
        assert_eq!(resize_word(Lang::En, "hug"), "Hug");
        assert_eq!(resize_word(Lang::En, "fixed"), "Fixed");
        assert_eq!(resize_word(Lang::En, "fill"), "Fill");
    }

    #[test]
    fn spacing_lines_use_words() {
        let node = json!({"layout": {"mode": "horizontal", "mainAlign": "center", "crossAlign": "center"}, "sizing": {"horizontal": "hug", "vertical": "fixed"}});
        let lines = layout_lines(Lang::En, &node);
        let text: Vec<String> = lines.iter().map(|(k, v)| format!("{k} : {v}")).collect();
        assert_eq!(text, ["Direction : Horizontal", "Alignment : Middle center", "Horizontal resizing : Hug", "Vertical resizing : Fixed"]);
    }

    #[test]
    fn redlines_follow_the_stage_and_the_line_colour() {
        // Light stage, default line colour: the line colour at full strength, 2 px, Prussian values and ring.
        let light = red_style(false, false, "#FCA311", "#FCA3114D");
        assert_eq!((light.guide.as_str(), light.ring), ("#FCA311", Some("#14213D")));
        assert_eq!((light.accent.as_str(), light.overlay_stroke.as_str(), light.value.as_str()), ("#FCA311", "#FCA311", "#14213D"));
        assert_eq!((light.guide_weight, light.overlay_weight, light.overlay_fill.as_str()), (2.0, 1.5, "#FCA3114D"));
        // Dark stage: the line colour everywhere, 1 px, no ring.
        let dark = red_style(true, false, "#FCA311", "#FCA3114D");
        assert_eq!((dark.guide.as_str(), dark.ring, dark.value.as_str(), dark.guide_weight), ("#FCA311", None, "#FCA311", 1.0));
        // A line colour set in the settings counts on every stage.
        let custom = red_style(false, true, "#3B82F6", "#3B82F64D");
        assert_eq!((custom.guide.as_str(), custom.ring, custom.overlay_stroke.as_str(), custom.value.as_str()), ("#3B82F6", None, "#3B82F6", "#3B82F6"));
        assert_eq!(stage_fill(false), "#E5E5E5");
        assert_eq!(stage_fill(true), "#14213D");
    }

    #[test]
    fn the_auto_stage_follows_the_background_or_else_the_elements() {
        let root = |fill: Option<&str>| json!({"name": "Root", "fills": fill.map(|f| json!([{"type": "solid", "color": f}])).unwrap_or(json!([]))});
        let text = |fill: &str| json!({"type": "TEXT", "fills": [{"type": "solid", "color": fill}]});
        // White text over a transparent root: the dark stage. Dark text: the light one.
        assert!(is_dark(StageMode::Auto, &[root(None), text("#FFFFFF")], None).0);
        assert!(!is_dark(StageMode::Auto, &[root(None), text("#171719")], None).0);
        // White text on a blue root: the blue is judged, and it reads on the light stage.
        assert!(!is_dark(StageMode::Auto, &[root(Some("#3B5BDB")), text("#FFFFFF")], None).0);
        // A transparent root takes its ancestor's fill (the set's) as the background.
        let white = contrast::parse_hex("#FFFFFF", 1.0);
        assert!(is_dark(StageMode::Auto, &[root(None), text("#171719")], white).0);
        // A hidden element doesn't count.
        assert!(!is_dark(StageMode::Auto, &[root(None), json!({"visible": false, "fills": [{"type": "solid", "color": "#FFFFFF"}]})], None).0);
        assert!(!is_dark(StageMode::Light, &[root(None), text("#FFFFFF")], None).0);
        assert!(is_dark(StageMode::Dark, &[], None).0);
    }
}
