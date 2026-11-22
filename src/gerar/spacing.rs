//! The Layouts frame (0.11): drawings of the component's spacing, ONE DRAWING FOR EACH LEVEL of nesting. A drawing is a stage (the
//! Anatomy's: the same width and the same colour rule) with the instance of the default variant and, over it, translucent bands on the
//! areas of the padding (one for each side) and between the children (the gap) of the layers of ONE level only, so the bands of a
//! container and of the container inside it are never stacked. The padding is green and the gap violet (neither is the orange of the
//! Anatomy's markers). A component with one level makes one drawing; with more, each has a title ("Level 2 · names") and there are at most
//! four. Bands of the same kind and value in the same level that are siblings share ONE number (the value, nothing else). A number of a
//! single band stands inside it (the band is empty space, and the patch is at least 4 px short of its edges all round) or outside the
//! instance, in a circle or a pill with a straight guide placed by the Anatomy's marker placer. A number of two or more bands stands at
//! the margin of the stage with a SPINE: a vertical line beside the instance, and from it a short horizontal trace to each band. No guide,
//! spine or trace crosses a text, a vector or an instance with nothing inside, nor another one; a band with no clean way to the spine has
//! no trace and the frame says so in a note. Values are the component's real px. The drawing is not scaled (see the report of prompt 59:
//! the host cannot scale an instance whole).
//!
//! 0.10.0 drew this with the host's `measure.padding` (a line for each side and gap of ONE frame, with its value) as a rectangle with an
//! outline over the instance and the value on a patch of the stage's colour; the layers it measured were the root or a wrapper standing in
//! for it. This version reads every layer's padding and gap from the walk of the instance and draws the bands itself, so many layers fit
//! in one drawing and the numbers do not depend on a host call for each layer.

#![warn(dead_code)] // the module above allows dead code (the 0.10.0 drawing); this one is checked

use super::section::{Ctx, StageChoice, USEFUL_W, header, pal, pick_stage_for, judge};
use super::tables::{base_walk, muted_note};
use super::*;

/// The most layers drawn (the component first, then the ones with the most area) and the most nodes the drawing may make (bands, numbers,
/// circles and guides; the instance is not counted).
pub const LAYER_CAP: usize = 8;
pub const NODE_CAP: usize = 120;
/// The size of a drawing's title.
const TITLE_SIZE: f64 = 18.0;
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
    /// What makes it one of a row of repeated layers: its parent in the walk and its name (the rows of a list). None for the component.
    pub family: Option<(usize, String)>,
    pub depth: usize,
    pub area: f64,
    pub bands: Vec<Band>,
}

/// One number for a set of bands: the bands of the same kind (padding or gap) and the same value in ONE layer ("the two gaps of 8 of
/// the list"), or in the layers of a row of repeated siblings (same parent, same name: the three rows of a list, each with its gap of 8
/// between the icon and the text). `members` are (layer, band) pairs in reading order (top to bottom, then left to right): the first is
/// where the guide goes. A layer with another parent or another name is never joined.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub gap: bool,
    pub value: f64,
    pub members: Vec<(usize, usize)>,
}

impl Group {
    /// The number as written: the value, nothing else (the spine shows which bands it is for).
    pub fn label(&self) -> String {
        value_text(self.value)
    }
}

/// The groups of the layers' bands, in the order of their first bands (reading order).
pub(super) fn groups_of(layers: &[Layer]) -> Vec<Group> {
    let mut order: Vec<(usize, usize)> = layers.iter().enumerate().flat_map(|(li, l)| (0..l.bands.len()).map(move |bi| (li, bi))).collect();
    let rect = |m: &(usize, usize)| layers[m.0].bands[m.1].rect;
    order.sort_by(|a, b| rect(a).y.total_cmp(&rect(b).y).then(rect(a).x.total_cmp(&rect(b).x)));
    let mut out: Vec<(Group, usize)> = Vec::new();
    for m in order {
        let b = &layers[m.0].bands[m.1];
        let joins = |g: &(Group, usize)| {
            let first = &layers[g.1];
            g.0.gap == b.gap && (g.0.value - b.value).abs() < 0.005 && (g.1 == m.0 || (first.family.is_some() && first.family == layers[m.0].family))
        };
        match out.iter_mut().find(|g| joins(g)) {
            Some(g) => g.0.members.push(m),
            None => out.push((Group { gap: b.gap, value: b.value, members: vec![m] }, m.0)),
        }
    }
    out.into_iter().map(|(g, _)| g).collect()
}

/// How many nodes a layer's drawing makes: a rectangle for each band, and for each number (a group of bands) at most a guide, a circle
/// and its text.
pub(super) fn layer_nodes(l: &Layer) -> usize {
    l.bands.len() + 3 * groups_of(std::slice::from_ref(l)).len()
}

/// The size of the patch a number inside a band is written on (the text and its padding).
pub(super) fn patch_size(label: &str) -> (f64, f64) {
    (label.chars().count() as f64 * CHAR_W + 2.0 * LABEL_PAD_X, LABEL_H)
}

/// The room around a number written inside a band: the band is empty space, and the number must not reach its edge (the content of the
/// next layer starts there).
pub const INSIDE_ROOM: f64 = 4.0;

/// Whether the number fits inside the band: the band is at least as thick as the patch is high, plus 4 px, and as long as the patch is
/// wide, plus 4 px.
pub(super) fn fits_inside(band: Rect, label: &str) -> bool {
    let (w, h) = patch_size(label);
    band.w >= w + INSIDE_ROOM && band.h >= h + INSIDE_ROOM
}

/// How much wider than a circle a number's marker outside is, on each side: a circle holds two digits; longer ("8 (x3)") makes a pill.
pub(super) fn pill_extra(label: &str) -> f64 {
    ((label.chars().count() as f64 * MARKER_CHAR_W + 12.0 - MARKER_D) / 2.0).max(0.0)
}

/// What an instance holds that a number or a guide must not cover: its texts, its vectors and its instances with nothing inside (an
/// icon). The component itself is not one. The rectangles are the walk's.
pub(super) fn content_of(walked: &[(Value, usize)], drawn: &[bool]) -> Vec<Rect> {
    walked
        .iter()
        .enumerate()
        .filter(|(i, (_, depth))| *depth >= 1 && drawn.get(*i).copied().unwrap_or(false))
        .filter(|(_, (n, _))| {
            let kind = n.get("type").and_then(Value::as_str).unwrap_or_default();
            let leaf = n.get("children").and_then(Value::as_array).is_none_or(Vec::is_empty);
            matches!(kind, "TEXT" | "VECTOR" | "ELLIPSE" | "STAR" | "POLYGON" | "LINE" | "BOOLEAN_OPERATION") || (kind == "INSTANCE" && leaf)
        })
        .filter_map(|(_, (n, _))| bounds(n))
        .filter(|b| b.w > 0.0 && b.h > 0.0)
        .collect()
}

/// The room a guide keeps from a content box, in px: the guide is 2 px thick, so it must stand clear of the box by at least that
/// (the report of prompt 60: a guide 1.3 px from a check mark looked as if it touched it).
pub const CONTENT_CLEAR: f64 = 2.0;

/// A content rectangle as an obstacle for a guide: shrunk by the room the placer then grows it by (less `CONTENT_CLEAR`), so the guide is
/// kept `CONTENT_CLEAR` off the content's own box and a band that touches that box is still reached; never below 2 px (an icon is still
/// an obstacle).
pub(super) fn obstacle(t: Rect) -> Rect {
    let shrink = (GUIDE_CLEAR - CONTENT_CLEAR).min((t.w.min(t.h) - 2.0) / 2.0).max(0.0);
    t.grow(-shrink)
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

/// The most drawings (levels of nesting) a component gets, and the most nodes all its drawings may make together: the bands, numbers,
/// guides and the copy of the instance each drawing carries (`walk size` nodes). A level that does not fit whole is not shown.
pub const LEVEL_CAP: usize = 4;
pub const TOTAL_NODE_CAP: usize = 1000;

/// A level of nesting: the layers with bands at one depth of the instance's tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    pub depth: usize,
    pub layers: Vec<Layer>,
}

/// The levels of the layers' bands: one for each depth that has a layer with a band, from the component down. The level's number is its
/// place in this list, not its depth.
pub(super) fn levels_of(mut layers: Vec<Layer>) -> Vec<Level> {
    layers.retain(|l| !l.bands.is_empty());
    layers.sort_by_key(|l| (l.depth, l.node));
    let mut out: Vec<Level> = Vec::new();
    for l in layers {
        match out.last_mut() {
            Some(last) if last.depth == l.depth => last.layers.push(l),
            _ => out.push(Level { depth: l.depth, layers: vec![l] }),
        }
    }
    out
}

/// The levels shown (at most `LEVEL_CAP`, and as many as fit in `budget` nodes, each with the `instance` nodes of its copy of the
/// instance; the first always) and whether some were left out.
pub(super) fn shown_levels(levels: Vec<Level>, instance: usize, budget: usize) -> (Vec<Level>, bool) {
    let total = levels.len();
    let mut spent = 0usize;
    let mut kept = Vec::new();
    for lvl in levels {
        let cost = lvl.layers.iter().map(layer_nodes).sum::<usize>().min(NODE_CAP) + instance;
        if kept.len() >= LEVEL_CAP || (!kept.is_empty() && spent + cost > budget) {
            break;
        }
        spent += cost;
        kept.push(lvl);
    }
    let left = kept.len() < total;
    (kept, left)
}

