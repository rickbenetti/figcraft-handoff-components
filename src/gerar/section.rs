//! The 0.11 document: one Section for the component, with a frame (in ONE ROW, left to right) for each part of the handoff (Cover, Component Master,
//! Properties, Anatomy, States and Variants, Layouts, Tokens, Changelog and the support components), 1920 px wide, 64 px
//! apart, with 64 px of room around them inside the Section.
//!
//! The first generation makes everything. A rerun replaces only the frames "Update" names (the five that are drawn from the
//! component); Cover, Changelog and the support components are made once, never replaced, and made again only when they were
//! deleted by hand. The Component Master is made again on every run (so a new version of the plugin reaches a document that
//! exists), with the original component moved out of the old frame and into the new one. "New Section" makes another Section
//! beside the first, with every frame but the Master.
//!
//! In this step Cover, Component Master, Changelog and the support components have their content; the other five frames carry
//! their title and a line that says when they are made.

#![warn(dead_code)] // the module above allows dead code (the 0.10.0 drawing); this one is checked

use super::*;
use crate::contrast::{self, Pick3, Stage};

/// The frame's width, its padding and the useful width inside it (the tables are always this wide).
pub const FRAME_W: f64 = 1180.0;
pub const FRAME_PAD: f64 = 48.0;
pub const USEFUL_W: f64 = FRAME_W - 2.0 * FRAME_PAD;
/// The room around the frames inside the Section, and the gap between them.
pub const SECTION_PAD: f64 = 64.0;
pub const FRAME_GAP: f64 = 64.0;
/// The frames are in ONE COLUMN, top to bottom, all at x = 64 inside the Section, the first at y = 64 and each one 64 below the one above
/// (eight frames: 64 + the heights + 7 × 64 + 64).
/// The room between the right edge of everything on the page and a new Section.
pub const SECTION_RIGHT_GAP: f64 = 256.0;

/// The y of the first Section (and of the Support frame) on the page.
pub const ORIGIN_Y: f64 = 256.0;

/// The x, inside the Section, of every frame.
pub const FRAME_X: f64 = SECTION_PAD;

/// A frame of the Section where it stands: its place inside the Section, and its size.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub sec: Sec,
    pub id: u64,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Placed {
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
}

/// The y of each frame of a new column, from the heights of the frames in order: the first at 64, each 64 under the one above.
pub fn column_ys(heights: &[f64]) -> Vec<f64> {
    let mut y = SECTION_PAD;
    heights
        .iter()
        .map(|h| {
            let at = y;
            y += h + FRAME_GAP;
            at
        })
        .collect()
}

/// The Section's size: 64 + the widest frame + 64 wide, and 64 + the lowest bottom + 64 tall (for a column that was never touched
/// by hand that is 64 + the sum of the heights + 7 × 64 + 64).
pub fn section_size(frames: &[Placed]) -> (f64, f64) {
    let widest = frames.iter().map(|f| f.w).fold(0.0, f64::max);
    let lowest = frames.iter().map(Placed::bottom).fold(SECTION_PAD, f64::max);
    (SECTION_PAD + widest + SECTION_PAD, lowest + SECTION_PAD)
}

/// A frame of `sec` was replaced and its height went from `old_h` to `new_h`: the frames whose top was at or under its old bottom move by
/// the difference (the frame itself keeps its place). Nothing else is rearranged, what the person moved by hand stays where it is.
pub fn shift_below(frames: &mut [Placed], sec: Sec, old_y: f64, old_h: f64, new_h: f64) {
    let delta = new_h - old_h;
    if delta.abs() < 0.5 {
        return;
    }
    let old_bottom = old_y + old_h;
    for f in frames.iter_mut().filter(|f| f.sec != sec && f.y >= old_bottom - 0.5) {
        f.y += delta;
    }
}

/// Where a Section goes: to the right of everything on the page (its largest right edge + 256; 0 on an empty page), at y = 256, or at
/// the y of the Section it goes beside ("New Section"). `boxes` are the page's top-level boxes as (x, y, w, h).
pub fn section_origin(boxes: &[(f64, f64, f64, f64)], beside_y: Option<f64>) -> (f64, f64) {
    let right = boxes.iter().map(|(x, _, w, _)| x + w).fold(None, |r: Option<f64>, v| Some(r.map_or(v, |r| r.max(v))));
    (right.map_or(0.0, |r| r + SECTION_RIGHT_GAP), beside_y.unwrap_or(ORIGIN_Y))
}

/// The mark of the Section on the page (the value is the id of the component), of a frame's section, and of the set of approval pills.
pub const SECTION_MARK: &str = "handoff-report-section";
pub const NAME_MARK: &str = "handoff-section";
pub const APPROVAL_SET_MARK: &str = "handoff-approval-set";
/// The mark of the Support frame: the top-level frame of the page that holds the set of approval pills for every Section of the document.
pub const SUPPORT_MARK: &str = "handoff-support";
/// The stage inside the Component Master frame.
const STAGE_PAD: f64 = 48.0;
/// The lowest contrast the automatic stage should reach for every variant (WCAG graphics: 3:1).
pub const MIN_STAGE_CONTRAST: f64 = 3.0;

/// The colours of the new frames, in ONE table (the light theme; change these to change the theme). The 0.10.0 tables keep theirs.
pub mod pal {
    pub const SECTION: &str = "#E7E7EA";
    pub const FRAME: &str = "#F4F4F5";
    /// Cards, pills and chips.
    pub const CARD: &str = "#FFFFFF";
    pub const LINE: &str = "#D4D4D8";
    /// The border of a colour sample (a white or a pale one must not vanish on the frame).
    pub const SWATCH_BORDER: &str = "#A1A1AA";
    pub const TEXT: &str = "#1C1C1F";
    pub const TEXT2: &str = "#56565C";
    pub const GUIDE: &str = "#6E6E75";
    pub const OK: &str = "#167A4C";
    pub const COND: &str = "#9A6410";
    pub const BAD: &str = "#C13B30";
    pub const PEND: &str = "#6E6E75";
    /// The symbol inside a state circle.
    pub const ON_STATE: &str = "#FFFFFF";
}

const CELL_PAD: [f64; 4] = [12.0, 16.0, 12.0, 16.0];
const TABLE_RADIUS: f64 = 8.0;
const CHANGELOG_COLS: [f64; 3] = [160.0, 180.0, 744.0];
/// The circle of a state, the pill that holds it, and the name of the variant property of the set that holds the four pills.
const STATE_D: f64 = 44.0;
const PILL_W: f64 = 290.0;
const STATE_PROP: &str = "Estado";

// ---------------------------------------------------------------------------------------------
// The sections, what to update, the approval states
// ---------------------------------------------------------------------------------------------

/// A frame of the Section, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sec {
    Cover,
    Master,
    Properties,
    Anatomy,
    Variants,
    Layouts,
    Tokens,
    Changelog,
    /// The Support frame of a Section made before 0.11's shared Support frame: such a Section keeps it (found by its mark, never made again).
    Kit,
}

impl Sec {
    /// The eight frames of a Section's column, in order. (The Support frame is not one of them: there is one for the whole document.)
    pub const ALL: [Sec; 8] = [Sec::Cover, Sec::Master, Sec::Properties, Sec::Anatomy, Sec::Variants, Sec::Layouts, Sec::Tokens, Sec::Changelog];

    /// The value of the frame's `handoff-section` data.
    pub fn key(self) -> &'static str {
        match self {
            Sec::Cover => "cover",
            Sec::Master => "master",
            Sec::Properties => "properties",
            Sec::Anatomy => "anatomy",
            Sec::Variants => "variants",
            Sec::Layouts => "layouts",
            Sec::Tokens => "tokens",
            Sec::Changelog => "changelog",
            Sec::Kit => "kit",
        }
    }

    pub fn title_key(self) -> Key {
        match self {
            Sec::Cover => Key::SecCover,
            Sec::Master => Key::SecMaster,
            Sec::Properties => Key::SecProperties,
            Sec::Anatomy => Key::SecAnatomy,
            Sec::Variants => Key::SecVariants,
            Sec::Layouts => Key::SecLayouts,
            Sec::Tokens => Key::SecTokens,
            Sec::Changelog => Key::SecChangelog,
            Sec::Kit => Key::SecKit,
        }
    }

    /// The place of the frame in the column (0 to 7; the old Support frame comes after the others).
    pub fn index(self) -> usize {
        Sec::ALL.iter().position(|s| *s == self).unwrap_or(Sec::ALL.len())
    }

    /// The frame a `handoff-section` mark names, the old Support frame included.
    pub fn from_key(key: &str) -> Option<Sec> {
        Sec::ALL.into_iter().chain([Sec::Kit]).find(|s| s.key() == key)
    }

    /// Made once, never replaced by a rerun, made again only when it was deleted by hand. (The Component Master is not one of
    /// them: it is made again on every run.)
    pub fn permanent(self) -> bool {
        matches!(self, Sec::Cover | Sec::Changelog | Sec::Kit)
    }
}

/// What the panel's "Update" asks a rerun to replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    All,
    Tables,
    Tokens,
    Properties,
    Anatomy,
    Variants,
    Layouts,
}

impl Update {
    pub const ALL: [Update; 7] = [Update::All, Update::Tables, Update::Tokens, Update::Properties, Update::Anatomy, Update::Variants, Update::Layouts];

    /// The value of the `update` parameter.
    pub fn key(self) -> &'static str {
        match self {
            Update::All => "all",
            Update::Tables => "tables",
            Update::Tokens => "tokens",
            Update::Properties => "properties",
            Update::Anatomy => "anatomy",
            Update::Variants => "variants",
            Update::Layouts => "layouts",
        }
    }

    pub fn from_key(key: &str) -> Option<Update> {
        Update::ALL.into_iter().find(|u| u.key() == key)
    }

    pub fn label_key(self) -> Key {
        match self {
            Update::All => Key::UpdAll,
            Update::Tables => Key::UpdTables,
            Update::Tokens => Key::UpdTokens,
            Update::Properties => Key::UpdProperties,
            Update::Anatomy => Key::UpdAnatomy,
            Update::Variants => Key::UpdVariants,
            Update::Layouts => Key::UpdLayouts,
        }
    }

    /// The frames a rerun replaces.
    pub fn sections(self) -> &'static [Sec] {
        match self {
            Update::All => &[Sec::Properties, Sec::Anatomy, Sec::Variants, Sec::Layouts, Sec::Tokens],
            Update::Tables => &[Sec::Properties, Sec::Anatomy, Sec::Variants, Sec::Layouts],
            Update::Tokens => &[Sec::Tokens],
            Update::Properties => &[Sec::Properties],
            Update::Anatomy => &[Sec::Anatomy],
            Update::Variants => &[Sec::Variants],
            Update::Layouts => &[Sec::Layouts],
        }
    }
}

/// One of the four approval states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Approval {
    Pending,
    Approved,
    Conditional,
    Rejected,
}

impl Approval {
    pub const ALL: [Approval; 4] = [Approval::Pending, Approval::Approved, Approval::Conditional, Approval::Rejected];

    /// The value of the variant property "Estado" (fixed: it is the name of a property of a component, not a text of the panel).
    pub fn value(self) -> &'static str {
        match self {
            Approval::Pending => "pendente",
            Approval::Approved => "aprovado",
            Approval::Conditional => "ressalvas",
            Approval::Rejected => "reprovado",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Approval::Pending => "…",
            Approval::Approved => "✓",
            Approval::Conditional => "!",
            Approval::Rejected => "✕",
        }
    }

    pub fn colour(self) -> &'static str {
        match self {
            Approval::Pending => pal::PEND,
            Approval::Approved => pal::OK,
            Approval::Conditional => pal::COND,
            Approval::Rejected => pal::BAD,
        }
    }

    pub fn label_key(self) -> Key {
        match self {
            Approval::Pending => Key::StatePending,
            Approval::Approved => Key::StateApproved,
            Approval::Conditional => Key::StateConditional,
            Approval::Rejected => Key::StateRejected,
        }
    }

    /// The name of the variant: "Estado=pendente".
    pub fn variant_name(self) -> String {
        format!("{STATE_PROP}={}", self.value())
    }
}

/// Something the person should know after a run, most important first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warn {
    /// A content frame could not be made: it holds the error.
    Failed,
    /// No stage suits every variant (the automatic stage's lowest contrast is under 3:1).
    NoStage,
    /// The set could not be organized in lines (the host did not keep the auto layout with wrap): the Master was made as wide as the set.
    ReflowFailed,
    /// The component is wider than the room a Master has even in one line: every frame of the Section was made wider.
    WideMaster,
    /// A loose documentation of version 0.10.0 was found and left as it was.
    LegacyDoc,
    /// The set of approval pills of the file has no text property for the area: the Cover's pills were made with no area written.
    ApprovalProperty,
    /// Markers of the Anatomy that did not fit in the drawing (how many numbers have none): the table says so in its Note column.
    MarkersDropped(usize),
    /// Numbers of the Layouts drawing that did not fit (how many): the frame says which in a note under the drawing.
    SpacingDropped(usize),
    /// The support frame (and the set of approval pills) had to be made again for a Cover and could not be: the Cover has no pills.
    SupportMissing,
    /// Frames that are not there and that "Update" does not name, so no run made them (a bit for each frame of `Sec::ALL`, by its index).
    FrameMissing(u16),
}

