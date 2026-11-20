//! The Layouts frame (0.11): a drawing of the component's spacing. A stage (the Anatomy's: the same width and the same colour rule) with the
//! instance of the default variant, and over it, for each layer with auto layout that has padding or a gap, translucent bands on the areas
//! of the padding (one for each side) and between the children (the gap), each with its value in px. The padding is green and the gap violet
//! (neither is the orange of the Anatomy's markers); a band that is too thin for its number sends the number out of the instance, to a
//! circle with a straight guide, placed by the Anatomy's marker placer (the same gaps of 12 px and the same ban on diagonals).
//!
//! 0.10.0 drew this with the host's `measure.padding` (a line for each side and gap of ONE frame, with its value) as a rectangle with an
//! outline over the instance and the value on a patch of the stage's colour; the layers it measured were the root or a wrapper standing in
//! for it. This version reads every layer's padding and gap from the walk of the instance and draws the bands itself, so many layers fit
//! in one drawing and the numbers do not depend on a host call for each layer.

#![warn(dead_code)] // the module above allows dead code (the 0.10.0 drawing); this one is checked

use super::section::{Ctx, USEFUL_W, header, pal, pick_stage_for, judge};
use super::tables::{base_walk, muted_note};
use super::*;

/// The most layers drawn (the component first, then the ones with the most area) and the most nodes the drawing may make (bands, numbers,
/// circles and guides; the instance is not counted).
pub const LAYER_CAP: usize = 8;
pub const NODE_CAP: usize = 120;
/// A band thinner than this in its thin direction has its number outside.
pub const THIN: f64 = 20.0;
/// The fill of the bands of the layers at the top (the component and its children) and of the deeper ones, as a percentage.
const FILL_TOP: u8 = 30;
const FILL_DEEP: u8 = 18;

/// Padding and gap colours: not the orange of the Anatomy. On the light stage they are deep, on the dark stage light.
pub const PADDING_LIGHT: &str = "#007D00";
pub const PADDING_DARK: &str = "#4ADE80";
pub const GAP_LIGHT: &str = "#6D28D9";
pub const GAP_DARK: &str = "#A78BFA";

/// A band of the drawing: an area of padding or of gap, with its value.
#[derive(Clone, Debug, PartialEq)]
pub struct Band {
    pub(super) rect: Rect,
    pub gap: bool,
    pub value: f64,
}

impl Band {
    /// The least of its width and height.
    pub(super) fn thin(&self) -> f64 {
        self.rect.w.min(self.rect.h)
    }
}

/// The auto layout of a layer as the bands need it.
#[derive(Clone, Debug, PartialEq)]
pub struct Auto {
    /// "horizontal", "vertical" or "grid" (a grid has no gap bands).
    pub mode: String,
    /// Top, right, bottom, left.
    pub padding: [f64; 4],
    pub gap: f64,
}

/// The bands of one layer: the layer's rectangle `r`, its auto layout and the rectangles of the children that are drawn. Padding: one band
/// for each side that has some. Gap: one band between two neighbouring children (in the order they stand in along the layer's direction)
/// when the gap is set and the space between them is the gap (a distribution that spreads them leaves other spaces, and those are not the
/// gap).
pub(super) fn bands_of(r: Rect, auto: &Auto, kids: &[Rect]) -> Vec<Band> {
    let [top, right, bottom, left] = auto.padding;
    let mut out = Vec::new();
    let mut pad = |rect: Rect, v: f64| {
        if v > 0.005 && rect.w > 0.0 && rect.h > 0.0 {
            out.push(Band { rect, gap: false, value: v });
        }
    };
    let inner_h = (r.h - top - bottom).max(0.0);
    pad(Rect { x: r.x, y: r.y, w: r.w, h: top }, top);
    pad(Rect { x: r.x, y: r.bottom() - bottom, w: r.w, h: bottom }, bottom);
    pad(Rect { x: r.x, y: r.y + top, w: left, h: inner_h }, left);
    pad(Rect { x: r.right() - right, y: r.y + top, w: right, h: inner_h }, right);
    if auto.gap > 0.005 && matches!(auto.mode.as_str(), "horizontal" | "vertical") {
        let horizontal = auto.mode == "horizontal";
        let mut ordered: Vec<Rect> = kids.to_vec();
        ordered.sort_by(|a, b| if horizontal { a.x.total_cmp(&b.x) } else { a.y.total_cmp(&b.y) });
        for w in ordered.windows(2) {
            let (a, b) = (w[0], w[1]);
            let (space, rect) = if horizontal {
                (b.x - a.right(), Rect { x: a.right(), y: r.y + top, w: b.x - a.right(), h: inner_h })
            } else {
                (b.y - a.bottom(), Rect { x: r.x + left, y: a.bottom(), w: (r.w - left - right).max(0.0), h: b.y - a.bottom() })
            };
            if (space - auto.gap).abs() <= 0.5 && rect.w > 0.0 && rect.h > 0.0 {
                out.push(Band { rect, gap: true, value: auto.gap });
            }
        }
    }
    out
}