/// The title of a level's drawing: its number and the names of its layers (the first three, then "…").
pub(super) fn level_names(names: &[String]) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for n in names {
        if !seen.contains(&n.as_str()) {
            seen.push(n);
        }
    }
    let shown = seen.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    if seen.len() > 3 { format!("{shown}…") } else { shown }
}

type Seg = ((f64, f64), (f64, f64));

/// Where the first spine stands from the instance's edge, and how far apart the next ones are (a spine is beside the instance, not far).
pub const SPINE_OFF: f64 = 10.0;
pub const SPINE_STEP: f64 = 8.0;
pub const SPINE_SLOTS: usize = 3;
/// How far a trace goes into its band (or to its middle when the band is thinner), and how long the stub from the spine to its number is.
pub const TRACE_INSET: f64 = 6.0;
pub const STUB: f64 = 12.0;
/// How far a trace keeps from a content box: the first choice, and the least (the guide is 2 px thick).
const TRACE_CLEAR: [f64; 2] = [3.0, CONTENT_CLEAR];

/// What a spine must keep clear of: the content (texts, vectors, icons), and the guides and numbers already drawn.
#[derive(Clone, Debug, Default)]
pub(super) struct Board {
    pub content: Vec<Rect>,
    pub segs: Vec<Seg>,
    pub boxes: Vec<Rect>,
}

impl Board {
    /// Whether the segment `s` is free: it crosses no content (but the content a band lies mostly under, which a trace has to enter), no
    /// guide and no number, and none of them is within 3 px of its ends.
    fn free(&self, s: Seg, clear: f64, band: Option<Rect>) -> bool {
        !self.content.iter().any(|t| !band.is_some_and(|b| mostly_under(b, *t)) && crosses(s.0, s.1, t.grow(clear + 0.5)))
            && !self.segs.iter().any(|o| {
                // Far apart (the boxes of the two, each grown by 3 px, do not meet): nothing can touch.
                if s.0.0.max(s.1.0) + 3.0 < o.0.0.min(o.1.0) || o.0.0.max(o.1.0) + 3.0 < s.0.0.min(s.1.0) || s.0.1.max(s.1.1) + 3.0 < o.0.1.min(o.1.1) || o.0.1.max(o.1.1) + 3.0 < s.0.1.min(s.1.1) {
                    return false;
                }
                segments_cross(s.0, s.1, o.0, o.1)
                    || point_to_segment(o.0, s.0, s.1) < 3.0
                    || point_to_segment(o.1, s.0, s.1) < 3.0
                    || point_to_segment(s.0, o.0, o.1) < 3.0
                    || point_to_segment(s.1, o.0, o.1) < 3.0
            })
            && !self.boxes.iter().any(|b| crosses(s.0, s.1, b.grow(CONTENT_CLEAR + 0.5)))
    }

    /// Takes a spine's lines and number off the board (to place it again with more bands).
    fn remove(&mut self, s: &Spine) {
        let mine: Vec<Seg> = [s.line, s.stub].into_iter().chain(s.traces.iter().map(|(_, t)| *t)).collect();
        self.segs.retain(|o| !mine.contains(o));
        self.boxes.retain(|b| *b != s.badge);
    }

    /// Whether a number's box `r` is free: off the content (4 px), 12 px from the other numbers and crossed by no guide.
    fn free_box(&self, r: Rect) -> bool {
        !self.content.iter().any(|t| r.grow(GUIDE_CLEAR).hits(*t))
            && !self.boxes.iter().any(|b| r.grow(MARKER_GAP / 2.0).hits(b.grow(MARKER_GAP / 2.0)))
            && !self.segs.iter().any(|s| crosses(s.0, s.1, r.grow(CONTENT_CLEAR + 0.5)))
    }
}

/// The bands of a group of two or more, and how wide its number's marker is beyond a circle.
#[derive(Clone, Debug)]
pub(super) struct SpineReq {
    /// The side the spine must stand on (`Some(true)` is the left); None: whichever reaches the most bands.
    pub side: Option<bool>,
    pub extra: f64,
    pub bands: Vec<Rect>,
}

/// A spine placed: its line, the stub to its number, the number's box and a trace to each band it reached (the band's place in the
/// request and the segment).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Spine {
    pub left: bool,
    pub line: Seg,
    pub stub: Seg,
    pub badge: Rect,
    pub traces: Vec<(usize, Seg)>,
}

/// The ys a trace may run at in a band, the middle first, then outwards 3 px at a time, kept 1.5 px from the band's edges.
fn trace_ys(b: Rect) -> Vec<f64> {
    let mid = b.y + b.h / 2.0;
    let room = (b.h / 2.0 - 1.5).max(0.0);
    let mut out = vec![mid];
    let mut d = 3.0;
    while d <= room + 1e-9 {
        out.push(mid - d);
        out.push(mid + d);
        d += 3.0;
    }
    out
}

fn try_spine(ib: Rect, stage: Rect, req: &SpineReq, board: &Board, left: bool, slot: usize) -> Option<Spine> {
    let x = if left { ib.x - SPINE_OFF - slot as f64 * SPINE_STEP } else { ib.right() + SPINE_OFF + slot as f64 * SPINE_STEP };
    let mut traces: Vec<(usize, Seg)> = Vec::new();
    for (mi, b) in req.bands.iter().enumerate() {
        let tip = if left { b.x + (b.w / 2.0).min(TRACE_INSET) } else { b.right() - (b.w / 2.0).min(TRACE_INSET) };
        if (left && tip <= x) || (!left && tip >= x) {
            continue;
        }
        let ys = trace_ys(*b);
        let found = TRACE_CLEAR.iter().find_map(|clear| {
            ys.iter().find_map(|y| {
                let s: Seg = ((x, *y), (tip, *y));
                (board.free(s, *clear, Some(*b)) && !traces.iter().any(|(_, o)| (o.0.1 - y).abs() < 6.0)).then_some(s)
            })
        });
        if let Some(s) = found {
            traces.push((mi, s));
        }
    }
    if traces.is_empty() {
        return None;
    }
    let y0 = traces.iter().map(|(_, s)| s.0.1).fold(f64::INFINITY, f64::min);
    let y1 = traces.iter().map(|(_, s)| s.0.1).fold(f64::NEG_INFINITY, f64::max);
    let half = MARKER_D / 2.0 + req.extra;
    let cx = if left { x - STUB - half } else { x + STUB + half };
    if cx - half < stage.x + MARKER_EDGE - MARKER_D / 2.0 || cx + half > stage.right() - (MARKER_EDGE - MARKER_D / 2.0) {
        return None;
    }
    // The number stands at the middle of the spine, or as near as it can (a line from the spine out to it, whatever its place).
    let mid = (y0 + y1) / 2.0;
    let (lo, hi) = ((y0 - 60.0).max(stage.y + MARKER_EDGE), (y1 + 60.0).min(stage.bottom() - MARKER_EDGE));
    let mut ys: Vec<f64> = (0..=40).flat_map(|j| [mid - 6.0 * j as f64, mid + 6.0 * j as f64]).filter(|y| *y >= lo && *y <= hi).collect();
    ys.dedup();
    ys.into_iter().find_map(|yb| {
        let line: Seg = ((x, y0.min(yb)), (x, y1.max(yb)));
        let near = if left { x - STUB } else { x + STUB };
        let stub: Seg = ((x, yb), (near, yb));
        let badge = Rect { x: cx - half, y: yb - MARKER_D / 2.0, w: 2.0 * half, h: MARKER_D };
        (board.free(line, CONTENT_CLEAR, None) && board.free(stub, CONTENT_CLEAR, None) && board.free_box(badge)).then(|| Spine { left, line, stub, badge, traces: traces.clone() })
    })
}

/// Places the spines of the groups of two or more bands, each on the side, and at the distance from the instance, that reaches the most of
/// its bands; a spine and what it holds are then part of the board the next ones keep clear of.
pub(super) fn place_spines(ib: Rect, stage: Rect, reqs: &[SpineReq], board: &mut Board) -> Vec<Option<Spine>> {
    reqs.iter()
        .map(|req| {
            let mut best: Option<Spine> = None;
            'sides: for left in [true, false].into_iter().filter(|l| req.side.is_none_or(|s| s == *l)) {
                for slot in 0..SPINE_SLOTS {
                    let Some(s) = try_spine(ib, stage, req, board, left, slot) else { continue };
                    let all = s.traces.len() == req.bands.len();
                    if best.as_ref().is_none_or(|b| s.traces.len() > b.traces.len()) {
                        best = Some(s);
                    }
                    if all {
                        break 'sides;
                    }
                }
            }
            if let Some(s) = &best {
                board.segs.push(s.line);
                board.segs.push(s.stub);
                board.segs.extend(s.traces.iter().map(|(_, t)| *t));
                board.boxes.push(s.badge);
            }
            best
        })
        .collect()
}