impl Warn {
    pub const ALL: [Warn; 10] = [Warn::Failed, Warn::NoStage, Warn::ReflowFailed, Warn::WideMaster, Warn::LegacyDoc, Warn::ApprovalProperty, Warn::SupportMissing, Warn::FrameMissing(0), Warn::MarkersDropped(0), Warn::SpacingDropped(0)];

    pub fn code(self) -> &'static str {
        match self {
            Warn::Failed => "table_failed",
            Warn::NoStage => "no_stage",
            Warn::ReflowFailed => "reflow_failed",
            Warn::WideMaster => "wide_master",
            Warn::LegacyDoc => "legacy_doc",
            Warn::ApprovalProperty => "approval_property_missing",
            Warn::MarkersDropped(_) => "markers_dropped",
            Warn::SpacingDropped(_) => "spacing_dropped",
            Warn::SupportMissing => "support_missing",
            Warn::FrameMissing(_) => "frame_missing",
        }
    }

    pub fn text(self, lang: Lang) -> &'static str {
        t(lang, match self {
            Warn::Failed => Key::PanelWarnFailed,
            Warn::NoStage => Key::PanelWarnNoStage,
            Warn::ReflowFailed => Key::PanelWarnReflow,
            Warn::WideMaster => Key::PanelWarnWide,
            Warn::LegacyDoc => Key::PanelWarnLegacy,
            Warn::ApprovalProperty => Key::PanelWarnApproval,
            Warn::MarkersDropped(_) => Key::PanelWarnMarkers,
            Warn::SpacingDropped(_) => Key::PanelWarnSpacing,
            Warn::SupportMissing => Key::PanelWarnSupport,
            Warn::FrameMissing(_) => Key::PanelWarnMissing,
        })
    }

    /// The warning as the panel says it: with its number filled in where it has one.
    pub fn line(self, lang: Lang) -> String {
        match self {
            Warn::MarkersDropped(n) => tf(lang, plural(n, Key::PanelWarnMarkersOne, Key::PanelWarnMarkers), &[("n", &n.to_string())]),
            Warn::SpacingDropped(n) => tf(lang, plural(n, Key::PanelWarnSpacingOne, Key::PanelWarnSpacing), &[("n", &n.to_string())]),
            Warn::FrameMissing(mask) => {
                let names: Vec<&str> = Sec::ALL.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, s)| t(lang, s.title_key())).collect();
                tf(lang, plural(names.len(), Key::PanelWarnMissingOne, Key::PanelWarnMissing), &[("names", &names.join(", "))])
            }
            w => w.text(lang).to_string(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The automatic stage over every variant
// ---------------------------------------------------------------------------------------------

/// What a variant shows the stage: its own background (its root's solid fill, else the set's), else the colours of its elements.
#[derive(Clone, Debug, Default)]
pub struct Judged {
    pub backdrop: Option<Rgba>,
    pub elements: Vec<Rgba>,
}

/// The colours the stage has to stand out from, over all variants: each variant's background when it has one, else its elements.
pub fn union_colours(variants: &[Judged], set_fill: Option<Rgba>) -> Vec<Rgba> {
    let mut out = Vec::new();
    for v in variants {
        match v.backdrop.or(set_fill) {
            Some(b) => out.push(b),
            None => out.extend(v.elements.iter().copied()),
        }
    }
    out
}

/// The automatic stage for those variants: the light, mid-grey or dark stage that maximises the lowest contrast over the union
/// (a tie is light, then mid-grey, then dark).
pub fn pick_union(variants: &[Judged], set_fill: Option<Rgba>) -> Pick3 {
    contrast::pick_stage3(&union_colours(variants, set_fill))
}

/// The stage the panel's "Stage" asks for: which of the three, and the lowest contrast when it is automatic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageChoice {
    pub stage: Stage,
    pub lowest: Option<f64>,
    pub auto: bool,
}

impl StageChoice {
    pub fn fill(self) -> &'static str {
        match self.stage {
            Stage::Light => theme::STAGE_LIGHT,
            Stage::Mid => theme::STAGE_MID,
            Stage::Dark => theme::PANEL,
        }
    }

    pub fn name(self) -> &'static str {
        match self.stage {
            Stage::Light => "light",
            Stage::Mid => "mid",
            Stage::Dark => "dark",
        }
    }

    /// Whether the automatic stage leaves some variant under 3:1.
    pub fn too_low(self) -> bool {
        self.auto && self.lowest.is_some_and(|l| l < MIN_STAGE_CONTRAST)
    }
}

/// The colours of one variant, as the layers under it say.
pub(super) fn judge(nodes: &[Value]) -> Judged {
    Judged { backdrop: nodes.first().and_then(solid_fill), elements: element_colours(nodes) }
}

/// The stage for one set of judged variants in `mode`: forced Light or Dark, or the best of the three.
pub(super) fn pick_stage_for(mode: StageMode, variants: &[Judged], set_fill: Option<Rgba>) -> StageChoice {
    match mode {
        StageMode::Light => StageChoice { stage: Stage::Light, lowest: None, auto: false },
        StageMode::Dark => StageChoice { stage: Stage::Dark, lowest: None, auto: false },
        StageMode::Auto => {
            let pick = pick_union(variants, set_fill);
            StageChoice { stage: pick.stage, lowest: pick.lowest, auto: true }
        }
    }
}

/// The most variants, and the most layers in all of them, the automatic stage looks at (each layer is a host call).
pub const MAX_STAGE_VARIANTS: usize = 30;
const MAX_STAGE_NODES: usize = 1_200;

/// The layers under `root` (itself first), as far as `budget` allows, with only what the stage's colours need: their paints, their
/// opacity and whether they are visible.
fn colour_nodes(lang: Lang, root: u64, budget: usize) -> Result<Vec<Value>> {
    let mut nodes = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if nodes.len() >= budget.max(1) {
            break;
        }
        let node = host(lang, "node.get", &json!({"id": id, "fields": ["fills", "strokes", "opacity", "visible", "children"]}))?;
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            stack.extend(children.iter().rev().filter_map(|c| c.get("id").and_then(Value::as_u64)));
        }
        nodes.push(node);
    }
    Ok(nodes)
}

/// The stage of the Component Master for `target` in `mode`.
/// The variants the automatic stage reads: all of them when there are at most `max`; otherwise the default variant and, for each
/// option of each axis, the variant with that option and the others at the default (one per value of each axis), cut evenly when
/// even those are more than `max`. Every colour a single property can bring is in the sample; a colour that exists only in a
/// combination of two properties may not be.
pub fn stage_sample(info: &SetInfo, default: &Pairs, max: usize) -> Vec<u64> {
    let all: Vec<u64> = info.variants.iter().map(|v| v.id).collect();
    if all.len() <= max {
        return all;
    }
    let mut ids: Vec<u64> = info.find(default).map(|v| v.id).into_iter().collect();
    for axis in variants::axes(info, default, &[]) {
        for opt in axis.options {
            if let Some(v) = opt.variant
                && !ids.contains(&v.id)
            {
                ids.push(v.id);
            }
        }
    }
    if ids.len() > max {
        let step = ids.len() as f64 / max as f64;
        ids = (0..max).map(|i| ids[(i as f64 * step) as usize]).collect();
    }
    ids
}

fn choose_stage(lang: Lang, target: &Picked, mode: StageMode, sampled: &std::cell::Cell<Option<(usize, usize)>>) -> Result<StageChoice> {
    match mode {
        StageMode::Light => Ok(StageChoice { stage: Stage::Light, lowest: None, auto: false }),
        StageMode::Dark => Ok(StageChoice { stage: Stage::Dark, lowest: None, auto: false }),
        StageMode::Auto => {
            let ids: Vec<u64> = match &target.set {
                Some(s) => {
                    let ids = stage_sample(&s.info, &s.default, MAX_STAGE_VARIANTS);
                    if ids.len() < s.info.variants.len() {
                        sampled.set(Some((ids.len(), s.info.variants.len())));
                    }
                    ids
                }
                None => vec![target.component],
            };
            let set_fill = set_backdrop(lang, target)?;
            let mut judged = Vec::new();
            let mut seen = 0usize;
            for id in ids {
                let nodes = colour_nodes(lang, id, MAX_STAGE_NODES.saturating_sub(seen))?;
                seen += nodes.len();
                judged.push(judge(&nodes));
            }
            let pick = pick_union(&judged, set_fill);
            Ok(StageChoice { stage: pick.stage, lowest: pick.lowest, auto: true })
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Finding what is already there
// ---------------------------------------------------------------------------------------------

pub(super) fn mark_of<'a>(node: &'a Value, key: &str) -> Option<&'a str> {
    node.get("pluginData").and_then(|d| d.get(key)).and_then(Value::as_str)
}

/// The id of the first Section of `key` among the page's own children.
pub fn find_section(page: &PageInfo, key: &str) -> Option<u64> {
    sections_of(page, key).first().copied()
}

/// The ids of every Section of `key` on the page, in the page's order.
pub fn sections_of(page: &PageInfo, key: &str) -> Vec<u64> {
    page.children.iter().filter(|c| mark_of(c, SECTION_MARK) == Some(key)).filter_map(|c| c.get("id").and_then(Value::as_u64)).collect()
}

/// A loose documentation of version 0.10.0: a frame on the page with the old mark and no Section.
fn has_legacy_frame(page: &PageInfo, key: &str) -> bool {
    page.children.iter().any(|c| mark_of(c, REPORT_KEY) == Some(key))
}

/// A frame of the Section: its id, its place in the Section (y) and its height.
#[derive(Clone, Debug, PartialEq)]
pub struct Existing {
    pub id: u64,
    pub sec: Sec,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The frames of a Section (one `node.get`): those with a known `handoff-section` mark, the first of each kind.
fn read_frames(lang: Lang, section: u64) -> Result<(Rect, Vec<Existing>)> {
    let node = host(lang, "node.get", &json!({"id": section, "fields": ["children", "pluginData", "absoluteBounds"]}))?;
    let sb = bounds(&node).unwrap_or_default();
    let mut out: Vec<Existing> = Vec::new();
    for c in node.get("children").and_then(Value::as_array).into_iter().flatten() {
        let Some(sec) = mark_of(c, NAME_MARK).and_then(Sec::from_key) else { continue };
        let (Some(id), Some(b)) = (c.get("id").and_then(Value::as_u64), bounds(c)) else { continue };
        if out.iter().any(|e| e.sec == sec) {
            continue;
        }
        out.push(Existing { id, sec, x: b.x - sb.x, y: b.y - sb.y, w: b.w, h: b.h });
    }
    Ok((sb, out))
}

/// The size of a frame as the layers inside it say it: the width read from the frame, the height the frame has or, when a layer reaches
/// lower than the frame's bottom padding allows (the host had not laid the frame out again), the bottom of the lowest layer plus the
/// padding, written back. One read (and a write only when the frame was short): the frames under it, and the Section, are placed from
/// this height, never from an estimate.
fn settled_size(lang: Lang, frame: u64) -> Result<(f64, f64)> {
    let node = host(lang, "node.get", &json!({"id": frame, "fields": ["absoluteBounds", "children"]}))?;
    let (w, h) = bounds(&node).map_or((0.0, 0.0), |b| (b.w, b.h));
    let content = node
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| Some(c.get("y")?.as_f64()? + c.get("height")?.as_f64()?))
        .fold(0.0, f64::max);
    let needed = content + FRAME_PAD;
    if needed > h + 0.5 {
        host(lang, "node.set", &json!({"ids": [frame], "props": {"height": needed, "sizingV": "fixed"}}))?;
        return Ok((w, needed));
    }
    Ok((w, h))
}

/// The set of approval pills in the support frame: the child that carries the `handoff-approval-set` mark.
fn read_approval_set(lang: Lang, kit: u64) -> Result<Option<u64>> {
    let node = host(lang, "node.get", &json!({"id": kit, "fields": ["children", "pluginData"]}))?;
    Ok(node.get("children").and_then(Value::as_array).into_iter().flatten().find(|c| mark_of(c, APPROVAL_SET_MARK).is_some()).and_then(|c| c.get("id").and_then(Value::as_u64)))
}

/// A property name with the case and the accents taken off, so "Area" and "Área" are the same name.
fn fold_name(name: &str) -> String {
    name.trim().chars().map(|ch| match ch { 'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' | 'Ã' => 'a', _ => ch.to_ascii_lowercase() }).collect()
}

/// The text property of a set of pills (the reply of `component.properties`) that holds the area: the one named like the area in ANY
/// language of the plugin, accents and case aside. The set belongs to the file, not to the language of this run: it may have been made
/// in another one, so its own name is the only one that can be written. None when the set has no such property.
pub(super) fn area_property(properties: &[Value]) -> Option<String> {
    let known: Vec<String> = [Lang::En, Lang::Pt].iter().map(|l| fold_name(t(*l, Key::ApprPropArea))).collect();
    properties
        .iter()
        .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|p| p.get("name").and_then(Value::as_str))
        .find(|n| known.contains(&fold_name(n)))
        .map(str::to_string)
}

/// The name that the set of pills `set` of the file gives its area property.
fn read_area_property(lang: Lang, set: u64) -> Result<Option<String>> {
    let props = host(lang, "component.properties", &json!({"id": set}))?;
    Ok(area_property(props.get("properties").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default()))
}