/// A layer with bands: where it is in the walk, how deep, its area and its bands.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub node: usize,
    pub depth: usize,
    pub area: f64,
    pub bands: Vec<Band>,
}

/// How many nodes a layer's bands make: a rectangle for each; a number inside a band is a patch and its text; a number outside is a
/// circle, its text and its guide.
pub(super) fn layer_nodes(l: &Layer) -> usize {
    l.bands.iter().map(|b| 1 + if b.thin() < THIN { 3 } else { 2 }).sum()
}

/// The layers drawn, and how many were left out: the component (depth 0) first, then the others by area, largest first; at most
/// `LAYER_CAP` and as many as fit in `NODE_CAP` nodes (a layer goes in whole or not at all).
pub(super) fn choose_layers(mut layers: Vec<Layer>) -> (Vec<Layer>, usize) {
    layers.retain(|l| !l.bands.is_empty());
    layers.sort_by(|a, b| (a.depth != 0).cmp(&(b.depth != 0)).then(b.area.total_cmp(&a.area)).then(a.node.cmp(&b.node)));
    let total = layers.len();
    let mut kept: Vec<Layer> = Vec::new();
    let mut nodes = 0usize;
    for l in layers {
        let cost = layer_nodes(&l);
        if kept.len() >= LAYER_CAP || nodes + cost > NODE_CAP {
            continue;
        }
        nodes += cost;
        kept.push(l);
    }
    let left = total - kept.len();
    (kept, left)
}

/// The colours of the drawing on a stage: the padding's, the gap's and what the number on each is written in.
pub struct Colours {
    pub padding: &'static str,
    pub gap: &'static str,
    pub on_padding: &'static str,
    pub on_gap: &'static str,
}

pub fn colours(dark: bool) -> Colours {
    if dark {
        Colours { padding: PADDING_DARK, gap: GAP_DARK, on_padding: theme::ON_REDLINE, on_gap: theme::ON_REDLINE }
    } else {
        Colours { padding: PADDING_LIGHT, gap: GAP_LIGHT, on_padding: theme::ON_LIGHT_ACCENT, on_gap: theme::ON_LIGHT_ACCENT }
    }
}

/// The least contrast between a number and the colour behind it (checked by the tests).
#[cfg(test)]
pub const MIN_NUMBER_CONTRAST: f64 = 4.5;

/// The auto layout of a node of the walk, when it has one (horizontal, vertical or grid).
fn auto_of(node: &Value) -> Option<Auto> {
    let layout = node.get("layout")?;
    let mode = layout.get("mode").and_then(Value::as_str).filter(|m| matches!(*m, "horizontal" | "vertical" | "grid"))?;
    let pad: Vec<f64> = layout.get("padding").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
    let [top, right, bottom, left] = pad[..] else { return None };
    Some(Auto { mode: mode.to_string(), padding: [top, right, bottom, left], gap: layout.get("gap").and_then(Value::as_f64).unwrap_or(0.0) })
}