/// A group of two or more bands to give spines: the marker's extra width and the bands (the instance's `ib` tells the sides).
#[derive(Clone, Debug)]
pub(super) struct SpineGroup {
    pub extra: f64,
    pub bands: Vec<Rect>,
}

/// A spine placed for a group: which of the group's bands (by place in `SpineGroup::bands`) it has a trace to, in the spine's order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PlacedSpine {
    pub spine: Spine,
    pub ids: Vec<usize>,
}

/// The side (true: left) each band of a group is given: by the band's centre in the left or right half of the instance; a band that
/// reaches across the middle (a gap between rows, a padding above or below) goes to the side that has more of the group's bands, and on a
/// tie to the left.
pub(super) fn sides_of(ib: Rect, bands: &[Rect]) -> Vec<bool> {
    let mid = ib.x + ib.w / 2.0;
    let across = |b: &Rect| b.x <= mid && b.right() >= mid;
    let left = bands.iter().filter(|b| !across(b) && b.x + b.w / 2.0 < mid).count();
    let right = bands.iter().filter(|b| !across(b) && b.x + b.w / 2.0 >= mid).count();
    bands.iter().map(|b| if across(b) { left >= right } else { b.x + b.w / 2.0 < mid }).collect()
}

/// Places the spines of one group: one for each side that has bands, each with its own number; a band with no way to the spine of its own
/// side tries the spine of the other, and then a spine of its own. Returns the spines and the bands (by place) with no trace at all.
pub(super) fn place_group(ib: Rect, stage: Rect, g: &SpineGroup, board: &mut Board) -> (Vec<PlacedSpine>, Vec<usize>) {
    let mut sides = sides_of(ib, &g.bands);
    // A band that the content alone keeps from a side (whatever else is drawn) is given the other side; one that it keeps from both has
    // no trace and is not tried again and again below.
    let content_only = Board { content: board.content.clone(), ..Board::default() };
    let reach = |b: &Rect, left: bool| {
        let x = if left { ib.x - SPINE_OFF } else { ib.right() + SPINE_OFF };
        let tip = if left { b.x + (b.w / 2.0).min(TRACE_INSET) } else { b.right() - (b.w / 2.0).min(TRACE_INSET) };
        trace_ys(*b).iter().any(|y| content_only.free(((x, *y), (tip, *y)), CONTENT_CLEAR, Some(*b)))
    };
    let mut lost: Vec<usize> = Vec::new();
    for (i, (band, side)) in g.bands.iter().zip(sides.iter_mut()).enumerate() {
        if !reach(band, *side) {
            if reach(band, !*side) {
                *side = !*side;
            } else {
                lost.push(i);
            }
        }
    }
    let place = |ids: &[usize], left: bool, board: &mut Board| -> Option<PlacedSpine> {
        let req = SpineReq { side: Some(left), extra: g.extra, bands: ids.iter().map(|i| g.bands[*i]).collect() };
        let s = place_spines(ib, stage, &[req], board).into_iter().next().flatten()?;
        let ids = s.traces.iter().map(|(k, _)| ids[*k]).collect();
        Some(PlacedSpine { spine: s, ids })
    };
    let mut placed: Vec<PlacedSpine> = Vec::new();
    for left in [true, false] {
        let own: Vec<usize> = (0..g.bands.len()).filter(|i| sides[*i] == left && !lost.contains(i)).collect();
        if !own.is_empty() {
            placed.extend(place(&own, left, board));
        }
    }
    // The bands left with no trace go to the spine of the other side.
    for left in [true, false] {
        let stuck: Vec<usize> = (0..g.bands.len()).filter(|i| sides[*i] != left && !lost.contains(i) && !placed.iter().any(|p| p.ids.contains(i))).collect();
        if stuck.is_empty() {
            continue;
        }
        match placed.iter().position(|p| p.spine.left == left) {
            Some(k) => {
                let old = placed[k].clone();
                board.remove(&old.spine);
                let ids: Vec<usize> = old.ids.iter().chain(stuck.iter()).copied().collect();
                match place(&ids, left, board) {
                    Some(n) if n.ids.len() > old.ids.len() => placed[k] = n,
                    _ => {
                        board.segs.push(old.spine.line);
                        board.segs.push(old.spine.stub);
                        board.segs.extend(old.spine.traces.iter().map(|(_, t)| *t));
                        board.boxes.push(old.spine.badge);
                    }
                }
            }
            None => placed.extend(place(&stuck, left, board)),
        }
    }
    // A band the spines of both sides cannot reach (a spine across it would have to cross another) gets a spine of its own, on its side or
    // the other: one trace and a number of the same value.
    for (i, &own) in sides.iter().enumerate().take(g.bands.len()) {
        if lost.contains(&i) || placed.iter().any(|p| p.ids.contains(&i)) {
            continue;
        }
        if let Some(p) = place(&[i], own, board).or_else(|| place(&[i], !own, board)) {
            placed.push(p);
        }
    }
    let none = (0..g.bands.len()).filter(|i| !placed.iter().any(|p| p.ids.contains(i))).collect();
    (placed, none)
}

/// Places the spines of every group on `base` (the board with what is already drawn). A spine placed first may take the way of one placed
/// later, so the groups that were left with bands with no trace go first in the next try (while that changes the order), up to three more; the try that leaves the
/// fewest bands with no trace is the one used. The result is in the order of `groups`.
pub(super) fn place_all_groups(ib: Rect, stage: Rect, groups: &[SpineGroup], base: &Board) -> Vec<(Vec<PlacedSpine>, Vec<usize>)> {
    let run = |order: &[usize]| -> Vec<(Vec<PlacedSpine>, Vec<usize>)> {
        let mut board = base.clone();
        let mut out: Vec<(Vec<PlacedSpine>, Vec<usize>)> = vec![(Vec::new(), Vec::new()); groups.len()];
        for k in order {
            out[*k] = place_group(ib, stage, &groups[*k], &mut board);
        }
        out
    };
    let lost = |r: &[(Vec<PlacedSpine>, Vec<usize>)]| r.iter().map(|(_, n)| n.len()).sum::<usize>();
    let mut order: Vec<usize> = (0..groups.len()).collect();
    let mut best = run(&order);
    for _ in 0..3 {
        if lost(&best) == 0 {
            break;
        }
        let first: Vec<usize> = order.iter().copied().filter(|k| !best[*k].1.is_empty()).collect();
        let next: Vec<usize> = first.iter().copied().chain(order.iter().copied().filter(|k| best[*k].1.is_empty())).collect();
        if next == order {
            break;
        }
        order = next;
        let tried = run(&order);
        if lost(&tried) < lost(&best) {
            best = tried;
        } else {
            break;
        }
    }
    best
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
    let parent_of: HashMap<usize, usize> = walked
        .iter()
        .enumerate()
        .flat_map(|(p, (n, _))| n.get("children").and_then(Value::as_array).into_iter().flatten().filter_map(|c| by_id.get(&c.get("id")?.as_u64()?).map(|k| (*k, p))).collect::<Vec<_>>())
        .collect();
    let name_of = |n: &Value| n.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
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
        out.push(Layer { node: i, family: parent_of.get(&i).map(|p| (*p, name_of(node))), depth: *depth, area: r.w * r.h, bands });
    }
    (with_auto, out)
}

/// The text of a value: whole numbers without decimals.
fn value_text(v: f64) -> String {
    audit::number(v)
}