/// The sets of approval pills of the whole document, and which one the Covers use. The plugin cannot change the page, but it can read
/// any page's top-level layers by id: the Support frames (`SUPPORT_MARK`) and, for the Sections made before the shared Support frame, the
/// Support frame inside each Section. When there are several the OLDEST is used (the lowest id: ids grow as layers are made), so the
/// choice is the same on every run. Returns that set and how many were found.
fn find_support(lang: Lang, page: &PageInfo) -> Result<(Option<u64>, usize)> {
    let pages = host(lang, "page.list", &json!({}))?;
    let mut sets: Vec<u64> = Vec::new();
    for p in pages.as_array().map(Vec::as_slice).unwrap_or_default().iter().take(MAX_PAGES) {
        let (Some(id), n) = (p.get("id").and_then(Value::as_u64), p.get("children").and_then(Value::as_u64).unwrap_or(1)) else { continue };
        if n == 0 {
            continue;
        }
        let children: Vec<Value> = if id == page.id { page.children.clone() } else { host(lang, "node.get", &json!({"id": id, "fields": ["children", "name", "type", "pluginData"]}))?.get("children").and_then(Value::as_array).cloned().unwrap_or_default() };
        for c in children.iter().take(MAX_TOP_LEVEL) {
            let Some(cid) = c.get("id").and_then(Value::as_u64) else { continue };
            if mark_of(c, APPROVAL_SET_MARK).is_some() {
                // The set itself, standing on the page (taken out of its frame by hand).
                sets.push(cid);
            } else if mark_of(c, SUPPORT_MARK).is_some() {
                sets.extend(read_approval_set(lang, cid)?);
            } else if mark_of(c, SECTION_MARK).is_some() {
                let (_, frames) = read_frames(lang, cid)?;
                if let Some(kit) = frames.iter().find(|f| f.sec == Sec::Kit) {
                    sets.extend(read_approval_set(lang, kit.id)?);
                }
            }
        }
    }
    sets.sort_unstable();
    sets.dedup();
    Ok((sets.first().copied(), sets.len()))
}

/// The most pages looked at for the set of approval pills.
const MAX_PAGES: usize = 500;

/// The Support frame: a top-level frame of the page (title, the note that says not to delete it, and the set of approval pills), as wide
/// as its content, at (`x`, `y`). Returns its id, the set's id and the frame's size.
fn build_support(c: &Ctx, x: f64, y: f64) -> Result<(u64, u64, f64, f64)> {
    let lang = c.lang;
    let made = host(
        lang,
        "frame.create",
        &json!({"x": x, "y": y, "width": 400, "height": 200, "name": t(lang, Key::SupportTitle), "select": false,
            "props": {"fill": pal::FRAME, "layout": "vertical", "gap": 32, "padding": FRAME_PAD, "crossAlign": "min", "sizingH": "hug", "sizingV": "hug", "cornerRadius": TABLE_RADIUS, "clip": false,
                "pluginData": {SUPPORT_MARK: "1"}}}),
    )?;
    let frame = id_of(lang, &made, "frame.create")?;
    text_with(lang, frame, t(lang, Key::SupportTitle), theme::TITLE_SIZE, pal::TEXT, "Title", bold())?;
    let note = text_with(lang, frame, t(lang, Key::KitNote), LINE_SIZE, pal::COND, "Warning", json!({"width": 400, "textAutoResize": "height"}))?;
    let set = build_approval_set(c, frame)?;
    // The note is as wide as the set (the frame hugs both).
    let (set_w, _) = size_of(lang, set)?;
    host(lang, "node.set", &json!({"ids": [note], "props": {"width": set_w}}))?;
    let (w, h) = size_of(lang, frame)?;
    Ok((frame, set, w, h))
}

// ---------------------------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------------------------

/// A frame of the Section, at `y` inside it (`x` is the room around): 1920 wide, padding 48, auto layout, hug.
fn new_frame(lang: Lang, section: u64, name: &str, comp_key: &str, sec: Sec, width: f64) -> Result<u64> {
    let created = host(
        lang,
        "frame.create",
        &json!({"parent": section, "x": FRAME_X, "y": SECTION_PAD, "width": width, "height": 200, "name": name, "select": false,
            "props": {"fill": pal::FRAME, "layout": "vertical", "gap": 32, "padding": FRAME_PAD, "crossAlign": "min", "sizingH": "fixed", "sizingV": "hug", "cornerRadius": TABLE_RADIUS, "clip": false,
                "pluginData": {REPORT_KEY: comp_key, NAME_MARK: sec.key()}}}),
    )?;
    id_of(lang, &created, "frame.create")
}

/// What a table cell holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Cv {
    /// The text of the table: the main colour.
    Text(String),
    /// The name of the row (bold).
    Bold(String),
    /// Secondary text.
    Muted(String),
    /// A guide (grey, for the person to replace).
    Guide(String),
    /// A value: its token (secondary, above it) and the value, with a square of its colour when it is one (`true`: hollow, a colour
    /// with an alpha).
    Val { token: Option<String>, text: String, swatch: Option<(String, bool)> },
    /// Several values stacked in one cell (the tokens of one layer), each in a frame of its own.
    Many(Vec<Cv>),
}

impl Cv {
    /// The nodes the cell makes: its frame, and what is in it.
    pub(super) fn nodes(&self) -> usize {
        match self {
            // Without a colour the token and the value are in the cell itself; with one, a square and a column of the two lines.
            Cv::Val { token, swatch, .. } => usize::from(token.is_some()) + if swatch.is_some() { 4 } else { 2 },
            // The cell, and each item: a value has a frame of its own (what `nodes` says of a value cell), a text is one node.
            Cv::Many(items) => 1 + items.iter().map(|i| if matches!(i, Cv::Val { .. } | Cv::Many(_)) { i.nodes() } else { 1 }).sum::<usize>(),
            _ => 2,
        }
    }
}

/// The nodes a table makes for these headers and rows: the grid, a header cell each (2), a separator, and the rows.
pub(super) fn table_nodes(headers: usize, rows: &[Vec<Cv>]) -> usize {
    1 + 2 * headers + 1 + rows.iter().map(|r| 1 + r.iter().map(Cv::nodes).sum::<usize>()).sum::<usize>()
}

fn cell(lang: Lang, grid: u64, col: usize, row: usize, fill: Option<&str>) -> Result<u64> {
    cell_in(lang, grid, col, row, fill, "horizontal")
}

fn cell_in(lang: Lang, grid: u64, col: usize, row: usize, fill: Option<&str>, direction: &str) -> Result<u64> {
    let mut f = FlowSpec::new("Cell", 100.0, direction);
    f.padding = json!(CELL_PAD);
    f.fill = fill;
    f.gap = if direction == "vertical" { 2.0 } else { 8.0 };
    f.extra = json!({"sizingH": "fill", "sizingV": "hug", "crossAlign": if direction == "vertical" { "min" } else { "center" }, "gridColumn": col, "gridRow": row, "gridAlignV": "min"});
    flow_frame(lang, grid, f)
}

fn cell_text(lang: Lang, parent: u64, s: &str, fill: &str, bold: bool) -> Result<u64> {
    let extra = if bold { json!({"sizingH": "fill", "textAutoResize": "height", "fontStyle": "Bold"}) } else { json!({"sizingH": "fill", "textAutoResize": "height"}) };
    text_with(lang, parent, s, LINE_SIZE, fill, "Text", extra)
}

fn separator(lang: Lang, grid: u64, row: usize, cols: usize) -> Result<()> {
    host(
        lang,
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 100, "height": 1, "parent": grid, "name": "Separator", "select": false,
            "props": {"fill": pal::LINE, "gridColumn": 0, "gridRow": row, "gridColumnSpan": cols, "sizingH": "fill"}}),
    )?;
    Ok(())
}

/// A table of fixed columns: a header row, then the rows, each followed by a separator of 1 px (a grid with no row gap and
/// the border out of the layout, so the cells and the separators touch).
pub(super) fn table(lang: Lang, frame: u64, name: &str, cols: &[f64], headers: &[String], rows: &[Vec<Cv>]) -> Result<u64> {
    let n = cols.len();
    let mut tracks: Vec<Value> = vec![json!("auto"), json!(1)];
    for _ in rows {
        tracks.push(json!("auto"));
        tracks.push(json!(1));
    }
    let created = host(
        lang,
        "frame.create",
        &json!({"parent": frame, "width": cols.iter().sum::<f64>(), "height": 40, "name": name, "select": false,
            "props": {"layout": "grid", "gridColumns": n, "gridRows": 2 + 2 * rows.len(), "columnTracks": cols, "rowTracks": tracks, "gap": 0, "rowGap": 0, "strokesIncluded": false,
                "padding": 0, "sizingH": "fixed", "sizingV": "hug", "fill": pal::FRAME, "cornerRadius": TABLE_RADIUS, "stroke": pal::LINE, "strokeWeight": 1, "strokeAlign": "inside", "clip": true}}),
    )?;
    let grid = id_of(lang, &created, "frame.create")?;
    for (col, h) in headers.iter().enumerate() {
        let c = cell(lang, grid, col, 0, Some(pal::CARD))?;
        cell_text(lang, c, h, pal::TEXT2, true)?;
    }
    separator(lang, grid, 1, n)?;
    for (i, row) in rows.iter().enumerate() {
        let gr = 2 + 2 * i;
        for (col, v) in row.iter().enumerate() {
            let direction = if matches!(v, Cv::Val { swatch: None, .. } | Cv::Many(_)) { "vertical" } else { "horizontal" };
            let c = cell_in(lang, grid, col, gr, None, direction)?;
            match v {
                Cv::Text(s) => cell_text(lang, c, s, pal::TEXT, false)?,
                Cv::Bold(s) => cell_text(lang, c, s, pal::TEXT, true)?,
                Cv::Muted(s) => cell_text(lang, c, s, pal::TEXT2, false)?,
                Cv::Guide(s) => cell_text(lang, c, s, pal::GUIDE, false)?,
                Cv::Val { token, text, swatch } => value_cell(lang, c, token.as_deref(), text, swatch.as_ref())?,
                Cv::Many(items) => many_cell(lang, c, items)?,
            };
        }
        separator(lang, grid, gr + 1, n)?;
    }
    Ok(grid)
}

/// A cell with several values: each in a frame of its own (a row when it has a square of a colour, else a column), the first line of
/// a value's text bold when it has a token.
fn many_cell(lang: Lang, cell: u64, items: &[Cv]) -> Result<u64> {
    for item in items {
        match item {
            Cv::Val { token, text, swatch } => {
                let mut f = FlowSpec::new("Item", 100.0, if swatch.is_some() { "horizontal" } else { "vertical" });
                f.gap = if swatch.is_some() { 8.0 } else { 2.0 };
                f.extra = json!({"sizingH": "fill", "crossAlign": "min"});
                let frame = flow_frame(lang, cell, f)?;
                value_cell(lang, frame, token.as_deref(), text, swatch.as_ref())?;
            }
            Cv::Text(s) | Cv::Bold(s) | Cv::Muted(s) | Cv::Guide(s) => {
                let (colour, bold) = match item {
                    Cv::Bold(_) => (pal::TEXT, true),
                    Cv::Muted(_) => (pal::TEXT2, false),
                    Cv::Guide(_) => (pal::GUIDE, false),
                    _ => (pal::TEXT, false),
                };
                cell_text(lang, cell, s, colour, bold)?;
            }
            Cv::Many(inner) => {
                many_cell(lang, cell, inner)?;
            }
        }
    }
    Ok(cell)
}

/// A value cell. With a colour: a square of it (every square has a 1 px border in the `swatch border` colour, so a white one does not vanish)
/// and a column of the token and the value. Without: the token (when there is one) and the value are in the cell itself.
fn value_cell(lang: Lang, cell: u64, token: Option<&str>, text: &str, swatch: Option<&(String, bool)>) -> Result<u64> {
    let (tok_parent, value_parent) = match swatch {
        Some((hex, _alpha)) => {
            host(
                lang,
                "shape.rectangle",
                &json!({"x": 0, "y": 0, "width": 16, "height": 16, "parent": cell, "name": "Swatch", "select": false,
                    "props": {"fill": hex, "stroke": pal::SWATCH_BORDER, "strokeWeight": 1, "strokeAlign": "inside", "cornerRadius": 3}}),
            )?;
            let mut col = FlowSpec::new("Value", 100.0, "vertical");
            col.gap = 2.0;
            col.extra = json!({"sizingH": "fill"});
            let col = flow_frame(lang, cell, col)?;
            (col, col)
        }
        None => (cell, cell),
    };
    if let Some(tok) = token {
        cell_text(lang, tok_parent, tok, pal::TEXT2, true)?;
    }
    cell_text(lang, value_parent, text, if token.is_some() { pal::TEXT2 } else { pal::TEXT }, false)
}

/// A walk of the layers of an instance (shared by the frames of a run).
pub(super) type Walk = std::rc::Rc<Vec<(Value, usize)>>;