/// The layers of the walk that have auto layout, and those of them with bands; `offset` moves the walk's rectangles to where the instance
/// stands on the stage. The first number is how many layers use auto layout at all.
pub(super) fn layers_of(walked: &[(Value, usize)], drawn: &[bool], offset: (f64, f64)) -> (usize, Vec<Layer>) {
    let by_id: HashMap<u64, usize> = walked.iter().enumerate().filter_map(|(i, (n, _))| Some((n.get("id")?.as_u64()?, i))).collect();
    let rect_of = |i: usize| bounds(&walked[i].0).map(|b| Rect { x: b.x + offset.0, y: b.y + offset.1, ..b });
    let mut with_auto = 0usize;
    let mut out = Vec::new();
    for (i, (node, depth)) in walked.iter().enumerate() {
        if !drawn.get(i).copied().unwrap_or(false) {
            continue;
        }
        let Some(auto) = auto_of(node) else { continue };
        with_auto += 1;
        let Some(r) = rect_of(i) else { continue };
        let kids: Vec<Rect> = node
            .get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|c| by_id.get(&c.get("id")?.as_u64()?).copied())
            .filter(|k| drawn.get(*k).copied().unwrap_or(false))
            .filter_map(rect_of)
            .collect();
        let bands = bands_of(r, &auto, &kids);
        out.push(Layer { node: i, depth: *depth, area: r.w * r.h, bands });
    }
    (with_auto, out)
}

/// The text of a value: whole numbers without decimals.
fn value_text(v: f64) -> String {
    audit::number(v)
}