/// Places the numbers that stand outside their bands (`avoid`: what no marker may cover, the content that spills out of the instance too). `items(choice)` makes the items for the first band of each group (`choice[k]` is
/// which band of group `k` its guide goes to); a number with no place is tried again at the group's next band, up to four more times, and
/// the try that places the most is the one used. Returns the markers and the band each group ended on.
pub(super) fn place_outside(ib: Rect, stage: Rect, groups: usize, spare: &[usize], avoid: &[Rect], items: &dyn Fn(&[usize]) -> Vec<Item>) -> (Vec<Marker>, Vec<usize>) {
    let mut choice = vec![0usize; groups];
    let mut best = place_markers(ib, stage, &items(&choice), avoid);
    for _ in 0..4 {
        let mut next = choice.clone();
        let mut changed = false;
        for k in (0..groups).filter(|k| !best.iter().any(|m| m.number == k + 1)) {
            if next[k] < spare[k] {
                next[k] += 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
        let tried = place_markers(ib, stage, &items(&next), avoid);
        if tried.len() <= best.len() {
            break;
        }
        best = tried;
        choice = next;
    }
    (best, choice)
}

/// What one level's drawing made, for the answer and the frame's notes.
struct Drawn {
    json: Value,
    /// Bands with no number, as (value, how many).
    dropped: Vec<(f64, usize)>,
    groups: usize,
    inside: usize,
    outside: usize,
    spines: usize,
    nodes: usize,
    left_out: usize,
}

/// Adds `n` bands of `value` to the list of bands that had no number.
fn add_dropped(list: &mut Vec<(f64, usize)>, value: f64, n: usize) {
    if n == 0 {
        return;
    }
    match list.iter_mut().find(|(v, _)| (v - value).abs() < 0.005) {
        Some(e) => e.1 += n,
        None => list.push((value, n)),
    }
}

/// The Layouts frame. Returns how many bands got no number (they are not drawn; the frame says which).
pub(super) fn build_layouts(c: &Ctx, frame: u64) -> Result<usize> {
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
        return Ok(0);
    }
    let name_of = |node: usize| walked[node].0.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let levels = levels_of(all);
    let total_levels = levels.len();
    let (shown, deeper) = shown_levels(levels, walked.len(), TOTAL_NODE_CAP);
    let mut picked = None;
    let mut results: Vec<Drawn> = Vec::new();
    for (index, level) in shown.iter().enumerate() {
        let title = (total_levels > 1).then(|| {
            let names: Vec<String> = level.layers.iter().map(|l| name_of(l.node)).collect();
            tf(lang, Key::LayoutsLevel, &[("n", &(index + 1).to_string()), ("names", &level_names(&names))])
        });
        let (layers, left_out) = choose_layers(level.layers.clone());
        results.push(draw_level(c, frame, &walked, &drawn, root, &layers, left_out, title, &mut picked)?);
    }
    let dark = picked.as_ref().is_some_and(|(_, dark)| *dark);
    let col = colours(dark);
    // The legend: one line, for every drawing.
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
    let left_out: usize = results.iter().map(|r| r.left_out).sum();
    if left_out > 0 {
        muted_note(lang, frame, &tf(lang, plural(left_out, Key::LayoutsLeftOutOne, Key::LayoutsLeftOut), &[("n", &left_out.to_string())]))?;
    }
    if deeper {
        muted_note(lang, frame, t(lang, Key::LayoutsDeeper))?;
    }
    let mut dropped: Vec<(f64, usize)> = Vec::new();
    for r in &results {
        for (v, n) in &r.dropped {
            add_dropped(&mut dropped, *v, *n);
        }
    }
    for (v, n) in &dropped {
        let value = value_text(*v);
        let line = if *n == 1 { tf(lang, Key::LayoutsNoMarkerOne, &[("value", &value)]) } else { tf(lang, Key::LayoutsNoMarker, &[("n", &n.to_string()), ("value", &value)]) };
        muted_note(lang, frame, &line)?;
    }
    let dropped_bands: usize = dropped.iter().map(|(_, n)| n).sum();
    *c.spacing.borrow_mut() = Some(json!({
        "levels": total_levels, "drawings": results.len(), "deeper": deeper, "left_out": left_out, "with_auto_layout": with_auto,
        "groups": results.iter().map(|r| r.groups).sum::<usize>(), "inside": results.iter().map(|r| r.inside).sum::<usize>(),
        "outside": results.iter().map(|r| r.outside).sum::<usize>(), "spines": results.iter().map(|r| r.spines).sum::<usize>(),
        "outside_dropped": dropped_bands, "dropped": dropped.iter().map(|(v, n)| json!([v, n])).collect::<Vec<_>>(),
        "nodes": results.iter().map(|r| r.nodes).sum::<usize>(), "dark": dark,
        "list": results.iter().map(|r| r.json.clone()).collect::<Vec<_>>(),
    }));
    Ok(dropped_bands)
}

/// One level's drawing: a stage with the instance, the bands of `layers` (all at one depth) and their numbers.
#[allow(clippy::too_many_arguments)]
fn draw_level(c: &Ctx, frame: u64, walked: &[(Value, usize)], drawn: &[bool], root: Rect, layers: &[Layer], left_out: usize, title: Option<String>, picked: &mut Option<(StageChoice, bool)>) -> Result<Drawn> {
    let lang = c.lang;
    // The stage: the Anatomy's width, the instance of the default variant in the middle, and the colour the Anatomy's rule picks.
    let target = c.target;
    let size = bounds(&target.node).unwrap_or(Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 });
    let stage_w = c.useful.get();
    let h0 = (size.h + STAGE_ROOM).max(MIN_STAGE_H);
    let sb0 = Rect { x: -30_000.0, y: STAGING_Y + 3_000.0, w: stage_w, h: h0 };
    let stage = make_stage(lang, sb0, false)?;
    let (instance, ib0) = place_instance(lang, target.component, stage, sb0, size)?;
    let (choice, dark) = match *picked {
        Some(p) => p,
        None => {
            let nodes: Vec<Value> = walked.iter().map(|(n, _)| n.clone()).collect();
            let choice = pick_stage_for(c.mode, std::slice::from_ref(&judge(&nodes)), set_backdrop(lang, target)?);
            let p = (choice, choice.stage == contrast::Stage::Dark);
            *picked = Some(p);
            p
        }
    };
    let col = colours(dark);
    // The walk's rectangles move onto the stage's instance.
    let offset = (ib0.x - root.x, ib0.y - root.y);
    let to_stage = |r: Rect, dy: f64| Rect { x: r.x + offset.0, y: r.y + offset.1 + dy, ..r };
    let mut moved: Vec<Layer> = layers.iter().map(|l| Layer { bands: l.bands.iter().map(|b| Band { rect: to_stage(b.rect, 0.0), ..*b }).collect(), ..l.clone() }).collect();
    let content0: Vec<Rect> = content_of(walked, drawn).into_iter().map(|r| to_stage(r, 0.0)).collect();
    // The numbers: one for each group of bands. A group of one band: inside the band when it is big enough for the number and has no
    // content under the patch (the band is empty space), else outside, with a straight guide (the Anatomy's placer). A group of two or
    // more: a number at the margin with a spine and a trace to each band.
    let groups: Vec<Group> = groups_of(&moved);
    let patch_at = |b: Rect, label: &str| {
        let (w, h) = patch_size(label);
        Rect { x: b.x + (b.w - w) / 2.0, y: b.y + (b.h - h) / 2.0, w, h }
    };
    let inside: Vec<Option<(usize, usize)>> = groups
        .iter()
        .map(|g| {
            let label = g.label();
            let single = g.members.len() == 1;
            g.members.iter().copied().find(|(li, bi)| {
                let b = moved[*li].bands[*bi].rect;
                single && fits_inside(b, &label) && !content0.iter().any(|t| patch_at(b, &label).hits(*t))
            })
        })
        .collect();
    let outside: Vec<usize> = (0..groups.len()).filter(|gi| inside[*gi].is_none() && groups[*gi].members.len() == 1).collect();
    let spined: Vec<usize> = (0..groups.len()).filter(|gi| groups[*gi].members.len() > 1).collect();
    let spare = vec![0usize; outside.len()];
    let items_for = |moved: &[Layer], dy: f64| {
        let shifted: Vec<Rect> = content0.iter().map(|t| Rect { y: t.y + dy, ..*t }).collect();
        let outside = outside.clone();
        let groups = groups.clone();
        let moved = moved.to_vec();
        move |choice: &[usize]| -> Vec<Item> {
            outside
                .iter()
                .enumerate()
                .map(|(k, gi)| {
                    let g = &groups[*gi];
                    let (li, bi) = g.members[choice[k]];
                    let rect = moved[li].bands[bi].rect;
                    // A content box that most of the band lies under (a big image instance the band is drawn on) cannot be avoided: the
                    // guide has to enter it, as the Anatomy's does for a layer on a wide text's box.
                    let blockers = shifted.iter().filter(|t| !mostly_under(rect, **t)).map(|t| obstacle(*t)).collect();
                    Item { number: k + 1, root: false, layer: rect, kind: Kind::Small, inside: vec![], blockers, rims: vec![], extra: pill_extra(&g.label()) }
                })
                .collect()
        }
    };
    let big = Rect { y: ib0.y - MARKERS_ROOM_MAX, h: ib0.h + 2.0 * MARKERS_ROOM_MAX, ..sb0 };
    let (first, _) = place_outside(ib0, big, outside.len(), &spare, &content0, &items_for(&moved, 0.0));
    let room = first.iter().map(|m| (ib0.y - m.centre.1).max(m.centre.1 - ib0.bottom()).max(0.0) + MARKER_EDGE).fold(STAGE_ROOM / 2.0, f64::max);
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
    for l in &mut moved {
        for b in &mut l.bands {
            b.rect.y += dy;
        }
    }
    let content: Vec<Rect> = content0.iter().map(|t| Rect { y: t.y + dy, ..*t }).collect();
    let (markers, _) = place_outside(ib, sb, outside.len(), &spare, &content, &items_for(&moved, dy));
    // The spines go in after the single guides: they keep clear of those, of the numbers and of the content.
    let mut board = Board { content: content.clone(), ..Board::default() };
    for m in &markers {
        let (a, b) = guide_of(m);
        board.segs.push((a, b));
        let w = MARKER_D + 2.0 * m.extra;
        board.boxes.push(Rect { x: m.centre.0 - w / 2.0, y: m.centre.1 - MARKER_D / 2.0, w, h: MARKER_D });
    }
    let spine_groups: Vec<SpineGroup> = spined
        .iter()
        .map(|gi| SpineGroup { extra: pill_extra(&groups[*gi].label()), bands: groups[*gi].members.iter().map(|(li, bi)| moved[*li].bands[*bi].rect).collect() })
        .collect();
    let placed = place_all_groups(ib, sb, &spine_groups, &board);
    let origin = (sb.x, sb.y);
    // The bands first (the numbers go on top).
    for l in &moved {
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
    }
    let mut badges: Vec<Value> = Vec::new();
    let mut guides: Vec<Value> = Vec::new();
    let mut dropped: Vec<(f64, usize)> = Vec::new();
    // The numbers inside their band: a patch of the band's colour with the number in the colour that reads on it.
    for (gi, g) in groups.iter().enumerate() {
        let Some((li, bi)) = inside[gi] else { continue };
        let label = g.label();
        let (fill, ink) = if g.gap { (col.gap, col.on_gap) } else { (col.padding, col.on_padding) };
        let at = patch_at(moved[li].bands[bi].rect, &label);
        value_label(lang, stage, origin, &label, at.x + LABEL_PAD_X, at.y + LABEL_PAD_Y, ink, fill, &tf(lang, Key::LayerValue, &[("value", &label)]))?;
        badges.push(json!({"label": label, "inside": true, "rect": [at.x, at.y, at.w, at.h]}));
    }
    // A marker (a circle, or a pill for a longer text) and its text.
    let marker = |centre: (f64, f64), extra: f64, label: &str, fill: &str, ink: &str| -> Result<Rect> {
        let width = MARKER_D + 2.0 * extra;
        let shape = host(
            lang,
            "shape.rectangle",
            &json!({"x": centre.0 - width / 2.0 - origin.0, "y": centre.1 - MARKER_D / 2.0 - origin.1, "width": width, "height": MARKER_D, "parent": stage,
                "name": tf(lang, Key::LayerValue, &[("value", label)]), "select": false, "props": {"fill": fill, "cornerRadius": MARKER_D / 2.0}}),
        )?;
        let _ = id_of(lang, &shape, "shape.rectangle")?;
        let tw = MARKER_CHAR_W * label.chars().count() as f64;
        text(lang, stage, label, centre.0 - tw / 2.0 - origin.0, centre.1 - MARKER_LABEL_H / 2.0 - origin.1, MARKER_FONT, ink, None, &tf(lang, Key::LayerValue, &[("value", label)]))?;
        Ok(Rect { x: centre.0 - width / 2.0, y: centre.1 - MARKER_D / 2.0, w: width, h: MARKER_D })
    };
    // The numbers outside of one band: a straight guide from the marker to the band's edge, then the marker. A number the placer found no
    // straight way for is not drawn (never over the instance): the frame says so.
    for (k, gi) in outside.iter().enumerate() {
        let g = &groups[*gi];
        let Some(m) = markers.iter().find(|m| m.number == k + 1) else {
            add_dropped(&mut dropped, g.value, 1);
            continue;
        };
        let (fill, ink) = if g.gap { (col.gap, col.on_gap) } else { (col.padding, col.on_padding) };
        let label = g.label();
        let (start, tip) = guide_of(m);
        leader(lang, stage, origin, start, tip, (fill, GUIDE_WEIGHT), &tf(lang, Key::LayerLeader, &[("n", &label)]))?;
        let r = marker(m.centre, m.extra, &label, fill, ink)?;
        badges.push(json!({"label": label, "inside": false, "rect": [r.x, r.y, r.w, r.h]}));
        guides.push(json!([start.0, start.1, tip.0, tip.1]));
    }
    // The numbers of two or more bands: the marker at the margin, its stub, the spine and a trace to each band that has a way. A band with
    // no trace is counted, and a group none of whose bands has one has no marker at all.
    let mut spine_json: Vec<Value> = Vec::new();
    for (k, gi) in spined.iter().enumerate() {
        let g = &groups[*gi];
        let (spines, none) = &placed[k];
        add_dropped(&mut dropped, g.value, none.len());
        let (fill, ink) = if g.gap { (col.gap, col.on_gap) } else { (col.padding, col.on_padding) };
        let label = g.label();
        let name = tf(lang, Key::LayerLeader, &[("n", &label)]);
        for ps in spines {
            let s = &ps.spine;
            for seg in [s.line, s.stub].into_iter().chain(s.traces.iter().map(|(_, t)| *t)) {
                leader(lang, stage, origin, seg.0, seg.1, (fill, GUIDE_WEIGHT), &name)?;
                guides.push(json!([seg.0.0, seg.0.1, seg.1.0, seg.1.1]));
            }
            let centre = (s.badge.x + s.badge.w / 2.0, s.badge.y + s.badge.h / 2.0);
            marker(centre, (s.badge.w - MARKER_D) / 2.0, &label, fill, ink)?;
            badges.push(json!({"label": label, "inside": false, "rect": [s.badge.x, s.badge.y, s.badge.w, s.badge.h]}));
            spine_json.push(json!({"label": label, "left": s.left, "bands": g.members.len(), "traces": s.traces.len(), "line": [s.line.0.0, s.line.0.1, s.line.1.0, s.line.1.1]}));
        }
    }
    if let Some(title) = &title {
        text(lang, frame, title, 0.0, 0.0, TITLE_SIZE, pal::TEXT, None, title)?;
    }
    host(lang, "node.reparent", &json!({"ids": [stage], "parent": frame}))?;
    let nodes = moved.iter().map(layer_nodes).sum::<usize>();
    let json = json!({
        "title": title, "depth": layers.first().map(|l| l.depth), "layers": layers.len(), "left_out": left_out, "stage": [sb.x, sb.y, sb.w, sb.h],
        "bands": moved.iter().map(|l| l.bands.len()).sum::<usize>(), "groups": groups.len(), "inside": inside.iter().flatten().count(), "outside": outside.len(),
        "spines": spine_json, "nodes": nodes,
        "list": moved.iter().flat_map(|l| l.bands.iter().map(move |b| json!({"layer": l.node, "depth": l.depth, "gap": b.gap, "value": b.value, "rect": [b.rect.x, b.rect.y, b.rect.w, b.rect.h]}))).collect::<Vec<_>>(),
        "markers": markers.iter().map(|m| json!({"n": m.number, "centre": [m.centre.0, m.centre.1], "tip": [m.tip.0, m.tip.1], "layer": [m.layer.x, m.layer.y, m.layer.w, m.layer.h]})).collect::<Vec<_>>(),
        "badges": badges, "guides": guides, "content": content.iter().map(|r| json!([r.x, r.y, r.w, r.h])).collect::<Vec<_>>(),
        "component": [ib.x, ib.y, ib.w, ib.h],
    });
    Ok(Drawn { json, dropped, groups: groups.len(), inside: inside.iter().flatten().count(), outside: outside.len(), spines: spine_json.len(), nodes, left_out })
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
        Layer { node, family: None, depth, area, bands: (0..bands).map(|i| Band { rect: r(i as f64 * 30.0, 0.0, 24.0, 24.0), gap: false, value: 8.0 + i as f64 }).collect() }
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
        // Nodes: a rectangle for each band and up to three for each number (12 bands of 12 values = 12 + 36 = 48 a layer), so 120 / 48 = 2
        // layers go in whole.
        let heavy: Vec<Layer> = (0..6).map(|i| layer(i, usize::from(i > 0), 100.0 - i as f64, 12)).collect();
        let (kept, left) = choose_layers(heavy);
        assert_eq!(kept.len(), 2);
        assert_eq!(left, 4);
        assert!(kept.iter().map(layer_nodes).sum::<usize>() <= NODE_CAP);
    }

    fn band(x: f64, y: f64, w: f64, h: f64, gap: bool, value: f64) -> Band {
        Band { rect: r(x, y, w, h), gap, value }
    }

    fn layer_with(node: usize, family: Option<(usize, &str)>, bands: Vec<Band>) -> Layer {
        Layer { node, family: family.map(|(p, n)| (p, n.to_string())), depth: 1, area: 1.0, bands }
    }

    #[test]
    fn bands_of_the_same_kind_and_value_in_a_layer_share_one_number() {
        // The list: three gaps of 8 (one number "8 (x3)", at the first from the top); a padding of 8 too is another kind: its own number.
        let bands = vec![
            band(0.0, 60.0, 100.0, 8.0, true, 8.0),
            band(0.0, 20.0, 100.0, 8.0, true, 8.0),
            band(0.0, 100.0, 100.0, 8.0, true, 8.0),
            band(0.0, 0.0, 100.0, 8.0, false, 8.0),
            band(0.0, 140.0, 100.0, 16.0, true, 16.0),
        ];
        let groups = groups_of(&[layer_with(3, Some((0, "List")), bands)]);
        assert_eq!(groups.len(), 3);
        let gaps8 = groups.iter().find(|g| g.gap && (g.value - 8.0).abs() < 0.01).unwrap();
        assert_eq!(gaps8.members, vec![(0, 1), (0, 0), (0, 2)], "reading order: top to bottom");
        assert_eq!(gaps8.label(), "8", "the number is the value alone: the spine shows how many bands it is for");
        assert_eq!(gaps8.members.len(), 3);
        let pad8 = groups.iter().find(|g| !g.gap).unwrap();
        assert_eq!((pad8.members.len(), pad8.label().as_str()), (1, "8"), "a padding is not a gap, even with the same value");
        assert_eq!(groups.iter().find(|g| g.value > 10.0).unwrap().label(), "16");
    }

    #[test]
    fn the_four_paddings_of_a_card_are_one_number() {
        let bands = vec![band(0.0, 0.0, 320.0, 24.0, false, 24.0), band(0.0, 342.0, 320.0, 24.0, false, 24.0), band(0.0, 24.0, 24.0, 318.0, false, 24.0), band(296.0, 24.0, 24.0, 318.0, false, 24.0)];
        let groups = groups_of(&[layer_with(0, None, bands)]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].label(), "24");
        assert_eq!(groups[0].members.len(), 4);
        assert_eq!(groups[0].members[0], (0, 0), "the guide goes to the top band");
    }

    #[test]
    fn the_rows_of_a_list_share_one_number_for_the_gap_between_their_icon_and_text() {
        // Three rows (siblings, same name), each with a gap of 8 between the icon and the text: "8 (x3)". The list's own gaps of 8
        // between the rows are in another layer (the rows' parent): another number.
        let row = |node: usize, y: f64| layer_with(node, Some((1, "Row")), vec![band(10.0, y, 8.0, 20.0, true, 8.0)]);
        let list = layer_with(1, Some((0, "List")), vec![band(0.0, 30.0, 100.0, 8.0, true, 8.0), band(0.0, 58.0, 100.0, 8.0, true, 8.0)]);
        let groups = groups_of(&[list, row(2, 20.0), row(3, 48.0), row(4, 76.0)]);
        let mut labels: Vec<String> = groups.iter().map(Group::label).collect();
        labels.sort();
        assert_eq!(labels, vec!["8", "8"]);
        // The guide goes to the first row from the top.
        let rows = groups.iter().find(|g| g.members.len() == 3).unwrap();
        assert_eq!(rows.members[0], (1, 0));
    }

    #[test]
    fn layers_that_are_not_repeated_siblings_never_share_a_number() {
        // A header and a footer, both with padding 16 but with other names; and two rows of different parents.
        let band16 = |y: f64| vec![band(0.0, y, 100.0, 16.0, false, 16.0)];
        let groups = groups_of(&[layer_with(1, Some((0, "Header")), band16(0.0)), layer_with(2, Some((0, "Footer")), band16(100.0)), layer_with(3, Some((5, "Row")), band16(200.0)), layer_with(4, Some((6, "Row")), band16(300.0))]);
        assert_eq!(groups.len(), 4);
        assert!(groups.iter().all(|g| g.members.len() == 1));
        // The component has no family: it is its own.
        let root = layer_with(0, None, band16(0.0));
        assert_eq!(groups_of(&[root.clone(), root]).len(), 2);
    }

    #[test]
    fn a_number_goes_inside_a_band_only_when_its_patch_is_four_short_of_every_edge() {
        // "20" is 17 + 8 = 25 wide and 18 high: the band needs 29 x 22.
        assert_eq!(patch_size("20"), (25.0, 18.0));
        assert!(fits_inside(r(0.0, 0.0, 320.0, 24.0), "24"), "a band 24 thick holds the number");
        assert!(!fits_inside(r(0.0, 0.0, 320.0, 21.0), "24"), "21 is less than 18 + 4");
        assert!(fits_inside(r(0.0, 0.0, 320.0, 22.0), "24"));
        // A band 20 wide (the Popovers' side padding) is thick enough for the height but the patch is 25 wide: outside.
        assert!(!fits_inside(r(0.0, 0.0, 20.0, 99.0), "20"));
        assert!(!fits_inside(r(0.0, 0.0, 28.0, 99.0), "20"));
        assert!(fits_inside(r(0.0, 0.0, 29.0, 99.0), "20"));
        // A longer text needs a longer band.
        assert!(!fits_inside(r(0.0, 0.0, 60.0, 24.0), "8 (x3)"));
        assert!(fits_inside(r(0.0, 0.0, 100.0, 24.0), "8 (x3)"));
    }

    #[test]
    fn a_marker_is_a_circle_for_two_digits_and_a_pill_for_a_longer_text() {
        assert_eq!(pill_extra("8"), 0.0);
        assert_eq!(pill_extra("20"), 0.0);
        assert!(pill_extra("8 (x3)") > 15.0);
        // The pill holds its text: the width it makes is at least the text and 6 px each side.
        let label = "24 (x4)";
        assert!(MARKER_D + 2.0 * pill_extra(label) >= MARKER_CHAR_W * label.chars().count() as f64 + 12.0 - 0.001);
    }

    #[test]
    fn content_is_what_a_number_or_a_guide_must_not_cover() {
        let node = |id: u64, kind: &str, x: f64, y: f64, w: f64, h: f64, kids: &[u64]| json!({"id": id, "type": kind, "absoluteBounds": {"x": x, "y": y, "width": w, "height": h}, "children": kids.iter().map(|k| json!({"id": k})).collect::<Vec<_>>()});
        let walked = vec![
            (node(1, "COMPONENT", 0.0, 0.0, 100.0, 40.0, &[2, 3, 4, 5, 6]), 0),
            (node(2, "TEXT", 10.0, 10.0, 40.0, 14.0, &[]), 1),
            (node(3, "FRAME", 0.0, 0.0, 100.0, 40.0, &[]), 1),
            (node(4, "INSTANCE", 60.0, 10.0, 16.0, 16.0, &[]), 1),
            (node(5, "INSTANCE", 80.0, 0.0, 20.0, 40.0, &[7]), 1),
            (node(6, "VECTOR", 90.0, 10.0, 8.0, 8.0, &[]), 1),
            (node(7, "TEXT", 82.0, 10.0, 10.0, 10.0, &[]), 2),
        ];
        let content = content_of(&walked, &[true; 7]);
        assert_eq!(content, vec![r(10.0, 10.0, 40.0, 14.0), r(60.0, 10.0, 16.0, 16.0), r(90.0, 10.0, 8.0, 8.0), r(82.0, 10.0, 10.0, 10.0)], "a text, an icon instance, a vector and the text in the button; not the frames, the component or an instance with something in it");
        // A hidden layer is not content.
        assert_eq!(content_of(&walked, &[true, false, true, true, true, true, true]).len(), 3);
    }

    #[test]
    fn an_obstacle_is_the_content_a_little_smaller_and_never_under_two_pixels() {
        assert_eq!(obstacle(r(0.0, 0.0, 100.0, 40.0)), r(2.0, 2.0, 96.0, 36.0), "the placer grows it by 4: the guide stands 2 px off the box");
        let icon = obstacle(r(0.0, 0.0, 14.0, 9.0));
        assert!(icon.w >= 2.0 - 0.001 && icon.h >= 2.0 - 0.001, "{icon:?}");
        assert!(icon.x >= 0.0 && icon.right() <= 14.0 + 0.001, "it stays inside the icon");
    }

    fn band_item(number: usize, layer: Rect, blockers: Vec<Rect>, extra: f64) -> Item {
        Item { number, root: false, layer, kind: Kind::Small, inside: vec![], blockers, rims: vec![], extra }
    }

    /// A guide as the placer returns it, whether it runs over any of the rectangles (a hair smaller, so that touching is fine).
    fn crosses_any(m: &Marker, over: &[Rect]) -> bool {
        let (a, b) = guide_of(m);
        over.iter().any(|t| {
            let t = t.grow(-0.5);
            let (x0, x1, y0, y1) = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
            x0 < t.right() && t.x < x1 && y0 < t.bottom() && t.y < y1 || (x0 == x1 && x0 > t.x && x0 < t.right() && y0 < t.bottom() && t.y < y1) || (y0 == y1 && y0 > t.y && y0 < t.bottom() && x0 < t.right() && t.x < x1)
        })
    }

    #[test]
    fn a_guide_runs_through_empty_space_to_a_band_between_two_lines_of_text() {
        // An instance 200 x 200 at (100, 100): two lines of text (full width), a gap band between them. A guide from above or below would
        // cross a line; one from the side runs along the band.
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 400.0, 400.0);
        let texts = [r(110.0, 120.0, 180.0, 40.0), r(110.0, 176.0, 180.0, 40.0)];
        let gap = r(110.0, 160.0, 180.0, 16.0);
        let blockers: Vec<Rect> = texts.iter().map(|t| obstacle(*t)).collect();
        let (ms, _) = place_outside(ib, stage, 1, &[0], &[], &|_| vec![band_item(1, gap, blockers.clone(), 0.0)]);
        assert_eq!(ms.len(), 1, "a straight way in from the side exists");
        let (a, b) = guide_of(&ms[0]);
        assert!(a.0 == b.0 || a.1 == b.1, "straight");
        assert!(!crosses_any(&ms[0], &texts), "the guide runs over no text: {a:?} {b:?}");
        assert!((ms[0].centre.1 - 168.0).abs() < 8.5, "the guide stays in the band's rows: {:?}", ms[0].centre);
    }

    #[test]
    fn a_number_with_no_clean_way_in_is_left_out_and_never_forced_over_content() {
        // The band is boxed in by text on every side: no straight guide gets in, so the placer returns no marker for it.
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 400.0, 400.0);
        let boxed = r(190.0, 190.0, 20.0, 20.0);
        let walls = [r(150.0, 150.0, 100.0, 40.0), r(150.0, 210.0, 100.0, 40.0), r(150.0, 150.0, 40.0, 100.0), r(210.0, 150.0, 40.0, 100.0)];
        let blockers: Vec<Rect> = walls.iter().map(|t| obstacle(*t)).collect();
        let (ms, _) = place_outside(ib, stage, 1, &[0], &[], &|_| vec![band_item(1, boxed, blockers.clone(), 0.0)]);
        assert!(ms.is_empty(), "{:?}", ms.iter().map(|m| m.centre).collect::<Vec<_>>());
    }

    #[test]
    fn a_group_tries_its_next_band_when_the_first_has_no_way_in() {
        // Two bands of one group: the first is boxed in; the second is open to the left. The number goes to the second.
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 400.0, 400.0);
        let boxed = r(190.0, 190.0, 20.0, 20.0);
        let open = r(110.0, 110.0, 180.0, 8.0);
        let walls = [r(150.0, 150.0, 100.0, 40.0), r(150.0, 210.0, 100.0, 40.0), r(150.0, 150.0, 40.0, 100.0), r(210.0, 150.0, 40.0, 100.0)];
        let blockers: Vec<Rect> = walls.iter().map(|t| obstacle(*t)).collect();
        let bands = [boxed, open];
        let (ms, choice) = place_outside(ib, stage, 1, &[1], &[], &|c| vec![band_item(1, bands[c[0]], blockers.clone(), 0.0)]);
        assert_eq!(ms.len(), 1);
        assert_eq!(choice, vec![1]);
        assert!(!crosses_any(&ms[0], &walls));
    }

    #[test]
    fn a_pill_beside_the_instance_stands_as_far_from_it_as_a_circle_does() {
        let ib = r(200.0, 100.0, 100.0, 100.0);
        let stage = r(0.0, 0.0, 500.0, 300.0);
        let extra = pill_extra("12 (x2)");
        for band in [r(200.0, 150.0, 100.0, 8.0), r(210.0, 100.0, 8.0, 100.0)] {
            let (ms, _) = place_outside(ib, stage, 1, &[0], &[], &|_| vec![band_item(1, band, vec![], extra)]);
            let m = &ms[0];
            let (left, right) = (m.centre.0 - MARKER_D / 2.0 - extra, m.centre.0 + MARKER_D / 2.0 + extra);
            let clear_of = if m.centre.0 < ib.x { ib.x - right } else if m.centre.0 > ib.right() { left - ib.right() } else { f64::INFINITY };
            assert!(clear_of >= MARKER_CLEAR - 0.01, "the pill is {clear_of} from the instance: {:?}", m.centre);
        }
    }

    #[test]
    fn a_marker_stays_off_content_that_spills_out_of_the_instance() {
        // A narrow instance whose text is wider than it: the text sticks out to the left, so a marker on the left would sit on it.
        let ib = r(100.0, 100.0, 16.0, 200.0);
        let stage = r(0.0, 0.0, 400.0, 400.0);
        let spill = r(60.0, 150.0, 56.0, 17.0);
        let band = r(100.0, 100.0, 16.0, 8.0);
        let (ms, _) = place_outside(ib, stage, 1, &[0], &[spill], &|_| vec![band_item(1, band, vec![], 0.0)]);
        assert_eq!(ms.len(), 1);
        let m = &ms[0];
        let circle = r(m.centre.0 - MARKER_D / 2.0, m.centre.1 - MARKER_D / 2.0, MARKER_D, MARKER_D);
        assert!(!circle.hits(spill), "the marker {:?} sits on the text that spills out", m.centre);
    }

    #[test]
    fn a_pill_keeps_clear_of_a_circle_next_to_it() {
        // Two bands side by side at the same height on the left of the instance: the pill takes more room, so the circle is farther.
        let ib = r(200.0, 100.0, 100.0, 100.0);
        let stage = r(0.0, 0.0, 500.0, 300.0);
        let (ms, _) = place_outside(ib, stage, 2, &[0, 0], &[], &|_| vec![band_item(1, r(200.0, 120.0, 100.0, 8.0), vec![], pill_extra("8 (x3)")), band_item(2, r(200.0, 140.0, 100.0, 8.0), vec![], 0.0)]);
        assert_eq!(ms.len(), 2);
        let (a, b) = (&ms[0], &ms[1]);
        let d = (a.centre.0 - b.centre.0).hypot(a.centre.1 - b.centre.1);
        assert!(d >= MARKER_STEP + a.extra - 0.01, "{d} apart, pill extra {}", a.extra);
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

    fn seg_hits(s: Seg, t: Rect) -> bool {
        crosses(s.0, s.1, t)
    }

    /// An instance 200 x 200 at (100, 100) with four lines of text (full width, 22 tall) and a gap band of 8 between each two of them.
    fn rows() -> (Rect, Rect, Vec<Rect>, Vec<Rect>) {
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 500.0, 400.0);
        let texts: Vec<Rect> = (0..4).map(|i| r(110.0, 108.0 + 30.0 * i as f64, 180.0, 22.0)).collect();
        let bands: Vec<Rect> = (0..3).map(|i| r(110.0, 130.0 + 30.0 * i as f64, 180.0, 8.0)).collect();
        (ib, stage, texts, bands)
    }

    #[test]
    fn a_group_of_bands_gets_one_number_a_spine_and_a_trace_to_each_band() {
        let (ib, stage, texts, bands) = rows();
        let mut board = Board { content: texts.clone(), ..Board::default() };
        let spines = place_spines(ib, stage, &[SpineReq { side: None, extra: 0.0, bands: bands.clone() }], &mut board);
        let s = spines[0].as_ref().unwrap();
        assert_eq!(s.traces.len(), 3, "a trace to each of the three bands");
        assert!(s.left, "the left side is the first tried");
        assert_eq!(s.line.0.0, s.line.1.0, "the spine is vertical");
        assert!(s.line.0.0 < ib.x && ib.x - s.line.0.0 <= SPINE_OFF + 0.01, "close beside the instance");
        for (i, (mi, t)) in s.traces.iter().enumerate() {
            assert_eq!(t.0.1, t.1.1, "a trace is horizontal");
            assert_eq!(t.0.0, s.line.0.0, "it starts on the spine");
            assert!(bands[*mi].y < t.0.1 && t.0.1 < bands[*mi].bottom() && t.1.0 > bands[*mi].x, "and ends inside its band: {i}");
            assert!(!texts.iter().any(|x| seg_hits(*t, *x)), "no trace crosses a line of text");
        }
        assert!(!texts.iter().any(|x| seg_hits(s.line, *x)) && !texts.iter().any(|x| s.badge.hits(*x)));
        assert!(s.badge.x >= stage.x && s.badge.right() <= stage.right(), "the number stands in the margin of the stage");
    }

    #[test]
    fn a_band_with_no_clean_way_has_no_trace_and_a_group_with_none_has_no_spine() {
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 500.0, 400.0);
        // A left padding and a band in the middle of a row, with a text on each side of it that fills its height: the middle one has no way.
        let left_pad = r(100.0, 100.0, 10.0, 200.0);
        let middle = r(140.0, 150.0, 20.0, 20.0);
        let texts = vec![r(110.0, 148.0, 30.0, 24.0), r(160.0, 148.0, 30.0, 24.0)];
        let mut board = Board { content: texts.clone(), ..Board::default() };
        let spines = place_spines(ib, stage, &[SpineReq { side: None, extra: 0.0, bands: vec![left_pad, middle] }], &mut board);
        let s = spines[0].as_ref().unwrap();
        assert_eq!(s.traces.len(), 1, "only the padding is reached");
        assert_eq!(s.traces[0].0, 0);
        // Only the boxed-in band: no spine at all.
        let mut board = Board { content: texts, ..Board::default() };
        let spines = place_spines(ib, stage, &[SpineReq { side: None, extra: 0.0, bands: vec![middle] }], &mut board);
        assert!(spines[0].is_none());
    }

    #[test]
    fn a_spine_keeps_clear_of_the_ones_before_it_and_crosses_no_guide() {
        let (ib, stage, texts, bands) = rows();
        let mut board = Board { content: texts, ..Board::default() };
        let reqs = [SpineReq { side: None, extra: 0.0, bands: bands.clone() }, SpineReq { side: None, extra: 0.0, bands: vec![bands[0], bands[2]] }];
        let spines = place_spines(ib, stage, &reqs, &mut board);
        let (a, b) = (spines[0].as_ref().unwrap(), spines[1].as_ref().unwrap());
        let all = |s: &Spine| std::iter::once(s.line).chain(std::iter::once(s.stub)).chain(s.traces.iter().map(|(_, t)| *t)).collect::<Vec<_>>();
        for p in all(a) {
            for q in all(b) {
                assert!(!segments_cross(p.0, p.1, q.0, q.1), "{p:?} crosses {q:?}");
            }
        }
        assert!(!a.badge.grow(6.0).hits(b.badge.grow(6.0)), "the numbers keep 12 px apart");
    }

    #[test]
    fn a_trace_enters_the_content_a_band_lies_mostly_under() {
        // A big image instance covers the band: the trace has to enter it (the Anatomy does the same for a layer on a wide text's box).
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 500.0, 400.0);
        let image = r(100.0, 100.0, 200.0, 200.0);
        let band = r(110.0, 150.0, 180.0, 8.0);
        let mut board = Board { content: vec![image], ..Board::default() };
        let spines = place_spines(ib, stage, &[SpineReq { side: None, extra: 0.0, bands: vec![band] }], &mut board);
        assert!(spines[0].is_some());
    }

    fn lay(node: usize, depth: usize) -> Layer {
        Layer { node, family: None, depth, area: 100.0, bands: vec![Band { rect: r(0.0, 0.0, 10.0, 10.0), gap: false, value: 8.0 }] }
    }

    #[test]
    fn a_component_with_one_level_has_one_drawing() {
        // The button: the root with padding and gap, the children with none.
        let levels = levels_of(vec![lay(0, 0), Layer { bands: vec![], ..lay(1, 1) }, Layer { bands: vec![], ..lay(2, 1) }]);
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].layers.len(), 1);
        let (shown, deeper) = shown_levels(levels, 20, TOTAL_NODE_CAP);
        assert_eq!((shown.len(), deeper), (1, false));
    }

    #[test]
    fn the_levels_are_the_depths_that_have_bands_numbered_in_sequence_and_each_holds_one_depth() {
        // The root, a child level and a level two deeper (depth 2 has no layer with a band): three levels, in order of depth.
        let levels = levels_of(vec![lay(4, 3), lay(0, 0), lay(2, 1), lay(1, 1), lay(5, 3)]);
        assert_eq!(levels.iter().map(|l| l.depth).collect::<Vec<_>>(), vec![0, 1, 3]);
        assert_eq!(levels[1].layers.iter().map(|l| l.node).collect::<Vec<_>>(), vec![1, 2]);
        assert!(levels.iter().all(|l| l.layers.iter().all(|x| x.depth == l.depth)), "two layers of different depth are never in one drawing");
        // A root with no padding or gap: the first level is the next depth.
        let levels = levels_of(vec![Layer { bands: vec![], ..lay(0, 0) }, lay(1, 1)]);
        assert_eq!(levels.iter().map(|l| l.depth).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn at_most_four_drawings_and_the_rest_is_said_not_shown() {
        let five = levels_of((0..5).map(|i| lay(i, i)).collect());
        assert_eq!(five.len(), 5);
        let (shown, deeper) = shown_levels(five, 20, TOTAL_NODE_CAP);
        assert_eq!(shown.iter().map(|l| l.depth).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
        assert!(deeper);
        let three = levels_of((0..3).map(|i| lay(i, i)).collect());
        assert_eq!(shown_levels(three, 20, TOTAL_NODE_CAP), (levels_of((0..3).map(|i| lay(i, i)).collect()), false));
    }

    #[test]
    fn a_level_that_does_not_fit_the_budget_is_not_shown_by_halves() {
        let three = levels_of((0..3).map(|i| lay(i, i)).collect());
        // Each level costs its nodes (a band and a number: 4) and the 100 of the instance's copy: a budget of 250 holds two.
        let (shown, deeper) = shown_levels(three.clone(), 100, 250);
        assert_eq!((shown.len(), deeper), (2, true));
        // The first level is always shown, whatever it costs.
        let (shown, deeper) = shown_levels(three, 100, 10);
        assert_eq!((shown.len(), deeper), (1, true));
    }

    #[test]
    fn a_level_title_names_three_layers_and_then_an_ellipsis() {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(level_names(&names(&["Header", "Body"])), "Header, Body");
        assert_eq!(level_names(&names(&["Header", "Body", "Footer", "Aside"])), "Header, Body, Footer…");
        assert_eq!(level_names(&names(&["Row", "Row", "Row", "Row"])), "Row", "a name that repeats is one name (the rows of a list)");
        for lang in [Lang::En, Lang::Pt] {
            assert!(tf(lang, Key::LayoutsLevel, &[("n", "2"), ("names", "Body")]).contains("2"));
            assert!(tf(lang, Key::LayoutsLevel, &[("n", "2"), ("names", "Body")]).ends_with("Body"));
        }
    }

    fn pad_group(bands: Vec<Rect>) -> SpineGroup {
        SpineGroup { extra: 0.0, bands }
    }

    #[test]
    fn a_group_with_bands_on_both_sides_gets_a_spine_and_a_number_on_each_with_the_same_value() {
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 500.0, 400.0);
        // The left and right paddings of a card, with a text across the middle: neither can reach the other.
        let g = pad_group(vec![r(100.0, 100.0, 10.0, 200.0), r(290.0, 100.0, 10.0, 200.0)]);
        let mut board = Board { content: vec![r(110.0, 190.0, 180.0, 20.0)], ..Board::default() };
        let (placed, none) = place_group(ib, stage, &g, &mut board);
        assert!(none.is_empty());
        assert_eq!(placed.len(), 2, "one spine for each side");
        let (l, rt) = (placed.iter().find(|p| p.spine.left).unwrap(), placed.iter().find(|p| !p.spine.left).unwrap());
        assert_eq!((l.ids.clone(), rt.ids.clone()), (vec![0], vec![1]));
        assert!(l.spine.badge.right() < ib.x && rt.spine.badge.x > ib.right(), "each number stands in its own margin");
        assert!(!l.spine.badge.grow(6.0).hits(rt.spine.badge.grow(6.0)));
    }

    #[test]
    fn a_band_across_the_middle_goes_to_the_side_with_more_bands_and_on_a_tie_to_the_left() {
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let top = r(100.0, 100.0, 200.0, 10.0);
        let (l1, l2) = (r(100.0, 150.0, 10.0, 50.0), r(100.0, 210.0, 10.0, 50.0));
        let (r1, r2) = (r(290.0, 150.0, 10.0, 50.0), r(290.0, 210.0, 10.0, 50.0));
        assert_eq!(sides_of(ib, &[top, l1, l2, r1]), vec![true, true, true, false], "more on the left");
        assert_eq!(sides_of(ib, &[top, l1, r1, r2]), vec![false, true, false, false], "more on the right");
        assert_eq!(sides_of(ib, &[top, l1, r1]), vec![true, true, false], "a tie goes left");
        assert_eq!(sides_of(ib, &[top]), vec![true], "alone, left");
    }

    #[test]
    fn a_band_with_no_way_on_its_own_side_uses_the_other_side() {
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 500.0, 400.0);
        // `x` is in the left half and a text on its left blocks the way from the left; the right is clear. The right padding is the other band.
        let x = r(110.0, 150.0, 30.0, 20.0);
        let g = pad_group(vec![x, r(290.0, 100.0, 10.0, 200.0)]);
        let mut board = Board { content: vec![r(100.0, 148.0, 10.0, 24.0)], ..Board::default() };
        let (placed, none) = place_group(ib, stage, &g, &mut board);
        assert!(none.is_empty(), "{none:?}");
        assert_eq!(placed.len(), 1);
        assert!(!placed[0].spine.left);
        assert_eq!(placed[0].ids.len(), 2, "both bands on the right spine");
    }

    #[test]
    fn a_band_with_no_way_on_either_side_is_left_with_no_trace() {
        let ib = r(100.0, 100.0, 200.0, 200.0);
        let stage = r(0.0, 0.0, 500.0, 400.0);
        let middle = r(140.0, 150.0, 20.0, 20.0);
        let g = pad_group(vec![r(100.0, 100.0, 10.0, 200.0), middle]);
        let mut board = Board { content: vec![r(110.0, 148.0, 30.0, 24.0), r(160.0, 148.0, 30.0, 24.0)], ..Board::default() };
        let (placed, none) = place_group(ib, stage, &g, &mut board);
        assert_eq!(none, vec![1]);
        assert_eq!(placed.len(), 1);
    }

    #[test]
    fn a_group_on_one_side_has_one_spine_as_before() {
        let (ib, stage, texts, bands) = rows();
        let mut board = Board { content: texts, ..Board::default() };
        let (placed, none) = place_group(ib, stage, &pad_group(bands), &mut board);
        assert!(none.is_empty());
        assert_eq!(placed.len(), 1);
        assert!(placed[0].spine.left);
        assert_eq!(placed[0].ids.len(), 3);
    }

    #[test]
    fn a_group_left_with_no_trace_is_placed_first_in_the_next_try() {
        // A button: paddings of 8 above and below (across the middle: left) and of 16 at the sides. Placed in this order, the spine of the
        // 8s takes the way from the left to the left 16; placed the other way round both fit.
        let ib = r(100.0, 100.0, 100.0, 36.0);
        let stage = r(0.0, 0.0, 500.0, 300.0);
        let eight = pad_group(vec![r(100.0, 100.0, 100.0, 8.0), r(100.0, 128.0, 100.0, 8.0)]);
        let sixteen = pad_group(vec![r(100.0, 108.0, 16.0, 20.0), r(184.0, 108.0, 16.0, 20.0)]);
        let base = Board { content: vec![r(116.0, 108.0, 68.0, 20.0)], ..Board::default() };
        let got = place_all_groups(ib, stage, &[eight, sixteen], &base);
        assert_eq!(got.iter().map(|(_, n)| n.len()).sum::<usize>(), 0, "every band has a trace");
        assert_eq!(got[1].0.len(), 2, "the 16 has a number on each side");
    }
}