/// What every frame needs to be drawn.
pub(super) struct Ctx<'a> {
    pub(super) lang: Lang,
    pub(super) target: &'a Picked,
    /// The page's Stage setting.
    pub(super) mode: StageMode,
    /// The set of approval pills (its default variant is Estado=pendente).
    pub(super) pills: Option<u64>,
    /// The name the set of pills (the one in the file, whatever language made it) gives its text property for the area; None when it has none.
    pub(super) pill_area: Option<String>,
    pub(super) stage: StageChoice,
    /// How many areas of the Cover's pills did not read back as written and were written again.
    pub(super) area_retries: std::cell::Cell<u32>,
    /// The file's variables and styles, read once per run and only when a layer needs them (`variable.list` costs about a tenth of the
    /// budget in a big file).
    pub(super) vars: std::cell::RefCell<Option<std::rc::Rc<Vars>>>,
    pub(super) styles: std::cell::RefCell<Option<std::rc::Rc<Styles>>>,
    /// The same `variable.list` reply read with every mode of every collection (the Tokens frame): made when `vars` is.
    pub(super) modes: std::cell::RefCell<Option<std::rc::Rc<super::tokens::ModeVars>>>,
    /// The walk of the default variant's instance, made by the first frame that needs it (Anatomy, when it is made in the same run) and
    /// used by the others: the instance is made and read once.
    pub(super) base_walk: std::cell::RefCell<Option<Walk>>,
    /// The tables the content frames made: (name, rows), for the answer.
    pub(super) tables: std::cell::RefCell<Vec<(String, usize)>>,
    /// What the Tokens frame found (its counts), for the answer.
    pub(super) tokens: std::cell::RefCell<Option<Value>>,
    /// The width of the frames made in this run (1180, or wider when the Master had to grow) and the room inside them.
    pub(super) frame_w: std::cell::Cell<f64>,
    pub(super) useful: std::cell::Cell<f64>,
    /// What the Master did with the set (kept, laid out again, or made wide), for the answer.
    pub(super) master: std::cell::RefCell<Option<Value>>,
    /// Where the Anatomy put its markers (centre, guide end, stage and component boxes), for the answer.
    pub(super) markers: std::cell::RefCell<Option<Value>>,
    /// What the gallery of States and Variants drew (cells, nodes, the budget), for the answer.
    pub(super) gallery: std::cell::RefCell<Option<Value>>,
    /// What the Layouts drawing made (layers, bands, numbers outside), for the answer.
    pub(super) spacing: std::cell::RefCell<Option<Value>>,
}

impl Ctx<'_> {
    /// The variables the layers of `nodes` may be bound to: read the first time one of them is bound, kept for the run.
    pub(super) fn vars_for(&self, nodes: &[Value]) -> Result<std::rc::Rc<Vars>> {
        let mut slot = self.vars.borrow_mut();
        if let Some(v) = slot.as_ref() {
            return Ok(v.clone());
        }
        if !any_bound(nodes) {
            return Ok(std::rc::Rc::new(Vars::default()));
        }
        let list = host(self.lang, "variable.list", &json!({}))?;
        let v = std::rc::Rc::new(Vars::parse(&list));
        *self.modes.borrow_mut() = Some(std::rc::Rc::new(super::tokens::ModeVars::parse(&list)));
        *slot = Some(v.clone());
        Ok(v)
    }

    /// The variables with all their modes, from the one `variable.list` of the run (read here only if nothing has read it yet and a
    /// layer is bound to a variable).
    pub(super) fn modes_for(&self, nodes: &[Value]) -> Result<std::rc::Rc<super::tokens::ModeVars>> {
        self.vars_for(nodes)?;
        Ok(self.modes.borrow().clone().unwrap_or_default())
    }

    /// The styles, likewise.
    pub(super) fn styles_for(&self, nodes: &[Value]) -> Result<std::rc::Rc<Styles>> {
        let mut slot = self.styles.borrow_mut();
        if let Some(v) = slot.as_ref() {
            return Ok(v.clone());
        }
        if !any_styled(nodes) {
            return Ok(std::rc::Rc::new(Styles::default()));
        }
        let v = std::rc::Rc::new(Styles::parse(&host(self.lang, "style.list", &json!({}))?));
        *slot = Some(v.clone());
        Ok(v)
    }
}

pub(super) fn bold() -> Value {
    json!({"fontStyle": "Bold"})
}

fn fill_text() -> Value {
    json!({"sizingH": "fill", "textAutoResize": "height"})
}

/// The head of a frame: a small grey label with the component's name, and under it the big title of the section.
pub(super) fn header(c: &Ctx, frame: u64, title: &str) -> Result<()> {
    let lang = c.lang;
    let mut hd = FlowSpec::new("Header", USEFUL_W, "vertical");
    hd.gap = 8.0;
    let hd = flow_frame(lang, frame, hd)?;
    text_with(lang, hd, &c.target.title, LINE_SIZE, pal::TEXT2, "Component", json!({}))?;
    text_with(lang, hd, title, theme::TITLE_SIZE, pal::TEXT, "Title", bold())?;
    Ok(())
}

/// A dashed chip with a guide in it (the person writes over it on the canvas).
fn guide_chip(lang: Lang, parent: u64, s: &str) -> Result<()> {
    let mut f = FlowSpec::new("Chip", 20.0, "horizontal");
    f.padding = json!([6, 14, 6, 14]);
    f.fill = Some(pal::CARD);
    f.extra = json!({"sizingH": "hug", "cornerRadius": 14, "stroke": pal::LINE, "strokeWeight": 1, "strokeAlign": "inside", "dashes": [4, 4]});
    let chip = flow_frame(lang, parent, f)?;
    text_with(lang, chip, s, LINE_SIZE, pal::GUIDE, "Guide", json!({}))?;
    Ok(())
}

/// A small upper-case label with the grey guide under it; `right` aligns both to the right edge.
fn labelled_guide(lang: Lang, parent: u64, label: Key, hint: Key, width: Option<f64>, right: bool) -> Result<()> {
    let mut g = FlowSpec::new("Field", width.unwrap_or(20.0), "vertical");
    g.gap = 4.0;
    g.extra = if width.is_some() { json!({}) } else { json!({"sizingH": "hug", "crossAlign": if right { "max" } else { "min" }}) };
    let g = flow_frame(lang, parent, g)?;
    let label_extra = if right { json!({"textAlign": "right", "fontStyle": "Bold"}) } else { bold() };
    text_with(lang, g, &t(lang, label).to_uppercase(), LINE_SIZE, pal::TEXT2, "Label", label_extra)?;
    let mut guide = if width.is_some() { json!({"width": width, "textAutoResize": "height"}) } else { json!({}) };
    if right && let Some(o) = guide.as_object_mut() {
        o.insert("textAlign".into(), json!("right"));
    }
    text_with(lang, g, t(lang, hint), LINE_SIZE, pal::GUIDE, "Guide", guide)?;
    Ok(())
}

/// A state's circle, drawn: the colour, the symbol in white on it.
fn state_circle(lang: Lang, parent: u64, a: Approval) -> Result<u64> {
    let made = host(
        lang,
        "frame.create",
        &json!({"parent": parent, "width": STATE_D, "height": STATE_D, "name": "Circle", "select": false,
            "props": {"fill": a.colour(), "layout": "horizontal", "mainAlign": "center", "crossAlign": "center", "cornerRadius": STATE_D / 2.0, "padding": 0,
                "sizingH": "fixed", "sizingV": "fixed", "clip": false}}),
    )?;
    let id = id_of(lang, &made, "frame.create")?;
    text_with(lang, id, a.symbol(), 24.0, pal::ON_STATE, "Symbol", bold())?;
    Ok(id)
}

/// Write the area of a pill and read it back; written once more when it did not take (the answer says how many times).
fn write_area(c: &Ctx, inst: u64, property: &str, value: &str) -> Result<()> {
    let lang = c.lang;
    for attempt in 0..2 {
        host(lang, "instance.setProperties", &json!({"ids": [inst], "properties": {property: value}}))?;
        let now = host(lang, "component.properties", &json!({"id": inst}))?;
        let read = now.get("properties").and_then(Value::as_array).into_iter().flatten().find(|p| p.get("name").and_then(Value::as_str) == Some(property)).and_then(|p| p.get("value")).and_then(Value::as_str);
        if read == Some(value) {
            return Ok(());
        }
        if attempt == 0 {
            c.area_retries.set(c.area_retries.get() + 1);
        }
    }
    Ok(())
}

fn build_cover(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    // 1. Overview: the small label, the big title (the component's name) and the two chips.
    let mut hd = FlowSpec::new("Header", USEFUL_W, "vertical");
    hd.gap = 12.0;
    let hd = flow_frame(lang, frame, hd)?;
    text_with(lang, hd, t(lang, Key::CoverOverview), LINE_SIZE, pal::TEXT2, "Overview", json!({}))?;
    text_with(lang, hd, &c.target.title, 48.0, pal::TEXT, "Title", bold())?;
    let mut chips = FlowSpec::new("Chips", 20.0, "horizontal");
    chips.gap = 12.0;
    chips.extra = json!({"sizingH": "hug"});
    let chips = flow_frame(lang, hd, chips)?;
    guide_chip(lang, chips, t(lang, Key::CoverGuideVersion))?;
    guide_chip(lang, chips, t(lang, Key::CoverGuideStatus))?;
    // 2. The last designer on the left, the date on the right, on one line.
    let mut meta = FlowSpec::new("Meta", USEFUL_W, "horizontal");
    meta.extra = json!({"mainAlign": "spaceBetween", "crossAlign": "min"});
    let meta = flow_frame(lang, frame, meta)?;
    labelled_guide(lang, meta, Key::CoverLastDesigner, Key::CoverGuideDesigner, None, false)?;
    labelled_guide(lang, meta, Key::CoverUpdatedOn, Key::CoverGuideUpdated, None, true)?;
    // 3. The description.
    labelled_guide(lang, frame, Key::CoverDescription, Key::CoverGuideDescription, Some(USEFUL_W), false)?;
    // 4. The approvals: six instances of the pill component in six equal columns, the area written in each.
    text_with(lang, frame, t(lang, Key::CoverApprovals), theme::SECTION_SIZE, pal::TEXT, "Approvals title", bold())?;
    if let Some(set) = c.pills {
        let mut g = FlowSpec::new("Approvals", USEFUL_W, "grid");
        g.extra = json!({"gridColumns": 3, "gridRows": 2, "columnTracks": [350, 350, 350], "rowTracks": ["auto", "auto"], "rowGap": 16, "sizingH": "fixed"});
        g.gap = 16.0;
        let grid = flow_frame(lang, frame, g)?;
        for role in [Key::RoleDesigner, Key::RoleEngineering, Key::RoleQa, Key::RoleWriters, Key::RolePo, Key::RolePm] {
            let made = host(lang, "instance.create", &json!({"component": set, "parent": grid}))?;
            let inst = id_of(lang, &made, "instance.create")?;
            if let Some(area) = c.pill_area.as_deref() {
                write_area(c, inst, area, t(lang, role))?;
            }
        }
    }
    // 5. The legend of the four states: drawn, not components.
    let mut lg = FlowSpec::new("Legend", USEFUL_W, "horizontal");
    lg.gap = 32.0;
    lg.extra = json!({"crossAlign": "center"});
    let legend = flow_frame(lang, frame, lg)?;
    for a in Approval::ALL {
        let mut item = FlowSpec::new("Legend item", 20.0, "horizontal");
        item.gap = 10.0;
        item.extra = json!({"sizingH": "hug", "crossAlign": "center"});
        let item = flow_frame(lang, legend, item)?;
        state_circle(lang, item, a)?;
        text_with(lang, item, t(lang, a.label_key()), LINE_SIZE, pal::TEXT2, "State", json!({}))?;
    }
    Ok(())
}

fn build_changelog(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    header(c, frame, t(lang, Key::SecChangelog))?;
    let headers: Vec<String> = [Key::ChangelogVersion, Key::ChangelogDate, Key::ChangelogChange].iter().map(|k| t(lang, *k).to_string()).collect();
    let rows = vec![vec![Cv::Guide(t(lang, Key::ChangelogGuideVersion).to_string()), Cv::Guide(t(lang, Key::ChangelogGuideDate).to_string()), Cv::Guide(t(lang, Key::ChangelogGuideChange).to_string())]];
    table(lang, frame, "Changelog", &reflow::scale_columns(&CHANGELOG_COLS, c.useful.get()), &headers, &rows)?;
    Ok(())
}

/// The Component Master: the title, a line about it, the fixed warning, and the stage with the original component moved onto it.
/// Returns the moved layer's id; the frame is made as wide as the component asks.
fn build_master(c: &Ctx, frame: u64, plan: &reflow::Outcome) -> Result<u64> {
    let lang = c.lang;
    let useful = c.useful.get();
    header(c, frame, t(lang, Key::MasterTitle))?;
    text_with(lang, frame, t(lang, Key::MasterDescription), LINE_SIZE, pal::TEXT2, "Description", json!({"width": useful, "textAutoResize": "height"}))?;
    text_with(lang, frame, t(lang, Key::MasterNote), LINE_SIZE, pal::COND, "Warning", json!({"width": useful, "textAutoResize": "height"}))?;
    // The set is the person's own component: when the plugin changed it (now or in an earlier run) the Master says so.
    if plan.marked {
        text_with(lang, frame, t(lang, Key::MasterOrganized), LINE_SIZE, pal::COND, "Organized", json!({"width": useful, "textAutoResize": "height"}))?;
    }
    let mut st = FlowSpec::new("Stage", useful, "vertical");
    st.padding = json!(STAGE_PAD);
    st.fill = Some(c.stage.fill());
    st.extra = json!({"sizingH": "fill", "sizingV": "hug", "cornerRadius": 12, "stroke": pal::LINE, "strokeWeight": 1, "strokeAlign": "inside"});
    let stage = flow_frame(lang, frame, st)?;
    let moved = c.target.holder;
    host(lang, "node.reparent", &json!({"ids": [moved], "parent": stage}))?;
    *c.master.borrow_mut() = Some(plan.json());
    Ok(moved)
}