/// The Layouts frame.
pub(super) fn build_layouts(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    header(c, frame, t(lang, Key::SecLayouts))?;
    let walked = base_walk(c)?;
    let drawn = rendered(&walked);
    // The walk's rectangles are where its instance stood; the stage's instance is somewhere else (the offset is read below).
    let root = walked.first().and_then(|(n, _)| bounds(n)).unwrap_or_default();
    let (with_auto, mut all) = layers_of(&walked, &drawn, (0.0, 0.0));
    all.retain(|l| !l.bands.is_empty());
    if with_auto == 0 || all.is_empty() {
        muted_note(lang, frame, t(lang, Key::LayoutsNone))?;
        return Ok(());
    }
    let (layers, left_out) = choose_layers(all);
    // The texts of the instance (where the walk stood): a guide to a number outside does not run over one.
    let texts: Vec<Rect> = walked.iter().enumerate().filter(|(i, (n, _))| drawn.get(*i).copied().unwrap_or(false) && n.get("type").and_then(Value::as_str) == Some("TEXT")).filter_map(|(_, (n, _))| bounds(n)).collect();
    // The stage: the Anatomy's width, the instance of the default variant in the middle, and the colour the Anatomy's rule picks.
    let target = c.target;
    let size = bounds(&target.node).unwrap_or(Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 });
    let stage_w = c.useful.get();
    let h0 = (size.h + STAGE_ROOM).max(MIN_STAGE_H);
    let sb0 = Rect { x: -30_000.0, y: STAGING_Y + 3_000.0, w: stage_w, h: h0 };
    let stage = make_stage(lang, sb0, false)?;
    let (instance, ib0) = place_instance(lang, target.component, stage, sb0, size)?;
    let nodes: Vec<Value> = walked.iter().map(|(n, _)| n.clone()).collect();
    let choice = pick_stage_for(c.mode, std::slice::from_ref(&judge(&nodes)), set_backdrop(lang, target)?);
    let dark = choice.stage == contrast::Stage::Dark;
    let col = colours(dark);
    // The walk's rectangles move onto the stage's instance.
    let offset = (ib0.x - root.x, ib0.y - root.y);
    let moved: Vec<Layer> = layers
        .iter()
        .map(|l| Layer { bands: l.bands.iter().map(|b| Band { rect: Rect { x: b.rect.x + offset.0, y: b.rect.y + offset.1, ..b.rect }, ..*b }).collect(), ..l.clone() })
        .collect();
    // The numbers: inside their band when it is thick enough; the others stand outside, each with a straight guide (the Anatomy's placer).
    let mut outside: Vec<(usize, usize)> = Vec::new();
    for (li, l) in moved.iter().enumerate() {
        for (bi, b) in l.bands.iter().enumerate() {
            if b.thin() < THIN || value_text(b.value).chars().count() as f64 * CHAR_W + 2.0 * LABEL_PAD_X > b.rect.w.max(b.rect.h) {
                outside.push((li, bi));
            }
        }
    }
    let make_items = |moved: &[Layer], dy: f64| -> Vec<Item> {
        outside
            .iter()
            .enumerate()
            .map(|(k, (li, bi))| {
                let rect = moved[*li].bands[*bi].rect;
                // The texts stand in the guide's way, shrunk by the room a guide keeps from them plus a pixel: a band that touches a text's
                // edge is reached on that edge, a guide that would have to run over the text is not.
                let blockers = texts.iter().map(|t| Rect { x: t.x + offset.0, y: t.y + offset.1 + dy, ..*t }.grow(-GUIDE_CLEAR - 1.0)).filter(|t| t.w > 0.0 && t.h > 0.0).collect();
                Item { number: k + 1, root: false, layer: rect, kind: Kind::Small, inside: vec![], blockers, rims: vec![] }
            })
            .collect()
    };
    let items = make_items(&moved, 0.0);
    let big = Rect { y: ib0.y - MARKERS_ROOM_MAX, h: ib0.h + 2.0 * MARKERS_ROOM_MAX, ..sb0 };
    let room = place_markers(ib0, big, &items, &[]).iter().map(|m| (ib0.y - m.centre.1).max(m.centre.1 - ib0.bottom()).max(0.0) + MARKER_EDGE).fold(STAGE_ROOM / 2.0, f64::max);
    let h1 = h0.max(size.h + 2.0 * room);
    let (sb, ib, dy) = if (h1 - h0).abs() > 0.5 {
        let dy = (h1 - h0) / 2.0;
        host(lang, "node.set", &json!({"ids": [stage], "props": {"height": h1}}))?;
        host(lang, "node.set", &json!({"ids": [instance], "props": {"y": (h1 - size.h) / 2.0}}))?;
        (Rect { h: h1, ..sb0 }, Rect { y: ib0.y + dy, ..ib0 }, dy)
    } else {
        (sb0, ib0, 0.0)
    };
    host(lang, "node.set", &json!({"ids": [stage], "props": {"fill": choice.fill(), "stroke": theme::BORDER, "strokeWeight": 1, "strokeAlign": "inside", "width": stage_w}}))?;
    let shift = |r: Rect| Rect { y: r.y + dy, ..r };
    let moved: Vec<Layer> = moved.iter().map(|l| Layer { bands: l.bands.iter().map(|b| Band { rect: shift(b.rect), ..*b }).collect(), ..l.clone() }).collect();
    let items = make_items(&moved, dy);
    let markers = place_markers(ib, sb, &items, &[]);
    let origin = (sb.x, sb.y);
    // The bands first (the numbers go on top).
    for (li, l) in moved.iter().enumerate() {
        let percent = if l.depth <= 1 { FILL_TOP } else { FILL_DEEP };
        for b in &l.bands {
            let colour = if b.gap { col.gap } else { col.padding };
            let name = tf(lang, Key::LayerMeasure, &[("value", &value_text(b.value))]);
            host(
                lang,
                "shape.rectangle",
                &json!({"x": b.rect.x - origin.0, "y": b.rect.y - origin.1, "width": b.rect.w, "height": b.rect.h, "parent": stage, "name": name, "select": false,
                    "props": {"fill": fill_colour(percent, colour), "stroke": colour, "strokeWeight": 1.0, "strokeAlign": "inside"}}),
            )?;
        }
        let _ = li;
    }
    // The numbers inside their band: a pill of the band's colour with the number in the colour that reads on it.
    for (li, l) in moved.iter().enumerate() {
        for (bi, b) in l.bands.iter().enumerate() {
            // A number outside that the placer found no straight way for stays on its band (over its edges, but there).
            if outside.iter().position(|o| *o == (li, bi)).is_some_and(|k| markers.iter().any(|m| m.number == k + 1)) {
                continue;
            }
            let label = value_text(b.value);
            let (fill, ink) = if b.gap { (col.gap, col.on_gap) } else { (col.padding, col.on_padding) };
            let tw = CHAR_W * label.chars().count() as f64;
            let (cx, cy) = (b.rect.x + b.rect.w / 2.0, b.rect.y + b.rect.h / 2.0);
            let patch = value_label(lang, stage, origin, &label, cx - tw / 2.0, cy - LABEL_H / 2.0 + LABEL_PAD_Y, ink, fill, &tf(lang, Key::LayerValue, &[("value", &label)]))?;
            let _ = patch;
        }
    }
    // The numbers outside: a guide (straight) from the circle to the band's edge, the circle and the number.
    let mut dropped = 0usize;
    for (k, (li, bi)) in outside.iter().enumerate() {
        let b = &moved[*li].bands[*bi];
        let Some(m) = markers.iter().find(|m| m.number == k + 1) else {
            dropped += 1;
            continue;
        };
        let (fill, ink) = if b.gap { (col.gap, col.on_gap) } else { (col.padding, col.on_padding) };
        let (start, tip) = guide_of(m);
        let label = value_text(b.value);
        let name = tf(lang, Key::LayerLeader, &[("n", &label)]);
        leader(lang, stage, origin, start, tip, (fill, GUIDE_WEIGHT), &name)?;
        let circle = host(
            lang,
            "shape.ellipse",
            &json!({"x": m.centre.0 - MARKER_D / 2.0 - origin.0, "y": m.centre.1 - MARKER_D / 2.0 - origin.1, "width": MARKER_D, "height": MARKER_D, "parent": stage,
                "name": tf(lang, Key::LayerValue, &[("value", &label)]), "select": false, "props": {"fill": fill}}),
        )?;
        let _ = id_of(lang, &circle, "shape.ellipse")?;
        let tw = MARKER_CHAR_W * label.chars().count() as f64;
        text(lang, stage, &label, m.centre.0 - tw / 2.0 - origin.0, m.centre.1 - MARKER_LABEL_H / 2.0 - origin.1, MARKER_FONT, ink, None, &tf(lang, Key::LayerValue, &[("value", &label)]))?;
    }
    host(lang, "node.reparent", &json!({"ids": [stage], "parent": frame}))?;
    // The legend: one line.
    let mut lg = FlowSpec::new(t(lang, Key::LayerLegend), USEFUL_W, "horizontal");
    lg.gap = 24.0;
    lg.extra = json!({"crossAlign": "center"});
    let legend = flow_frame(lang, frame, lg)?;
    for (colour, key) in [(col.padding, Key::LayoutsLegendPadding), (col.gap, Key::LayoutsLegendGap)] {
        let item_name = tf(lang, Key::LayerGroup, &[("name", t(lang, key))]);
        let mut item = FlowSpec::new(&item_name, 20.0, "horizontal");
        item.gap = 8.0;
        item.extra = json!({"sizingH": "hug", "crossAlign": "center"});
        let item = flow_frame(lang, legend, item)?;
        host(lang, "shape.rectangle", &json!({"width": 16, "height": 16, "parent": item, "name": t(lang, Key::LayerDeltaSwatch), "select": false, "props": {"fill": colour, "cornerRadius": 3}}))?;
        text(lang, item, t(lang, key), 0.0, 0.0, FONT, pal::TEXT2, None, t(lang, key))?;
    }
    if left_out > 0 {
        muted_note(lang, frame, &tf(lang, plural(left_out, Key::LayoutsLeftOutOne, Key::LayoutsLeftOut), &[("n", &left_out.to_string())]))?;
    }
    *c.spacing.borrow_mut() = Some(json!({
        "layers": layers.len(), "left_out": left_out, "with_auto_layout": with_auto, "stage": [sb.x, sb.y, sb.w, sb.h], "dark": dark,
        "bands": moved.iter().map(|l| l.bands.len()).sum::<usize>(), "outside": outside.len(), "outside_dropped": dropped,
        "nodes": moved.iter().map(layer_nodes).sum::<usize>(),
        "list": moved.iter().flat_map(|l| l.bands.iter().map(move |b| json!({"depth": l.depth, "gap": b.gap, "value": b.value, "rect": [b.rect.x, b.rect.y, b.rect.w, b.rect.h]}))).collect::<Vec<_>>(),
        "markers": markers.iter().map(|m| json!({"n": m.number, "centre": [m.centre.0, m.centre.1], "tip": [m.tip.0, m.tip.1], "layer": [m.layer.x, m.layer.y, m.layer.w, m.layer.h]})).collect::<Vec<_>>(),
        "component": [ib.x, ib.y, ib.w, ib.h],
    }));
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn r(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    fn auto(mode: &str, padding: [f64; 4], gap: f64) -> Auto {
        Auto { mode: mode.to_string(), padding, gap }
    }

    #[test]
    fn a_row_has_a_band_for_each_side_of_padding_and_one_between_children() {
        // A 200 x 60 row at (10, 20): padding 8 / 16 / 8 / 16, gap 12, two children 40 wide at the left padding and a gap apart.
        let layer = r(10.0, 20.0, 200.0, 60.0);
        let kids = [r(26.0, 28.0, 40.0, 44.0), r(78.0, 28.0, 40.0, 44.0)];
        let bands = bands_of(layer, &auto("horizontal", [8.0, 16.0, 8.0, 16.0], 12.0), &kids);
        let by = |gap: bool| bands.iter().filter(|b| b.gap == gap).collect::<Vec<_>>();
        assert_eq!(by(false).len(), 4);
        assert_eq!(by(true).len(), 1);
        // The top band: the layer's width, 8 high, at its top; the left band: 16 wide between the top and bottom bands.
        assert!(bands.contains(&Band { rect: r(10.0, 20.0, 200.0, 8.0), gap: false, value: 8.0 }));
        assert!(bands.contains(&Band { rect: r(10.0, 28.0, 16.0, 44.0), gap: false, value: 16.0 }));
        assert!(bands.contains(&Band { rect: r(194.0, 28.0, 16.0, 44.0), gap: false, value: 16.0 }));
        assert!(bands.contains(&Band { rect: r(10.0, 72.0, 200.0, 8.0), gap: false, value: 8.0 }));
        // The gap band: from the first child's right edge to the second's left edge, over the inner height.
        assert_eq!(by(true)[0].rect, r(66.0, 28.0, 12.0, 44.0));
        assert_eq!(by(true)[0].value, 12.0);
    }

    #[test]
    fn a_column_puts_its_gap_bands_between_children_from_top_to_bottom() {
        let layer = r(0.0, 0.0, 100.0, 120.0);
        let kids = [r(10.0, 10.0, 80.0, 20.0), r(10.0, 42.0, 80.0, 20.0), r(10.0, 74.0, 80.0, 20.0)];
        let bands = bands_of(layer, &auto("vertical", [10.0, 10.0, 26.0, 10.0], 12.0), &kids);
        let gaps: Vec<Rect> = bands.iter().filter(|b| b.gap).map(|b| b.rect).collect();
        assert_eq!(gaps, vec![r(10.0, 30.0, 80.0, 12.0), r(10.0, 62.0, 80.0, 12.0)]);
        // The children in any order give the same bands.
        let shuffled = [kids[2], kids[0], kids[1]];
        assert_eq!(bands_of(layer, &auto("vertical", [10.0, 10.0, 26.0, 10.0], 12.0), &shuffled), bands);
    }

    #[test]
    fn nothing_is_drawn_for_a_zero_padding_a_zero_gap_or_a_space_that_is_not_the_gap() {
        let layer = r(0.0, 0.0, 100.0, 40.0);
        assert!(bands_of(layer, &auto("horizontal", [0.0; 4], 0.0), &[r(0.0, 0.0, 30.0, 40.0), r(40.0, 0.0, 30.0, 40.0)]).is_empty());
        // Gap 8 but the children are 30 apart (the distribution spreads them): the space is not the gap, no band.
        let spread = bands_of(layer, &auto("horizontal", [0.0; 4], 8.0), &[r(0.0, 0.0, 30.0, 40.0), r(60.0, 0.0, 30.0, 40.0)]);
        assert!(spread.is_empty(), "{spread:?}");
        // A grid has padding bands and no gap bands.
        let grid = bands_of(layer, &auto("grid", [4.0, 0.0, 0.0, 0.0], 8.0), &[r(0.0, 4.0, 30.0, 36.0), r(38.0, 4.0, 30.0, 36.0)]);
        assert_eq!(grid.len(), 1);
        assert!(!grid[0].gap);
    }

    fn layer(node: usize, depth: usize, area: f64, bands: usize) -> Layer {
        Layer { node, depth, area, bands: (0..bands).map(|i| Band { rect: r(i as f64 * 30.0, 0.0, 24.0, 24.0), gap: false, value: 8.0 }).collect() }
    }

    #[test]
    fn the_component_comes_first_then_the_largest_and_at_most_eight_layers_and_120_nodes() {
        let layers = vec![layer(3, 2, 100.0, 4), layer(0, 0, 50.0, 4), layer(1, 1, 900.0, 4), layer(2, 1, 500.0, 0)];
        let (kept, left) = choose_layers(layers);
        assert_eq!(kept.iter().map(|l| l.node).collect::<Vec<_>>(), vec![0, 1, 3], "the root first, then by area; a layer with no bands is not one");
        assert_eq!(left, 0);
        // Ten layers: eight are drawn, two are left out.
        let many: Vec<Layer> = (0..10).map(|i| layer(i, usize::from(i > 0), 100.0 - i as f64, 1)).collect();
        let (kept, left) = choose_layers(many);
        assert_eq!((kept.len(), left), (8, 2));
        assert_eq!(kept[0].node, 0);
        // Nodes: bands 3 each (rectangle, patch, text); 12 bands of 3 = 36 a layer, so 120 / 36 = 3 layers go in whole.
        let heavy: Vec<Layer> = (0..6).map(|i| layer(i, usize::from(i > 0), 100.0 - i as f64, 12)).collect();
        let (kept, left) = choose_layers(heavy);
        assert_eq!(kept.len(), 3);
        assert_eq!(left, 3);
        assert!(kept.iter().map(layer_nodes).sum::<usize>() <= NODE_CAP);
    }

    #[test]
    fn a_thin_band_costs_a_circle_a_number_and_a_guide() {
        let thin = Layer { node: 0, depth: 0, area: 1.0, bands: vec![Band { rect: r(0.0, 0.0, 100.0, 8.0), gap: false, value: 8.0 }] };
        assert_eq!(layer_nodes(&thin), 1 + 3);
        let thick = Layer { node: 0, depth: 0, area: 1.0, bands: vec![Band { rect: r(0.0, 0.0, 100.0, 24.0), gap: true, value: 8.0 }] };
        assert_eq!(layer_nodes(&thick), 1 + 2);
    }

    #[test]
    fn the_number_reads_on_its_band_colour_at_4_5_to_1_on_both_stages() {
        for dark in [false, true] {
            let c = colours(dark);
            for (fill, ink) in [(c.padding, c.on_padding), (c.gap, c.on_gap)] {
                let (a, b) = (contrast::parse_hex(fill, 1.0).unwrap(), contrast::parse_hex(ink, 1.0).unwrap());
                let ratio = contrast::ratio(a, b);
                assert!(ratio >= MIN_NUMBER_CONTRAST, "dark {dark}: {ink} on {fill} is {ratio:.2}");
            }
            // Neither band colour is the orange of the Anatomy's markers.
            assert!(![theme::LIGHT_ANATOMY, theme::DARK_ANATOMY].contains(&c.padding) && ![theme::LIGHT_ANATOMY, theme::DARK_ANATOMY].contains(&c.gap));
            assert_ne!(c.padding, c.gap);
        }
    }

    #[test]
    fn a_walk_gives_the_layers_with_auto_layout_and_the_bands_of_the_drawn_ones() {
        let node = |id: u64, x: f64, y: f64, w: f64, h: f64, layout: Value, kids: &[u64]| {
            json!({"id": id, "name": "n", "type": "FRAME", "absoluteBounds": {"x": x, "y": y, "width": w, "height": h}, "layout": layout, "children": kids.iter().map(|k| json!({"id": k})).collect::<Vec<_>>()})
        };
        let lay = |mode: &str, pad: [f64; 4], gap: f64| json!({"mode": mode, "padding": pad, "gap": gap});
        let walked = vec![
            (node(1, 0.0, 0.0, 100.0, 40.0, lay("horizontal", [4.0, 4.0, 4.0, 4.0], 8.0), &[2, 3]), 0),
            (node(2, 4.0, 4.0, 40.0, 32.0, json!({"mode": "none"}), &[]), 1),
            (node(3, 52.0, 4.0, 40.0, 32.0, lay("vertical", [0.0; 4], 0.0), &[]), 1),
        ];
        let (with_auto, layers) = layers_of(&walked, &[true, true, true], (0.0, 0.0));
        assert_eq!(with_auto, 2, "the root and the column; the node with no auto layout is not one");
        assert_eq!(layers.iter().find(|l| l.node == 0).map(|l| l.bands.len()), Some(5), "four padding bands and one gap");
        assert!(layers.iter().find(|l| l.node == 2).is_some_and(|l| l.bands.is_empty()), "a column with no padding or gap has no band");
        // A hidden layer is not drawn.
        let (with_auto, layers) = layers_of(&walked, &[true, true, false], (0.0, 0.0));
        assert_eq!(with_auto, 1);
        assert_eq!(layers.len(), 1);
        // The rectangles move with the offset.
        let (_, moved) = layers_of(&walked, &[true, true, true], (100.0, 50.0));
        assert_eq!(moved[0].bands[0].rect.x, layers[0].bands[0].rect.x + 100.0);
    }
}