/// The set of approval pills, made inside `frame` (its name starts with a dot, which keeps it out of a published library; it is found by its
/// mark, never by name, so a set of an older Section with the name it had before is found just the same): four variants (Estado = pendente, aprovado, ressalvas, reprovado), each the pill
/// itself (a circle with the state's symbol, and the area and a "name · date" guide), with the text properties "Area" and
/// "Name · date" bound to those two texts in every variant. Returns the set's id.
fn build_approval_set(c: &Ctx, frame: u64) -> Result<u64> {
    let lang = c.lang;
    // The pills are made in a row (so the set is a row, not a column), and the set is moved into the frame.
    let mut row = FlowSpec::new("Pills", 20.0, "horizontal");
    row.gap = 24.0;
    row.extra = json!({"sizingH": "hug"});
    let row = flow_frame(lang, frame, row)?;
    let (area, name_date) = (t(lang, Key::ApprPropArea), t(lang, Key::ApprPropName));
    let (mut areas, mut guides, mut pills) = (Vec::new(), Vec::new(), Vec::new());
    for a in Approval::ALL {
        let vname = a.variant_name();
        let mut p = FlowSpec::new(&vname, PILL_W, "horizontal");
        p.gap = 12.0;
        p.padding = json!(14);
        p.fill = Some(pal::CARD);
        p.extra = json!({"cornerRadius": 28, "crossAlign": "center", "stroke": pal::LINE, "strokeWeight": 1, "strokeAlign": "inside"});
        let pill = flow_frame(lang, row, p)?;
        state_circle(lang, pill, a)?;
        let mut col = FlowSpec::new("Text", 100.0, "vertical");
        col.gap = 2.0;
        col.extra = json!({"sizingH": "fill"});
        let col = flow_frame(lang, pill, col)?;
        areas.push(text_with(lang, col, area, LINE_SIZE, pal::TEXT, "Area", json!({"fontStyle": "Bold", "sizingH": "fill", "textAutoResize": "height"}))?);
        guides.push(text_with(lang, col, t(lang, Key::CoverGuideNameDate), LINE_SIZE, pal::GUIDE, "Guide", fill_text())?);
        pills.push(pill);
    }
    for id in &pills {
        host(lang, "component.create", &json!({"ids": [id]}))?;
    }
    let combined = host(lang, "component.combineVariants", &json!({"ids": pills}))?;
    let set = id_of(lang, &combined, "component.combineVariants")?;
    host(lang, "node.reparent", &json!({"ids": [set], "parent": frame}))?;
    host(lang, "edit.delete", &json!({"ids": [row]}))?;
    // The property goes on the first variant's text and is bound to the other three.
    host(lang, "component.addProperty", &json!({"id": set, "name": area, "type": "text", "default": area, "layer": areas[0]}))?;
    host(lang, "component.addProperty", &json!({"id": set, "name": name_date, "type": "text", "default": t(lang, Key::CoverGuideNameDate), "layer": guides[0]}))?;
    for (a, g) in areas.iter().zip(&guides).skip(1) {
        host(lang, "component.bindProperty", &json!({"layer": a, "property": area, "field": "characters"}))?;
        host(lang, "component.bindProperty", &json!({"layer": g, "property": name_date, "field": "characters"}))?;
    }
    host(lang, "node.set", &json!({"ids": [set], "props": {"name": t(lang, Key::KitSetName), "pluginData": {APPROVAL_SET_MARK: "1"}}}))?;
    Ok(set)
}

// ---------------------------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------------------------

/// Whether another Section of the component already holds the Component Master.
fn other_has_master(lang: Lang, page: &PageInfo, key: &str, this: Option<u64>) -> Result<bool> {
    for s in sections_of(page, key).into_iter().filter(|s| Some(*s) != this) {
        let (_, frames) = read_frames(lang, s)?;
        if frames.iter().any(|f| f.sec == Sec::Master) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The frames that are not there, that no run made and that "Update" does not name: the Section is not new, the frame is not permanent (those
/// are made again whenever they are gone) and it is not the Master. A bit for each frame of `Sec::ALL`, by its index.
pub fn missing_mask(exists: &dyn Fn(Sec) -> bool, created: bool, update: Update) -> u16 {
    if created {
        return 0;
    }
    Sec::ALL.iter().filter(|s| !s.permanent() && **s != Sec::Master && !exists(**s) && !update.sections().contains(s)).fold(0, |m, s| m | (1 << s.index()))
}

/// Whether a run makes the frame `sec`: the Component Master whenever it is allowed; a frame that is not there yet when the Section is new
/// (a permanent one also when it was deleted by hand); and a frame that is there only when the update names it, never a permanent one.
/// Cover, Changelog and Support, once made, are never touched by any option of "Update", nor by Generate.
pub fn frame_is_made(sec: Sec, exists: bool, created: bool, update: Update, master_allowed: bool) -> bool {
    match (exists, sec) {
        (_, Sec::Master) => master_allowed,
        (false, s) if s.permanent() => true,
        (false, s) => created || update.sections().contains(&s),
        (true, s) if s.permanent() => false,
        (true, s) => update.sections().contains(&s),
    }
}

/// Make or update the document: the Section of the selected component and its frames.
///
/// Parameters: `update` (a key of [`Update`], default `all`) says which frames a rerun replaces; `new_section` (bool) makes another
/// Section beside the first, with every frame but the Component Master. The first generation ignores `update`.
pub fn generate(input: Input) -> Result<Value> {
    // 1. The page's settings come first: they give the language, even for an error.
    let page = page_info()?;
    let lang = page.settings.lang.as_deref().map_or(Lang::En, Lang::parse);
    if input.selection.is_empty() {
        return Err(t(lang, Key::ErrNoSelection).to_string());
    }
    let target = resolve(lang, &input.selection)?;
    // A component whose Section was deleted is gone with it: an instance may still name it.
    let holder = host(lang, "node.get", &json!({"id": target.holder, "fields": ["type", "pluginData"]}))?;
    if holder.is_null() {
        return Err(t(lang, Key::ErrMainGone).to_string());
    }
    // The approval pill is the plugin's own support component: documenting it (a pill of the Cover is an instance of it) would make a
    // Section of it and take the set out of the support frame, so the Cover's pills would no longer be the ones of the Section.
    if mark_of(&holder, APPROVAL_SET_MARK).is_some() {
        return Err(t(lang, Key::ErrSupportComponent).to_string());
    }
    let comp_key = target.key.to_string();
    let name = target.title.clone();
    let update = input.str("update").and_then(Update::from_key).unwrap_or(Update::All);
    let new_section = input.params.get("new_section").and_then(Value::as_bool).unwrap_or(false);
    // 2. Which Section: the first one of the component, unless a new one is asked for (or there is none).
    let mut found = find_section(&page, &comp_key);
    // A Section an earlier build made for a variant is marked with the variant's id: it is the set's Section now (its marks are rewritten below).
    let migrated = match (found, target.legacy_key) {
        (None, Some(old)) => find_section(&page, &old.to_string()).inspect(|id| found = Some(*id)).map(|_| old),
        _ => None,
    };
    let first_run = found.is_none();
    let beside = found.is_some() && new_section;
    let section_id = if beside { None } else { found };
    let mut warnings: Vec<Warn> = Vec::new();
    if first_run && has_legacy_frame(&page, &comp_key) {
        warnings.push(Warn::LegacyDoc);
    }
    // A Section goes to the right of everything on the page (+ 256), at y = 256; "New Section" at the y of the reference Section (the
    // first of the component), so the new column stands beside the old one.
    let boxes: Vec<(f64, f64, f64, f64)> = page.children.iter().filter_map(bounds).map(|b| (b.x, b.y, b.w, b.h)).collect();
    let beside_y = match found {
        Some(id) if beside => page.children.iter().find(|c| c.get("id").and_then(Value::as_u64) == Some(id)).and_then(bounds).map(|b| b.y),
        _ => None,
    };
    let calls0 = HOST_CALLS.load(Ordering::Relaxed);
    let mut ctx = Ctx { lang, target: &target, mode: page.settings.stage_mode(), pills: None, pill_area: None, stage: StageChoice { stage: Stage::Light, lowest: None, auto: false }, area_retries: std::cell::Cell::new(0), vars: std::cell::RefCell::new(None), styles: std::cell::RefCell::new(None), modes: std::cell::RefCell::new(None), tables: std::cell::RefCell::new(Vec::new()), tokens: std::cell::RefCell::new(None), markers: std::cell::RefCell::new(None), gallery: std::cell::RefCell::new(None), spacing: std::cell::RefCell::new(None), frame_w: std::cell::Cell::new(FRAME_W), useful: std::cell::Cell::new(USEFUL_W), master: std::cell::RefCell::new(None), base_walk: std::cell::RefCell::new(None) };
    // 2b. The set of approval pills: ONE for the whole document. It lives in the Support frame, outside every Section; a new Section
    // reuses the oldest one there is, and only when there is none (first generation, or a Support frame that was deleted) a Support frame
    // is made, to the right of everything, before the Section. A rerun never makes one.
    let (support_set, support_count) = find_support(lang, &page)?;
    let mut boxes = boxes;
    let mut support_made: Option<Value> = None;
    ctx.pills = support_set;
    if support_set.is_none() && section_id.is_none() {
        let (x, y) = section_origin(&boxes, None);
        let (frame, set, w, h) = build_support(&ctx, x, y)?;
        boxes.push((x, y, w, h));
        ctx.pills = Some(set);
        support_made = Some(json!({"frame": frame, "set": set, "x": x, "y": y, "w": w, "h": h}));
    }
    // The pills are written by the names the set of the FILE gives its properties (it may have been made in another language).
    if let Some(set) = ctx.pills {
        ctx.pill_area = read_area_property(lang, set)?;
        if ctx.pill_area.is_none() {
            warnings.push(Warn::ApprovalProperty);
        }
    }
    // 3. The Section: the one that is there, or a new one right of everything.
    let (section, sx, sy, existing, created) = match section_id {
        Some(id) => {
            let (sb, frames) = read_frames(lang, id)?;
            if migrated.is_some() {
                host(lang, "node.set", &json!({"ids": [id], "props": {"pluginData": {SECTION_MARK: comp_key}}}))?;
                for f in &frames {
                    host(lang, "node.set", &json!({"ids": [f.id], "props": {"pluginData": {REPORT_KEY: comp_key}}}))?;
                }
            }
            (id, sb.x, sb.y, frames, false)
        }
        None => {
            let (x, y) = section_origin(&boxes, beside_y);
            let made = host(
                lang,
                "node.create",
                &json!({"type": "section", "name": tf(lang, Key::SectionName, &[("name", &name)]), "x": x, "y": y, "width": FRAME_W + 2.0 * SECTION_PAD, "height": 400, "select": false,
                    "props": {"fill": pal::SECTION, "pluginData": {SECTION_MARK: comp_key}}}),
            )?;
            (id_of(lang, &made, "node.create")?, x, y, Vec::new(), true)
        }
    };
    let old = |sec: Sec| existing.iter().find(|e| e.sec == sec);
    // 4. What is made. The first generation and a new Section make everything (a new Section: not the Master); a rerun replaces
    // what "Update" names and makes again any permanent frame that is missing. The Master is made again on every run when this
    // Section has one; a Section that has none makes it only when no other Section of the component holds the original.
    let master_allowed = !beside && (old(Sec::Master).is_some() || !other_has_master(lang, &page, &comp_key, section_id)?);
    let will_make = |sec: Sec| frame_is_made(sec, old(sec).is_some(), created, update, master_allowed);
    // 4b. Frames that are gone and that Update does not name stay gone: the run says so.
    let missing = missing_mask(&|s| old(s).is_some(), created, update);
    if missing != 0 {
        warnings.push(Warn::FrameMissing(missing));
    }
    // 4c. A Cover that is made again needs the set of approval pills: when the support frame is gone (a rerun never found one) it is made now,
    // the way the first generation makes it. When that fails the Cover is made without pills and the run says so.
    if ctx.pills.is_none() && will_make(Sec::Cover) && section_id.is_some() {
        let (x, y) = section_origin(&boxes, None);
        match build_support(&ctx, x, y) {
            Ok((frame, set, w, h)) => {
                boxes.push((x, y, w, h));
                ctx.pills = Some(set);
                support_made = Some(json!({"frame": frame, "set": set, "x": x, "y": y, "w": w, "h": h}));
                ctx.pill_area = read_area_property(lang, set)?;
            }
            Err(_) => warnings.push(Warn::SupportMissing),
        }
    }
    // 5b. The width of the frames. They are all 1180, with the Master's room for the set at 988; a set that is wider is given auto layout with
    // wrap (the host lays its variants out in lines, the set is 988 wide). Only when not even one variant fits in a line, or the host does not
    // keep the layout, the Master grows and every frame made in this run grows with it, to the same width, so the column stays straight.
    let master_plan = if will_make(Sec::Master) { Some(reflow::organize(lang, target.holder, target.set.is_some())?) } else { None };
    if let Some(p) = &master_plan {
        ctx.frame_w.set(p.frame_w);
        ctx.useful.set(p.frame_w - 2.0 * FRAME_PAD);
        if p.failed.is_some() {
            warnings.push(Warn::ReflowFailed);
        }
        if p.wide {
            warnings.push(Warn::WideMaster);
        }
    }
    // 6. The stage of the Master, over every variant (a sample of them in a big set), when the Master is made now.
    let stage_sampled: std::cell::Cell<Option<(usize, usize)>> = std::cell::Cell::new(None);
    if will_make(Sec::Master) {
        ctx.stage = choose_stage(lang, &target, page.settings.stage_mode(), &stage_sampled)?;
        if ctx.stage.too_low() {
            warnings.push(Warn::NoStage);
        }
    }
    // 7. The frames, in order. A frame that is made again takes the place of the old one; a frame that was not there is placed below.
    let mut report: Vec<Value> = Vec::new();
    let mut master_moved: Option<u64> = None;
    let mut failed: Vec<&str> = Vec::new();
    // Where every frame stands now (inside the Section) and what it was made as.
    let mut placed: Vec<Placed> = existing.iter().map(|e| Placed { sec: e.sec, id: e.id, x: e.x, y: e.y, w: e.w, h: e.h }).collect();
    let mut actions: Vec<(Sec, &str)> = Vec::new();
    let mut fresh: Vec<Sec> = Vec::new();
    for sec in Sec::ALL {
        let (id, action): (u64, &str) = if will_make(sec) {
            let frame_name = if sec == Sec::Master { tf(lang, Key::MasterFrameName, &[("name", &name)]) } else { tf(lang, Key::SectionFrameName, &[("title", t(lang, sec.title_key())), ("name", &name)]) };
            let replaced = old(sec).is_some();
            // The old Master is deleted only AFTER the original component is in the new one (deleting a frame deletes what is in it).
            if sec != Sec::Master && let Some(e) = old(sec) {
                host(lang, "edit.delete", &json!({"ids": [e.id]}))?;
            }
            let mut frame = new_frame(lang, section, &frame_name, &comp_key, sec, ctx.frame_w.get())?;
            // A table that cannot be made leaves its frame with the error in place of the content, and the run goes on.
            let drawn: Option<Result<()>> = match sec {
                Sec::Properties => Some(tables::build_properties(&ctx, frame)),
                Sec::Anatomy => Some(tables::build_anatomy(&ctx, frame).map(|(no_stage, dropped)| {
                    if no_stage && !warnings.contains(&Warn::NoStage) {
                        warnings.push(Warn::NoStage);
                    }
                    if dropped > 0 {
                        warnings.push(Warn::MarkersDropped(dropped));
                    }
                })),
                Sec::Variants => Some(tables::build_variants(&ctx, frame)),
                Sec::Tokens => Some(tokens::build_tokens(&ctx, frame)),
                Sec::Layouts => Some(spacing::build_layouts(&ctx, frame).map(|dropped| {
                    if dropped > 0 {
                        warnings.push(Warn::SpacingDropped(dropped));
                    }
                })),
                _ => None,
            };
            #[cfg(feature = "test-hooks")]
            let drawn = if drawn.is_some() && input.str("fail_table") == Some(sec.key()) { Some(Err("forced failure (test hook)".to_string())) } else { drawn };
            if let Some(Err(reason)) = drawn {
                host(lang, "edit.delete", &json!({"ids": [frame]}))?;
                frame = new_frame(lang, section, &frame_name, &comp_key, sec, ctx.frame_w.get())?;
                header(&ctx, frame, t(lang, sec.title_key()))?;
                text_with(lang, frame, &tf(lang, Key::TableFailed, &[("reason", &reason)]), LINE_SIZE, pal::BAD, "Error", json!({"width": USEFUL_W, "textAutoResize": "height"}))?;
                failed.push(sec.key());
            }
            match sec {
                Sec::Cover => build_cover(&ctx, frame)?,
                Sec::Changelog => build_changelog(&ctx, frame)?,
                Sec::Master => {
                    let moved = build_master(&ctx, frame, master_plan.as_ref().ok_or_else(|| "the Master was made without a plan".to_string())?)?;
                    if let Some(e) = old(sec) {
                        host(lang, "edit.delete", &json!({"ids": [e.id]}))?;
                    }
                    master_moved = Some(moved);
                }
                _ => {}
            }
            let (w, h) = settled_size(lang, frame)?;
            match old(sec) {
                // The new frame takes the old one's place; what is under the old one moves by the difference in height.
                Some(e) => {
                    // Where it stands now: an earlier frame of this run may have moved it (its own height changed).
                    let (y0, h0) = placed.iter().find(|p| p.sec == sec).map_or((e.y, e.h), |p| (p.y, p.h));
                    shift_below(&mut placed, sec, y0, h0, h);
                    placed.retain(|p| p.sec != sec);
                    placed.push(Placed { sec, id: frame, x: e.x, y: y0, w, h });
                }
                None => {
                    placed.retain(|p| p.sec != sec);
                    placed.push(Placed { sec, id: frame, x: FRAME_X, y: SECTION_PAD, w, h });
                    fresh.push(sec);
                }
            }
            (frame, if replaced { "replaced" } else { "created" })
        } else if old(sec).is_some() {
            (old(sec).map_or(0, |e| e.id), "kept")
        } else {
            continue;
        };
        if fresh.contains(&sec) && !placed.iter().any(|p| p.sec == sec) {
            let (w, h) = settled_size(lang, id)?;
            placed.push(Placed { sec, id, x: FRAME_X, y: SECTION_PAD, w, h });
        }
        actions.push((sec, action));
    }
    // 8. The frames that were not there go in their place of the column: the whole column of a new Section is stacked from the top;
    // a frame made again in a Section that lacked it goes under the frame above it, and the ones under it move down.
    let all_fresh = placed.iter().all(|p| fresh.contains(&p.sec));
    if all_fresh {
        let ordered: Vec<Sec> = Sec::ALL.into_iter().filter(|s| placed.iter().any(|p| p.sec == *s)).collect();
        let heights: Vec<f64> = ordered.iter().filter_map(|s| placed.iter().find(|p| p.sec == *s).map(|p| p.h)).collect();
        for (sec, y) in ordered.iter().zip(column_ys(&heights)) {
            if let Some(p) = placed.iter_mut().find(|p| p.sec == *sec) {
                p.y = y;
            }
        }
    } else {
        for sec in Sec::ALL.into_iter().filter(|s| fresh.contains(s)) {
            let Some(me) = placed.iter().find(|p| p.sec == sec).cloned() else { continue };
            let above = Sec::ALL.iter().take(sec.index()).rev().find_map(|s| placed.iter().find(|p| p.sec == *s));
            let y = above.map_or(SECTION_PAD, |a| a.bottom() + FRAME_GAP);
            // Room for it: what stood at or under that line moves so that the frame right under starts one gap below this one (a hole the
            // deleted frame left is closed, and a frame of another height is made room for), the others by the same amount.
            let first = placed.iter().filter(|p| p.sec != sec && p.y >= y - 0.5).map(|p| p.y).fold(f64::INFINITY, f64::min);
            if first.is_finite() {
                let delta = y + me.h + FRAME_GAP - first;
                for p in placed.iter_mut().filter(|p| p.sec != sec && p.y >= y - 0.5) {
                    p.y += delta;
                }
            }
            if let Some(p) = placed.iter_mut().find(|p| p.sec == sec) {
                p.y = y;
            }
        }
    }
    // Every frame stands where `placed` says (the moves are written only when a frame is not already there).
    for p in &placed {
        let before = existing.iter().find(|e| e.sec == p.sec && e.id == p.id);
        if before.is_none_or(|e| (e.x - p.x).abs() > 0.5 || (e.y - p.y).abs() > 0.5) {
            host(lang, "node.set", &json!({"ids": [p.id], "props": {"x": p.x, "y": p.y}}))?;
        }
    }
    for (sec, action) in &actions {
        if let Some(p) = placed.iter().find(|p| p.sec == *sec) {
            report.push(json!({"key": sec.key(), "id": p.id, "x": p.x, "y": p.y, "w": p.w, "h": p.h, "action": action}));
        }
    }
    // 9. The Section is as wide as its widest frame and as tall as the column, with the room around it.
    let (section_w, section_h) = section_size(&placed);
    let tallest = placed.iter().map(|p| p.h).fold(0.0, f64::max);
    host(lang, "node.set", &json!({"ids": [section], "props": {"width": section_w, "height": section_h}}))?;
    let export_result = if created {
        match call("export.add", &json!({"ids": [section], "setting": {"size": "1x", "suffix": "", "format": "svg"}})) {
            Ok(_) => "ok".to_string(),
            Err(e) => e,
        }
    } else {
        "kept".to_string()
    };
    if !failed.is_empty() {
        warnings.push(Warn::Failed);
    }
    warnings.sort_by_key(|w| Warn::ALL.iter().position(|x| std::mem::discriminant(x) == std::mem::discriminant(w)));
    Ok(json!({
        "name": name, "frame": section, "section": section, "created": created, "rerun": !first_run, "new_section": beside, "update": update.key(), "x": sx, "y": sy,
        "frame_size": {"w": section_w}, "view_w": section_w, "tallest": tallest, "section_size": {"w": section_w, "h": section_h}, "frames": report, "master_moved": master_moved,
        "warnings": warnings.iter().map(|w| w.code()).collect::<Vec<_>>(), "export": export_result, "lang": lang.code(), "component": target.key, "migrated_from": migrated, "missing": Sec::ALL.iter().filter(|s| missing & (1 << s.index()) != 0).map(|s| s.key()).collect::<Vec<_>>(), "holder": target.holder,
        "stage": {"sampled": stage_sampled.get().map(|(a, b)| json!([a, b])), "name": ctx.stage.name(), "dark": ctx.stage.stage == Stage::Dark, "auto": ctx.stage.auto, "lowest": ctx.stage.lowest, "fill": ctx.stage.fill()},
        "failed": failed, "master": ctx.master.borrow().clone(), "tokens": ctx.tokens.borrow().clone(), "anatomy": ctx.markers.borrow().clone(), "gallery": ctx.gallery.borrow().clone(), "spacing": ctx.spacing.borrow().clone(), "tables": ctx.tables.borrow().iter().map(|(n, r)| json!({"name": n, "rows": r})).collect::<Vec<_>>(), "pill_set": ctx.pills, "support": {"sets_found": support_count, "made": support_made, "set": ctx.pills}, "area_retries": ctx.area_retries.get(), "host_calls": HOST_CALLS.load(Ordering::Relaxed) - calls0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Rgba {
        contrast::parse_hex(s, 1.0).unwrap_or(contrast::LIGHT_STAGE)
    }

    fn text_prop(name: &str) -> Value {
        json!({"name": name, "type": "text", "default": name})
    }

    #[test]
    fn the_pill_is_written_by_the_name_the_set_of_the_file_has_not_by_the_language_of_the_run() {
        let variant = json!({"name": "Estado", "type": "variant", "options": ["pendente"], "default": "pendente"});
        // A set made in English, read in a run in Portuguese (and the other way round): the name that is there is the one to write.
        for lang in Lang::ALL {
            let _ = lang;
            assert_eq!(area_property(&[variant.clone(), text_prop("Area"), text_prop("Name · date")]), Some("Area".to_string()));
            assert_eq!(area_property(&[variant.clone(), text_prop("Área"), text_prop("Nome · data")]), Some("Área".to_string()));
        }
        // The order of the properties does not matter.
        assert_eq!(area_property(&[text_prop("Name · date"), variant.clone(), text_prop("Area")]), Some("Area".to_string()));
    }

    #[test]
    fn a_set_with_no_area_property_gives_none_and_the_run_warns_instead_of_failing() {
        let variant = json!({"name": "Estado", "type": "variant"});
        assert_eq!(area_property(&[variant.clone(), text_prop("Name · date")]), None);
        // A property named Area that is not text (a variant axis, say) is not the area.
        assert_eq!(area_property(&[json!({"name": "Area", "type": "variant"})]), None);
        assert_eq!(area_property(&[]), None);
        assert_eq!(Warn::ApprovalProperty.code(), "approval_property_missing");
        for lang in Lang::ALL {
            let text = Warn::ApprovalProperty.text(lang);
            assert!(!text.is_empty() && text.chars().count() <= 120, "{text}");
        }
    }

    #[test]
    fn the_sections_come_in_the_order_of_the_frames_with_their_own_keys() {
        let keys: Vec<&str> = Sec::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(keys, ["cover", "master", "properties", "anatomy", "variants", "layouts", "tokens", "changelog"]);
        for s in Sec::ALL {
            assert_eq!(Sec::from_key(s.key()), Some(s));
        }
        // A Section made before the shared Support frame has one inside it: it is still recognised (by its mark), though no run makes it.
        assert_eq!(Sec::from_key("kit"), Some(Sec::Kit));
        assert!(!Sec::ALL.contains(&Sec::Kit));
        assert_eq!(Sec::from_key("nope"), None);
    }

    #[test]
    fn two_frames_are_made_once_and_the_others_are_the_ones_a_rerun_replaces() {
        let permanent: Vec<Sec> = Sec::ALL.into_iter().filter(|s| s.permanent()).collect();
        assert_eq!(permanent, [Sec::Cover, Sec::Changelog], "the Master is not among them: it is made again on every run");
        assert_eq!(Update::All.sections(), [Sec::Properties, Sec::Anatomy, Sec::Variants, Sec::Layouts, Sec::Tokens]);
        assert!(Update::ALL.iter().all(|u| u.sections().iter().all(|s| !s.permanent())), "no choice of Update replaces a permanent frame");
    }

    #[test]
    fn every_option_of_update_names_its_frames() {
        let keys: Vec<&str> = Update::ALL.iter().map(|u| u.key()).collect();
        assert_eq!(keys, ["all", "tables", "tokens", "properties", "anatomy", "variants", "layouts"]);
        assert_eq!(Update::Tables.sections(), [Sec::Properties, Sec::Anatomy, Sec::Variants, Sec::Layouts]);
        assert_eq!(Update::Tokens.sections(), [Sec::Tokens]);
        for u in Update::ALL {
            assert_eq!(Update::from_key(u.key()), Some(u));
        }
        assert_eq!(Update::from_key("cover"), None);
    }

    #[test]
    fn the_four_states_have_their_property_value_symbol_and_colour() {
        let all: Vec<(String, &str, &str)> = Approval::ALL.iter().map(|a| (a.variant_name(), a.symbol(), a.colour())).collect();
        assert_eq!(
            all,
            [
                ("Estado=pendente".to_string(), "…", "#6E6E75"),
                ("Estado=aprovado".to_string(), "✓", "#167A4C"),
                ("Estado=ressalvas".to_string(), "!", "#9A6410"),
                ("Estado=reprovado".to_string(), "✕", "#C13B30"),
            ]
        );
    }

    fn frame(sec: Sec, y: f64, w: f64, h: f64) -> Placed {
        Placed { sec, id: sec.index() as u64 + 1, x: FRAME_X, y, w, h }
    }

    #[test]
    fn the_frames_are_in_one_column_with_64_around_and_between_them() {
        assert_eq!((FRAME_W, FRAME_PAD, USEFUL_W, FRAME_GAP, SECTION_PAD, SECTION_RIGHT_GAP, ORIGIN_Y), (1180.0, 48.0, 1084.0, 64.0, 64.0, 256.0, 256.0));
        let heights = [615.0, 1359.0, 537.0, 723.0, 1505.0, 451.0, 1441.0, 279.0];
        let ys = column_ys(&heights);
        assert_eq!(ys[0], SECTION_PAD);
        assert!(ys.windows(2).zip(&heights).all(|(w, h)| w[1] - w[0] - h == FRAME_GAP));
        for (i, s) in Sec::ALL.iter().enumerate() {
            assert_eq!(s.index(), i);
        }
        // The Section: 64 + the widest frame + 64 wide; 64 + the sum of the heights + 7 x 64 + 64 tall (eight frames).
        let frames: Vec<Placed> = Sec::ALL.iter().zip(&heights).zip(&ys).map(|((s, h), y)| frame(*s, *y, FRAME_W, *h)).collect();
        assert_eq!(section_size(&frames), (64.0 + 1180.0 + 64.0, 64.0 + heights.iter().sum::<f64>() + 7.0 * 64.0 + 64.0));
        assert_eq!(section_size(&[]), (128.0, 128.0));
    }

    fn page(boxes: &[(f64, f64, f64, f64)]) -> Vec<(f64, f64, f64, f64)> {
        boxes.to_vec()
    }

    #[test]
    fn the_first_section_goes_right_of_everything_at_y_256_with_256_between() {
        // An empty page: x = 0. A page with one Section 1308 wide at x = 0: its right edge + 256.
        assert_eq!(section_origin(&[], None), (0.0, 256.0));
        let first = (0.0, 256.0, 1308.0, 6000.0);
        assert_eq!(section_origin(&page(&[first]), None), (1308.0 + 256.0, 256.0));
        // The Section of another component goes to the right of both.
        let second = (1564.0, 256.0, 1308.0, 4000.0);
        assert_eq!(section_origin(&page(&[first, second]), None), (1564.0 + 1308.0 + 256.0, 256.0));
    }

    #[test]
    fn a_new_section_of_the_same_component_goes_beside_the_old_one_at_its_y() {
        let old = (0.0, 256.0, 1308.0, 6000.0);
        let other = (1564.0, 256.0, 2000.0, 4000.0); // a wide Section of another component
        // At the y of the reference, to the right of everything (the other component's Section included).
        assert_eq!(section_origin(&page(&[old, other]), Some(256.0)), (1564.0 + 2000.0 + 256.0, 256.0));
        // The reference is not at 256 (the person moved it): the new column is level with it.
        assert_eq!(section_origin(&page(&[(0.0, 900.0, 1308.0, 6000.0)]), Some(900.0)), (1308.0 + 256.0, 900.0));
    }

    #[test]
    fn the_support_frame_and_a_frame_of_version_0_10_0_are_something_to_clear_too() {
        let legacy = (0.0, 0.0, 3000.0, 1200.0); // "Documentation: X"
        assert_eq!(section_origin(&page(&[legacy]), None), (3000.0 + 256.0, 256.0));
        // The first generation in a document with no set: the Support frame (1368 wide) goes right of everything, the Section right of it.
        let support = (3256.0, 256.0, 1368.0, 331.0);
        assert_eq!(section_origin(&page(&[legacy, support]), None), (3256.0 + 1368.0 + 256.0, 256.0));
    }

    #[test]
    fn a_rerun_that_changes_the_height_of_a_middle_frame_moves_the_ones_under_it_down_by_the_difference() {
        let heights = [615.0, 1359.0, 537.0, 723.0, 1505.0, 451.0, 1441.0, 279.0];
        let ys = column_ys(&heights);
        let mut frames: Vec<Placed> = Sec::ALL.iter().zip(&heights).zip(&ys).map(|((s, h), y)| frame(*s, *y, FRAME_W, *h)).collect();
        let before: Vec<f64> = frames.iter().map(|f| f.y).collect();
        // Anatomy (index 3) goes from 723 to 900: Variants and everything under it move 177 down; Cover, Master and Properties stay.
        let anatomy = frames[3].clone();
        shift_below(&mut frames, Sec::Anatomy, anatomy.y, anatomy.h, 900.0);
        for (i, f) in frames.iter().enumerate() {
            let want = if i > 3 { before[i] + 177.0 } else { before[i] };
            assert_eq!(f.y, want, "{:?}", f.sec);
        }
        // The same height: nothing moves.
        let snapshot = frames.clone();
        shift_below(&mut frames, Sec::Anatomy, anatomy.y, 723.0, 723.0);
        assert_eq!(frames, snapshot);
        // A frame the person moved UP over the changed one (its top above the old bottom) is left alone.
        let mut moved = vec![frame(Sec::Properties, 300.0, FRAME_W, 200.0), frame(Sec::Anatomy, 64.0, FRAME_W, 723.0), frame(Sec::Variants, 851.0, FRAME_W, 100.0)];
        shift_below(&mut moved, Sec::Anatomy, 64.0, 723.0, 800.0);
        assert_eq!((moved[0].y, moved[2].y), (300.0, 928.0));
    }

    #[test]
    fn the_section_follows_the_widest_frame() {
        // A Master that had to grow (a variant too wide for one line) makes every frame of the Section as wide: 1048 + 192.
        let w = reflow::Plan::Wide { width: 1048.0, organize: None }.frame_width();
        assert_eq!(w, 1240.0);
        let frames = [frame(Sec::Cover, 64.0, w, 615.0), frame(Sec::Master, 743.0, w, 900.0), frame(Sec::Properties, 1707.0, w, 500.0)];
        assert_eq!(section_size(&frames), (64.0 + 1240.0 + 64.0, 1707.0 + 500.0 + 64.0));
    }

    #[test]
    fn the_columns_of_the_table_fill_the_useful_width() {
        assert_eq!(USEFUL_W, 1084.0);
        assert_eq!(CHANGELOG_COLS.iter().sum::<f64>(), USEFUL_W);
        assert_eq!(CHANGELOG_COLS, [160.0, 180.0, 744.0]);
    }

    #[test]
    fn the_section_is_found_by_its_mark_among_the_pages_own_children() {
        let page = PageInfo {
            id: 1,
            settings: Settings::default(),
            children: vec![
                json!({"id": 5, "pluginData": {"handoff-report-section": "10"}}),
                json!({"id": 6, "pluginData": {"handoff-report": "10"}}),
                json!({"id": 7, "pluginData": {"handoff-report-section": "11"}}),
                json!({"id": 8, "pluginData": {"handoff-report-section": "10"}}),
            ],
        };
        assert_eq!(find_section(&page, "10"), Some(5), "the first Section of the component is the one a rerun works on");
        assert_eq!(sections_of(&page, "10"), [5, 8]);
        assert_eq!(find_section(&page, "11"), Some(7));
        assert_eq!(find_section(&page, "12"), None);
        assert!(has_legacy_frame(&page, "10"), "a loose frame with the old mark is a 0.10.0 documentation");
        assert!(!has_legacy_frame(&page, "11"));
    }

    // ---- the automatic stage over every variant ----------------------------------------------

    fn only_elements(cols: &[&str]) -> Judged {
        Judged { backdrop: None, elements: cols.iter().map(|c| hex(c)).collect() }
    }

    /// A variant as the demo's Button has it: the fill of its root (if any) and the colours of the layers under it.
    fn v(backdrop: Option<&str>, elements: &[&str]) -> Judged {
        Judged { backdrop: backdrop.map(hex), elements: elements.iter().map(|c| hex(c)).collect() }
    }

    /// The Button of the DS Base Demo, read from the file: 5 states × 3 sizes for each of its four types (the colours are those of the
    /// root's fill and of the layers under it).
    fn demo_button(kind: &str) -> Vec<Judged> {
        let rows: &[(Option<&str>, &[&str])] = match kind {
            // Primary: the fill changes with the state; the text is the same.
            "Primary" => &[(Some("#C6005C"), &["#FDF2F8"]), (Some("#B10652"), &["#FDF2F8"]), (Some("#9E0749"), &["#FDF2F8"]), (Some("#C6005C"), &["#FDF2F8"]), (Some("#C6005C"), &["#FDF2F8"])],
            "Secondary" => &[(Some("#F4F4F5"), &["#18181B"]), (Some("#E7E7E8"), &["#18181B"]), (Some("#DADADB"), &["#18181B"]), (Some("#F4F4F5"), &["#18181B"]), (Some("#F4F4F5"), &["#18181B"])],
            // Outline: no fill, except on Hover and Pressed.
            "Outline" => &[(None, &["#090B0C"]), (Some("#F1F3F3"), &["#161B1D"]), (Some("#DADADB"), &["#090B0C"]), (None, &["#090B0C"]), (None, &["#090B0C"])],
            _ => &[(Some("#E7000B"), &["#FFFFFF"]), (Some("#D10C0F"), &["#FFFFFF"]), (Some("#BD0208"), &["#FFFFFF"]), (Some("#E7000B"), &["#FFFFFF"]), (Some("#E7000B"), &["#FFFFFF"])],
        };
        (0..3).flat_map(|_| rows.iter().map(|(b, e)| v(*b, e))).collect()
    }

    fn near(l: Option<f64>, want: f64) -> bool {
        l.is_some_and(|l| (l - want).abs() < 0.001)
    }

    #[test]
    fn the_demo_buttons_outline_without_its_hover_and_pressed_asks_for_the_light_stage() {
        // Enabled, Disabled and Focus of the Outline: no fill, dark text: the light stage, at 15.66:1.
        let quiet: Vec<Judged> = demo_button("Outline").into_iter().filter(|j| j.backdrop.is_none()).collect();
        let pick = pick_union(&quiet, None);
        assert_eq!(pick.stage, Stage::Light);
        assert!(near(pick.lowest, 15.656), "{pick:?}");
    }

    #[test]
    fn a_big_set_is_sampled_one_variant_per_value_of_each_axis_and_a_small_one_is_read_whole() {
        let names: Vec<String> = ["Primary", "Secondary", "Outline", "Destructive"]
            .iter()
            .flat_map(|t| ["Small", "Medium", "Large"].iter().flat_map(move |s| ["Enabled", "Hover", "Pressed", "Disabled", "Focus"].iter().map(move |st| format!("Type={t}, Size={s}, State={st}"))))
            .collect();
        let variants: Vec<Variant> = names.iter().enumerate().map(|(i, n)| Variant { id: 100 + i as u64, name: n.clone(), w: 10.0, h: 10.0 }).collect();
        let info = SetInfo::new(1, "Button".into(), variants);
        let default = variants::parse_name("Type=Primary, Size=Medium, State=Enabled");
        assert_eq!(info.variants.len(), 60);
        // 60 variants > 30: the default plus 3 + 2 + 4 more = 10.
        let sample = stage_sample(&info, &default, MAX_STAGE_VARIANTS);
        assert_eq!(sample.len(), 10);
        let name_of = |id: u64| info.variants.iter().find(|v| v.id == id).map(|v| v.name.clone()).unwrap_or_default();
        assert_eq!(name_of(sample[0]), "Type=Primary, Size=Medium, State=Enabled");
        assert!(sample.iter().any(|id| name_of(*id) == "Type=Outline, Size=Medium, State=Enabled"));
        assert!(sample.iter().any(|id| name_of(*id) == "Type=Primary, Size=Medium, State=Focus"));
        // At most `max`: read whole.
        assert_eq!(stage_sample(&info, &default, 60).len(), 60);
        // Even the axes' variants may be too many: cut evenly to `max`.
        assert_eq!(stage_sample(&info, &default, 5).len(), 5);
    }

    #[test]
    fn the_whole_outline_of_the_demo_button_gets_the_mid_grey_stage_at_3_21_to_1() {
        // Hover (#F1F3F3) and Pressed (#DADADB) are as pale as the light stage and the text #090B0C is as dark as the dark one:
        // only the mid-grey #777777 keeps all of them apart, at 3.21:1, over the 3:1 line: no warning.
        let pick = pick_union(&demo_button("Outline"), None);
        assert_eq!(pick.stage, Stage::Mid);
        assert!(near(pick.lowest, 3.2056), "{pick:?}");
        assert!(!StageChoice { stage: pick.stage, lowest: pick.lowest, auto: true }.too_low());
    }

    #[test]
    fn the_demo_buttons_secondary_asks_for_the_dark_stage() {
        let pick = pick_union(&demo_button("Secondary"), None);
        assert_eq!(pick.stage, Stage::Dark);
        assert!(near(pick.lowest, 11.434), "{pick:?}");
    }

    #[test]
    fn the_demo_buttons_primary_and_destructive_ask_for_the_light_stage() {
        let primary = pick_union(&demo_button("Primary"), None);
        assert_eq!(primary.stage, Stage::Light);
        assert!(near(primary.lowest, 4.688), "{primary:?}");
        let destructive = pick_union(&demo_button("Destructive"), None);
        assert_eq!(destructive.stage, Stage::Light);
        assert!(near(destructive.lowest, 3.787), "{destructive:?}");
    }

    #[test]
    fn the_whole_demo_button_still_has_no_stage_that_serves_it_and_the_panel_warns() {
        // The pale Secondary fills, the dark Primary and Destructive fills and the black outline text: dark wins with 1.23:1
        // (mid-grey gets 1.07, light 1.02).
        let all: Vec<Judged> = ["Primary", "Secondary", "Outline", "Destructive"].iter().flat_map(|k| demo_button(k)).collect();
        assert_eq!(all.len(), 60);
        let pick = pick_union(&all, None);
        assert_eq!(pick.stage, Stage::Dark);
        assert!(near(pick.lowest, 1.2347), "{pick:?}");
        assert!(StageChoice { stage: pick.stage, lowest: pick.lowest, auto: true }.too_low(), "no stage serves every variant: the panel says so");
    }

    #[test]
    fn the_demo_card_the_icon_and_the_accordion() {
        // Card: three variants with a white fill: the dark stage (15.97:1; mid-grey would be 4.48). Icon: 30 variants with no fill
        // and a dark glyph: the light stage (15.66:1). Accordion: no fill, dark text and a grey: the light stage (3.66:1).
        let card = pick_union(&[v(Some("#FFFFFF"), &[]), v(Some("#FFFFFF"), &[]), v(Some("#FFFFFF"), &[])], None);
        assert_eq!(card.stage, Stage::Dark);
        assert!(near(card.lowest, 15.974), "{card:?}");
        let icon: Vec<Judged> = (0..30).map(|_| v(None, &["#090B0C"])).collect();
        let icon = pick_union(&icon, None);
        assert_eq!(icon.stage, Stage::Light);
        assert!(near(icon.lowest, 15.656), "{icon:?}");
        let accordion = pick_union(&[v(None, &["#090B0C", "#67787C"]), v(None, &["#090B0C", "#67787C"]), v(None, &["#090B0C", "#67787C"])], None);
        assert_eq!(accordion.stage, Stage::Light);
        assert!(near(accordion.lowest, 3.660), "{accordion:?}");
        assert!(!StageChoice { stage: accordion.stage, lowest: accordion.lowest, auto: true }.too_low());
    }

    #[test]
    fn a_variant_with_a_background_is_judged_by_it_and_one_without_by_its_elements() {
        let with_bg = Judged { backdrop: Some(hex("#14213D")), elements: vec![hex("#FFFFFF")] };
        let without = only_elements(&["#FFFFFF"]);
        assert_eq!(union_colours(&[with_bg, without], None), [hex("#14213D"), hex("#FFFFFF")]);
        assert_eq!(union_colours(&[only_elements(&["#FFFFFF"])], Some(hex("#222222"))), [hex("#222222")], "the set's fill stands in for a variant with none");
    }

    #[test]
    fn a_tie_goes_to_light_then_mid_grey_then_dark_and_a_forced_stage_never_warns() {
        // A union with nothing to judge, or the same on every stage, is light.
        assert_eq!(pick_union(&[], None), Pick3 { stage: Stage::Light, lowest: None });
        // White, navy and the mid-grey itself: each stage loses one of them (light 1.2:1 on white, dark 1.0 on navy, mid-grey 1.0 on itself).
        let all = [only_elements(&["#FFFFFF"]), only_elements(&["#14213D"]), only_elements(&["#777777"])];
        let pick = pick_union(&all, None);
        assert!(StageChoice { stage: pick.stage, lowest: pick.lowest, auto: true }.too_low(), "{pick:?}");
        assert!(!StageChoice { stage: Stage::Dark, lowest: Some(1.0), auto: false }.too_low());
        assert!(!StageChoice { stage: Stage::Light, lowest: Some(3.0), auto: true }.too_low());
        assert_eq!(StageChoice { stage: Stage::Mid, lowest: None, auto: true }.fill(), "#777777");
    }
    #[test]
    fn a_cell_with_several_values_counts_its_cell_and_a_frame_and_the_contents_of_each() {
        let colour = Cv::Val { token: Some("t".into()), text: "Fill · #000000".into(), swatch: Some(("#000000".into(), false)) };
        let plain = Cv::Val { token: Some("t".into()), text: "Gap · 8px".into(), swatch: None };
        // A value cell is its frame (counted in `nodes`), a square, a column and the two texts; a plain one its frame and two texts.
        assert_eq!((colour.nodes(), plain.nodes()), (5, 3));
        assert_eq!(Cv::Many(vec![colour.clone(), plain.clone()]).nodes(), 1 + 5 + 3);
        assert_eq!(Cv::Many(vec![Cv::Muted("x".into()), plain]).nodes(), 1 + 1 + 3, "a text in a cell of several values is one node");
        // The table counts them: the grid, the headers (2 each), the separator and the row (its frame and its cells).
        let rows = vec![vec![Cv::Bold("a".into()), Cv::Many(vec![colour.clone(), colour])]];
        assert_eq!(table_nodes(2, &rows), 1 + 4 + 1 + (1 + 2 + (1 + 5 + 5)));
    }
    #[test]
    fn no_option_of_update_makes_a_cover_changelog_or_support_frame_that_is_there() {
        for update in Update::ALL {
            for created in [false, true] {
                for master in [false, true] {
                    for sec in [Sec::Cover, Sec::Changelog, Sec::Kit] {
                        assert!(!frame_is_made(sec, true, created, update, master), "{sec:?} under {update:?}");
                        assert!(frame_is_made(sec, false, created, update, master), "{sec:?} is made again when it is gone");
                    }
                }
            }
        }
        // The others: only what the update names; a new Section makes them all.
        assert!(frame_is_made(Sec::Tokens, true, false, Update::Tokens, false));
        assert!(!frame_is_made(Sec::Properties, true, false, Update::Tokens, false));
        assert!(frame_is_made(Sec::Layouts, false, true, Update::Tokens, false));
        assert!(frame_is_made(Sec::Master, true, false, Update::Layouts, true));
    }
    #[test]
    fn a_frame_that_is_gone_is_made_again_when_update_names_it_and_only_then() {
        // Update names it: made again (a rerun, the Section is not new).
        assert!(frame_is_made(Sec::Properties, false, false, Update::Properties, true));
        assert!(frame_is_made(Sec::Anatomy, false, false, Update::All, true));
        assert!(frame_is_made(Sec::Layouts, false, false, Update::Tables, true));
        // Update does not name it: not made.
        assert!(!frame_is_made(Sec::Properties, false, false, Update::Tokens, true));
        assert!(!frame_is_made(Sec::Tokens, false, false, Update::Tables, true));
    }

    #[test]
    fn the_frames_that_stay_gone_are_the_ones_update_does_not_name() {
        let gone = |gone: &'static [Sec]| move |s: Sec| !gone.contains(&s);
        let bit = |s: Sec| 1u16 << s.index();
        // Properties is gone and Update names Tokens: it stays gone, and the run says so.
        assert_eq!(missing_mask(&gone(&[Sec::Properties]), false, Update::Tokens), bit(Sec::Properties));
        // Update names it: nothing is missing. A new Section makes everything. Cover, Changelog and the Master are never "missing".
        assert_eq!(missing_mask(&gone(&[Sec::Properties]), false, Update::Properties), 0);
        assert_eq!(missing_mask(&gone(&[Sec::Properties]), true, Update::Tokens), 0);
        assert_eq!(missing_mask(&gone(&[Sec::Cover, Sec::Changelog, Sec::Master]), false, Update::Tokens), 0);
        assert_eq!(missing_mask(&gone(&[Sec::Anatomy, Sec::Tokens]), false, Update::Layouts), bit(Sec::Anatomy) | bit(Sec::Tokens));
    }

    #[test]
    fn the_new_warnings_have_their_codes_and_say_the_names_of_the_frames_in_both_languages() {
        assert_eq!(Warn::SupportMissing.code(), "support_missing");
        assert_eq!(Warn::FrameMissing(0).code(), "frame_missing");
        let mask = (1u16 << Sec::Properties.index()) | (1 << Sec::Tokens.index());
        assert_eq!(Warn::FrameMissing(mask).line(Lang::En), "Not made, because they are missing and Update does not name them: Properties, Tokens.");
        assert_eq!(Warn::FrameMissing(1 << Sec::Layouts.index()).line(Lang::Pt), "Não foi feito, porque falta e Atualizar não o nomeia: Layouts (espaçamento).");
        for lang in Lang::ALL {
            assert!(!Warn::SupportMissing.text(lang).is_empty());
            for w in [Warn::SupportMissing, Warn::FrameMissing(mask)] {
                assert!(w.line(lang).chars().count() <= 140, "{}", w.line(lang));
            }
        }
        let codes: Vec<&str> = Warn::ALL.iter().map(|w| w.code()).collect();
        assert_eq!(codes.len(), 10);
    }

    #[test]
    fn the_warning_for_markers_left_out_counts_them_and_is_short_in_both_languages() {
        // One: singular; three: plural; the number is in the text.
        assert_eq!(Warn::MarkersDropped(1).line(Lang::En), "1 marker did not fit in the drawing; see the Note column.");
        assert_eq!(Warn::MarkersDropped(3).line(Lang::En), "3 markers did not fit in the drawing; see the Note column.");
        assert_eq!(Warn::MarkersDropped(1).line(Lang::Pt), "1 marcador não coube no desenho; veja a coluna Nota.");
        assert_eq!(Warn::MarkersDropped(3).line(Lang::Pt), "3 marcadores não couberam no desenho; veja a coluna Nota.");
        for lang in [Lang::En, Lang::Pt] {
            for n in [1, 2, 3, 12] {
                let text = Warn::MarkersDropped(n).line(lang);
                assert!(text.chars().count() <= 120 && !text.contains('{'), "{text}");
            }
            assert!(t(lang, Key::AnatNoMarker).chars().count() <= 60);
        }
        assert_eq!(Warn::MarkersDropped(2).code(), "markers_dropped");
        // The code finds it among all the warnings whatever the count, and it sorts after the others.
        assert!(Warn::ALL.iter().any(|w| w.code() == "markers_dropped"));
    }

    #[test]
    fn the_approval_set_has_a_hidden_name_and_is_found_by_its_mark_not_by_name() {
        for lang in [Lang::En, Lang::Pt] {
            assert!(t(lang, Key::KitSetName).starts_with('.'), "a name that starts with a dot keeps the set out of a published library");
        }
        assert_eq!(t(Lang::En, Key::KitSetName), ".Handoff · Approval");
        assert_eq!(t(Lang::Pt, Key::KitSetName), ".Handoff · Aprovação");
        // A set of an older Section (the name without the dot) and one of a new Section both carry the mark, which is all that is looked at.
        for name in ["Handoff · Approval", ".Handoff · Approval", "renamed by hand"] {
            let child = json!({"id": 7, "name": name, "pluginData": {APPROVAL_SET_MARK: "1"}});
            assert_eq!(mark_of(&child, APPROVAL_SET_MARK), Some("1"), "{name}");
        }
    }
}
