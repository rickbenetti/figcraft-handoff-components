//! The 0.11 document: one Section for the component, with a frame (in ONE ROW, left to right) for each part of the handoff (Cover, Component Master,
//! Properties, Anatomy, States and Variants, Layouts, Tokens, Changelog and the support components), 1920 px wide, 64 px
//! apart, with 256 px of room around them inside the Section.
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

/// The frame's width, its padding and the useful width inside it.
pub const FRAME_W: f64 = 1920.0;
pub const FRAME_PAD: f64 = 48.0;
pub const USEFUL_W: f64 = FRAME_W - 2.0 * FRAME_PAD;
/// The room around the frames inside the Section, and the gap between them.
pub const SECTION_PAD: f64 = 256.0;
pub const FRAME_GAP: f64 = 64.0;
/// The frames are in ONE ROW, left to right, all at y = 256: the one of order `i` is at x = 256 + i × (1920 + 64).
pub const FRAME_COUNT: usize = 9;
/// The Section's width: 256 + 9 × 1920 + 8 × 64 + 256.
pub const SECTION_W: f64 = FRAME_COUNT as f64 * FRAME_W + (FRAME_COUNT as f64 - 1.0) * FRAME_GAP + 2.0 * SECTION_PAD;
/// The room between a Section and the one made below it with "New Section".
pub const SECTION_BELOW_GAP: f64 = 512.0;

/// The x, inside the Section, of the frame of order `i`.
pub fn frame_x(i: usize) -> f64 {
    SECTION_PAD + i as f64 * (FRAME_W + FRAME_GAP)
}

/// The Section's height: the room around the tallest frame.
pub fn section_height(tallest: f64) -> f64 {
    SECTION_PAD + tallest + SECTION_PAD
}

/// Where "New Section" goes: at the x of the reference Section, below everything on the page that stands in its column (that
/// is, whose horizontal extent meets the reference's) at or under its top, plus 512. `boxes` are the page's top-level boxes as
/// (x, y, w, h); the reference is among them.
pub fn below_position(reference: (f64, f64, f64, f64), boxes: &[(f64, f64, f64, f64)]) -> (f64, f64) {
    let (rx, ry, rw, rh) = reference;
    let bottom = boxes.iter().filter(|(x, y, w, h)| *x < rx + rw && rx < *x + *w && *y + *h > ry).map(|(_, y, _, h)| y + h).fold(ry + rh, f64::max);
    (rx, bottom + SECTION_BELOW_GAP)
}
/// The mark of the Section on the page (the value is the id of the component), of a frame's section, and of the set of approval pills.
pub const SECTION_MARK: &str = "handoff-report-section";
pub const NAME_MARK: &str = "handoff-section";
pub const APPROVAL_SET_MARK: &str = "handoff-approval-set";
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
const CHANGELOG_COLS: [f64; 3] = [200.0, 240.0, 1384.0];
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
    Kit,
}

impl Sec {
    pub const ALL: [Sec; 9] = [Sec::Cover, Sec::Master, Sec::Properties, Sec::Anatomy, Sec::Variants, Sec::Layouts, Sec::Tokens, Sec::Changelog, Sec::Kit];

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

    /// The place of the frame in the row (0 to 8).
    pub fn index(self) -> usize {
        Sec::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    pub fn from_key(key: &str) -> Option<Sec> {
        Sec::ALL.into_iter().find(|s| s.key() == key)
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
    /// The component is wider than the stage's useful width.
    WideMaster,
    /// A loose documentation of version 0.10.0 was found and left as it was.
    LegacyDoc,
}

impl Warn {
    pub const ALL: [Warn; 4] = [Warn::Failed, Warn::NoStage, Warn::WideMaster, Warn::LegacyDoc];

    pub fn code(self) -> &'static str {
        match self {
            Warn::Failed => "table_failed",
            Warn::NoStage => "no_stage",
            Warn::WideMaster => "wide_master",
            Warn::LegacyDoc => "legacy_doc",
        }
    }

    pub fn text(self, lang: Lang) -> &'static str {
        t(lang, match self {
            Warn::Failed => Key::PanelWarnFailed,
            Warn::NoStage => Key::PanelWarnNoStage,
            Warn::WideMaster => Key::PanelWarnWide,
            Warn::LegacyDoc => Key::PanelWarnLegacy,
        })
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

fn mark_of<'a>(node: &'a Value, key: &str) -> Option<&'a str> {
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
        out.push(Existing { id, sec, x: b.x - sb.x, y: b.y - sb.y, h: b.h });
    }
    Ok((sb, out))
}

/// The set of approval pills in the support frame: the child that carries the `handoff-approval-set` mark.
fn read_approval_set(lang: Lang, kit: u64) -> Result<Option<u64>> {
    let node = host(lang, "node.get", &json!({"id": kit, "fields": ["children", "pluginData"]}))?;
    Ok(node.get("children").and_then(Value::as_array).into_iter().flatten().find(|c| mark_of(c, APPROVAL_SET_MARK).is_some()).and_then(|c| c.get("id").and_then(Value::as_u64)))
}

// ---------------------------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------------------------

/// A frame of the Section, at `y` inside it (`x` is the room around): 1920 wide, padding 48, auto layout, hug.
fn new_frame(lang: Lang, section: u64, name: &str, comp_key: &str, sec: Sec) -> Result<u64> {
    let created = host(
        lang,
        "frame.create",
        &json!({"parent": section, "x": frame_x(sec.index()), "y": SECTION_PAD, "width": FRAME_W, "height": 200, "name": name, "select": false,
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
}

impl Cv {
    /// The nodes the cell makes: its frame, and what is in it.
    pub(super) fn nodes(&self) -> usize {
        match self {
            // Without a colour the token and the value are in the cell itself; with one, a square and a column of the two lines.
            Cv::Val { token, swatch, .. } => usize::from(token.is_some()) + if swatch.is_some() { 4 } else { 2 },
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
        &json!({"parent": frame, "width": USEFUL_W, "height": 40, "name": name, "select": false,
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
            let direction = if matches!(v, Cv::Val { swatch: None, .. }) { "vertical" } else { "horizontal" };
            let c = cell_in(lang, grid, col, gr, None, direction)?;
            match v {
                Cv::Text(s) => cell_text(lang, c, s, pal::TEXT, false)?,
                Cv::Bold(s) => cell_text(lang, c, s, pal::TEXT, true)?,
                Cv::Muted(s) => cell_text(lang, c, s, pal::TEXT2, false)?,
                Cv::Guide(s) => cell_text(lang, c, s, pal::GUIDE, false)?,
                Cv::Val { token, text, swatch } => value_cell(lang, c, token.as_deref(), text, swatch.as_ref())?,
            };
        }
        separator(lang, grid, gr + 1, n)?;
    }
    Ok(grid)
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
    pub(super) stage: StageChoice,
    /// How many areas of the Cover's pills did not read back as written and were written again.
    pub(super) area_retries: std::cell::Cell<u32>,
    /// The file's variables and styles, read once per run and only when a layer needs them (`variable.list` costs about a tenth of the
    /// budget in a big file).
    pub(super) vars: std::cell::RefCell<Option<std::rc::Rc<Vars>>>,
    pub(super) styles: std::cell::RefCell<Option<std::rc::Rc<Styles>>>,
    /// The walk of the default variant's instance, made by the first frame that needs it (Anatomy, when it is made in the same run) and
    /// used by the others: the instance is made and read once.
    pub(super) base_walk: std::cell::RefCell<Option<Walk>>,
    /// The tables the content frames made: (name, rows), for the answer.
    pub(super) tables: std::cell::RefCell<Vec<(String, usize)>>,
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
        let v = std::rc::Rc::new(Vars::parse(&host(self.lang, "variable.list", &json!({}))?));
        *slot = Some(v.clone());
        Ok(v)
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
        g.extra = json!({"gridColumns": 6, "gridRows": 1, "columnTracks": [PILL_W, PILL_W, PILL_W, PILL_W, PILL_W, PILL_W], "rowTracks": ["auto"], "rowGap": 0, "sizingH": "fixed"});
        g.gap = 16.0;
        let grid = flow_frame(lang, frame, g)?;
        let area = t(lang, Key::ApprPropArea);
        for role in [Key::RoleDesigner, Key::RoleEngineering, Key::RoleQa, Key::RoleWriters, Key::RolePo, Key::RolePm] {
            let made = host(lang, "instance.create", &json!({"component": set, "parent": grid}))?;
            let inst = id_of(lang, &made, "instance.create")?;
            write_area(c, inst, area, t(lang, role))?;
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
    table(lang, frame, "Changelog", &CHANGELOG_COLS, &headers, &rows)?;
    Ok(())
}

/// A section that is not made yet: its title and a line that says when it will be.
fn build_reserved(c: &Ctx, frame: u64, sec: Sec) -> Result<()> {
    header(c, frame, t(c.lang, sec.title_key()))?;
    let step = if sec == Sec::Tokens { Key::ReservedC } else { Key::ReservedB };
    text_with(c.lang, frame, t(c.lang, step), LINE_SIZE, pal::GUIDE, "Note", json!({}))?;
    Ok(())
}

/// The Component Master: the title, a line about it, the fixed warning, and the stage with the original component moved onto it.
/// Returns the moved layer's id and whether it is wider than the stage's useful width.
fn build_master(c: &Ctx, frame: u64) -> Result<(u64, bool)> {
    let lang = c.lang;
    header(c, frame, t(lang, Key::MasterTitle))?;
    text_with(lang, frame, t(lang, Key::MasterDescription), LINE_SIZE, pal::TEXT2, "Description", json!({"width": USEFUL_W, "textAutoResize": "height"}))?;
    text_with(lang, frame, t(lang, Key::MasterNote), LINE_SIZE, pal::COND, "Warning", json!({"width": USEFUL_W, "textAutoResize": "height"}))?;
    let mut st = FlowSpec::new("Stage", USEFUL_W, "vertical");
    st.padding = json!(STAGE_PAD);
    st.fill = Some(c.stage.fill());
    st.extra = json!({"sizingH": "fill", "sizingV": "hug", "cornerRadius": 12, "stroke": pal::LINE, "strokeWeight": 1, "strokeAlign": "inside"});
    let stage = flow_frame(lang, frame, st)?;
    let moved = c.target.holder;
    host(lang, "node.reparent", &json!({"ids": [moved], "parent": stage}))?;
    let node = host(lang, "node.get", &json!({"id": moved, "fields": ["absoluteBounds"]}))?;
    let wide = bounds(&node).is_some_and(|b| b.w > USEFUL_W - 2.0 * STAGE_PAD + 0.5);
    Ok((moved, wide))
}

/// The set of approval pills, made inside `frame`: four variants (Estado = pendente, aprovado, ressalvas, reprovado), each the pill
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

/// The support components: a note that says not to delete it, and the set of approval pills.
fn build_kit(c: &Ctx, frame: u64) -> Result<u64> {
    let lang = c.lang;
    header(c, frame, t(lang, Key::SecKit))?;
    text_with(lang, frame, t(lang, Key::KitNote), LINE_SIZE, pal::COND, "Warning", json!({"width": USEFUL_W, "textAutoResize": "height"}))?;
    build_approval_set(c, frame)
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
    if host(lang, "node.get", &json!({"id": target.holder, "fields": ["type"]}))?.is_null() {
        return Err(t(lang, Key::ErrMainGone).to_string());
    }
    let comp_key = target.key.to_string();
    let name = target.title.clone();
    let update = input.str("update").and_then(Update::from_key).unwrap_or(Update::All);
    let new_section = input.params.get("new_section").and_then(Value::as_bool).unwrap_or(false);
    // 2. Which Section: the first one of the component, unless a new one is asked for (or there is none).
    let found = find_section(&page, &comp_key);
    let first_run = found.is_none();
    let beside = found.is_some() && new_section;
    let section_id = if beside { None } else { found };
    let mut warnings: Vec<Warn> = Vec::new();
    if first_run && has_legacy_frame(&page, &comp_key) {
        warnings.push(Warn::LegacyDoc);
    }
    // "New Section" goes BELOW the reference Section (the first of the component), at the same x.
    let below = match found {
        Some(id) if beside => {
            let boxes: Vec<(f64, f64, f64, f64)> = page.children.iter().filter_map(bounds).map(|b| (b.x, b.y, b.w, b.h)).collect();
            let reference = page.children.iter().find(|c| c.get("id").and_then(Value::as_u64) == Some(id)).and_then(bounds).map(|b| (b.x, b.y, b.w, b.h));
            reference.map(|r| below_position(r, &boxes))
        }
        _ => None,
    };
    let right = page.children.iter().filter_map(bounds).map(Rect::right).fold(None, |r: Option<f64>, v| Some(r.map_or(v, |r| r.max(v))));
    let calls0 = HOST_CALLS.load(Ordering::Relaxed);
    // 3. The Section: the one that is there, or a new one right of everything (512 px beyond it, at y = 0), or below the first one.
    let (section, sx, sy, existing, created) = match section_id {
        Some(id) => {
            let (sb, frames) = read_frames(lang, id)?;
            (id, sb.x, sb.y, frames, false)
        }
        None => {
            let (x, y) = below.unwrap_or((right.map_or(0.0, |r| r + GAP_RIGHT), 0.0));
            let made = host(
                lang,
                "node.create",
                &json!({"type": "section", "name": tf(lang, Key::SectionName, &[("name", &name)]), "x": x, "y": y, "width": SECTION_W, "height": 400, "select": false,
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
    let will_make = |sec: Sec| -> bool {
        match (old(sec), sec) {
            (_, Sec::Master) => master_allowed,
            (None, s) if s.permanent() => true,
            (None, _) => created,
            (Some(_), s) if s.permanent() => false,
            (Some(_), s) => update.sections().contains(&s),
        }
    };
    // 5. The support frame first: the Cover's instances need its set. It is made at the top and moved into place with the others.
    // A support frame made by the A2 build has no set of pills: the set is made inside it, and the frame is not deleted.
    let mut ctx = Ctx { lang, target: &target, mode: page.settings.stage_mode(), pills: None, stage: StageChoice { stage: Stage::Light, lowest: None, auto: false }, area_retries: std::cell::Cell::new(0), vars: std::cell::RefCell::new(None), styles: std::cell::RefCell::new(None), tables: std::cell::RefCell::new(Vec::new()), base_walk: std::cell::RefCell::new(None) };
    let kit_frame: Option<(u64, &str)>;
    let mut set_added = false;
    if let Some(e) = old(Sec::Kit) {
        ctx.pills = read_approval_set(lang, e.id)?;
        if ctx.pills.is_none() {
            ctx.pills = Some(build_approval_set(&ctx, e.id)?);
            set_added = true;
        }
        kit_frame = Some((e.id, "kept"));
    } else {
        let kit_name = tf(lang, Key::SectionFrameName, &[("title", t(lang, Sec::Kit.title_key())), ("name", &name)]);
        let frame = new_frame(lang, section, &kit_name, &comp_key, Sec::Kit)?;
        ctx.pills = Some(build_kit(&ctx, frame)?);
        kit_frame = Some((frame, "created"));
    }
    // 6. The stage of the Master, over every variant (a sample of them in a big set), when the Master is made now.
    let stage_sampled: std::cell::Cell<Option<(usize, usize)>> = std::cell::Cell::new(None);
    if will_make(Sec::Master) {
        ctx.stage = choose_stage(lang, &target, page.settings.stage_mode(), &stage_sampled)?;
        if ctx.stage.too_low() {
            warnings.push(Warn::NoStage);
        }
    }
    // 7. The frames, in order.
    let mut tallest: f64 = 0.0;
    let mut report: Vec<Value> = Vec::new();
    let mut master_moved: Option<u64> = None;
    let mut failed: Vec<&str> = Vec::new();
    for sec in Sec::ALL {
        let (id, h, action): (u64, f64, &str) = if sec == Sec::Kit {
            let (id, action) = kit_frame.unwrap_or((0, "kept"));
            (id, height_of(lang, id)?, action)
        } else if will_make(sec) {
            let frame_name = if sec == Sec::Master { tf(lang, Key::MasterFrameName, &[("name", &name)]) } else { tf(lang, Key::SectionFrameName, &[("title", t(lang, sec.title_key())), ("name", &name)]) };
            let replaced = old(sec).is_some();
            // The old Master is deleted only AFTER the original component is in the new one (deleting a frame deletes what is in it).
            if sec != Sec::Master && let Some(e) = old(sec) {
                host(lang, "edit.delete", &json!({"ids": [e.id]}))?;
            }
            let mut frame = new_frame(lang, section, &frame_name, &comp_key, sec)?;
            // A table that cannot be made leaves its frame with the error in place of the content, and the run goes on.
            let drawn: Option<Result<()>> = match sec {
                Sec::Properties => Some(tables::build_properties(&ctx, frame)),
                Sec::Anatomy => Some(tables::build_anatomy(&ctx, frame).map(|no_stage| {
                    if no_stage && !warnings.contains(&Warn::NoStage) {
                        warnings.push(Warn::NoStage);
                    }
                })),
                Sec::Variants => Some(tables::build_variants(&ctx, frame)),
                Sec::Layouts => Some(tables::build_layouts(&ctx, frame)),
                _ => None,
            };
            #[cfg(feature = "test-hooks")]
            let drawn = if drawn.is_some() && input.str("fail_table") == Some(sec.key()) { Some(Err("forced failure (test hook)".to_string())) } else { drawn };
            if let Some(Err(reason)) = drawn {
                host(lang, "edit.delete", &json!({"ids": [frame]}))?;
                frame = new_frame(lang, section, &frame_name, &comp_key, sec)?;
                header(&ctx, frame, t(lang, sec.title_key()))?;
                text_with(lang, frame, &tf(lang, Key::TableFailed, &[("reason", &reason)]), LINE_SIZE, pal::BAD, "Error", json!({"width": USEFUL_W, "textAutoResize": "height"}))?;
                failed.push(sec.key());
            }
            match sec {
                Sec::Cover => build_cover(&ctx, frame)?,
                Sec::Changelog => build_changelog(&ctx, frame)?,
                Sec::Master => {
                    let (moved, wide) = build_master(&ctx, frame)?;
                    if let Some(e) = old(sec) {
                        host(lang, "edit.delete", &json!({"ids": [e.id]}))?;
                    }
                    master_moved = Some(moved);
                    if wide {
                        warnings.push(Warn::WideMaster);
                    }
                }
                Sec::Tokens => build_reserved(&ctx, frame, Sec::Tokens)?,
                _ => {}
            }
            (frame, height_of(lang, frame)?, if replaced { "replaced" } else { "created" })
        } else if let Some(e) = old(sec) {
            (e.id, e.h, "kept")
        } else {
            continue;
        };
        // A frame that is not where it should be (a kept one from an older layout) moves to its place in the row.
        let (want_x, want_y) = (frame_x(sec.index()), SECTION_PAD);
        let (at_x, at_y) = if action == "kept" { old(sec).map_or((want_x, want_y), |e| (e.x, e.y)) } else { (want_x, want_y) };
        if (at_x - want_x).abs() > 0.5 || (at_y - want_y).abs() > 0.5 {
            host(lang, "node.set", &json!({"ids": [id], "props": {"x": want_x, "y": want_y}}))?;
        }
        tallest = tallest.max(h);
        report.push(json!({"key": sec.key(), "id": id, "x": want_x, "y": want_y, "h": h, "action": action}));
    }
    // 8. The Section is as wide as the row and as tall as its tallest frame and the room around it.
    let section_h = section_height(tallest);
    host(lang, "node.set", &json!({"ids": [section], "props": {"width": SECTION_W, "height": section_h}}))?;
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
    warnings.sort_by_key(|w| Warn::ALL.iter().position(|x| x == w));
    Ok(json!({
        "name": name, "frame": section, "section": section, "created": created, "rerun": !first_run, "new_section": beside, "update": update.key(), "x": sx, "y": sy,
        "frame_size": {"w": SECTION_W}, "view_w": 2.0 * FRAME_W + FRAME_GAP, "tallest": tallest, "section_size": {"w": SECTION_W, "h": section_h}, "frames": report, "master_moved": master_moved,
        "warnings": warnings.iter().map(|w| w.code()).collect::<Vec<_>>(), "export": export_result, "lang": lang.code(), "component": target.key, "holder": target.holder,
        "stage": {"sampled": stage_sampled.get().map(|(a, b)| json!([a, b])), "name": ctx.stage.name(), "dark": ctx.stage.stage == Stage::Dark, "auto": ctx.stage.auto, "lowest": ctx.stage.lowest, "fill": ctx.stage.fill()},
        "failed": failed, "tables": ctx.tables.borrow().iter().map(|(n, r)| json!({"name": n, "rows": r})).collect::<Vec<_>>(), "pill_set": ctx.pills, "area_retries": ctx.area_retries.get(), "pill_set_added": set_added, "host_calls": HOST_CALLS.load(Ordering::Relaxed) - calls0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Rgba {
        contrast::parse_hex(s, 1.0).unwrap_or(contrast::LIGHT_STAGE)
    }

    #[test]
    fn the_sections_come_in_the_order_of_the_frames_with_their_own_keys() {
        let keys: Vec<&str> = Sec::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(keys, ["cover", "master", "properties", "anatomy", "variants", "layouts", "tokens", "changelog", "kit"]);
        for s in Sec::ALL {
            assert_eq!(Sec::from_key(s.key()), Some(s));
        }
        assert_eq!(Sec::from_key("nope"), None);
    }

    #[test]
    fn three_frames_are_made_once_and_the_other_five_are_the_ones_a_rerun_replaces() {
        let permanent: Vec<Sec> = Sec::ALL.into_iter().filter(|s| s.permanent()).collect();
        assert_eq!(permanent, [Sec::Cover, Sec::Changelog, Sec::Kit], "the Master is not among them: it is made again on every run");
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

    #[test]
    fn the_frames_are_in_one_row_at_y_256_with_64_between_them() {
        let xs: Vec<f64> = (0..FRAME_COUNT).map(frame_x).collect();
        assert_eq!(xs, [256.0, 2240.0, 4224.0, 6208.0, 8192.0, 10176.0, 12160.0, 14144.0, 16128.0]);
        assert!(xs.windows(2).all(|w| w[1] - w[0] - FRAME_W == FRAME_GAP));
        assert_eq!(xs[8] + FRAME_W + SECTION_PAD, SECTION_W);
        assert_eq!(SECTION_W, 18304.0);
        assert_eq!(SECTION_W, 256.0 + 9.0 * 1920.0 + 8.0 * 64.0 + 256.0);
        for (i, s) in Sec::ALL.iter().enumerate() {
            assert_eq!(s.index(), i);
        }
    }

    #[test]
    fn the_section_is_as_tall_as_its_tallest_frame_plus_the_room_around() {
        assert_eq!(section_height(1359.0), 256.0 + 1359.0 + 256.0);
        assert_eq!(section_height(0.0), 512.0);
    }

    #[test]
    fn a_new_section_goes_below_everything_in_the_column_of_the_reference_and_not_beside_other_components() {
        let button = (3552.0, 0.0, SECTION_W, 4000.0);
        let card = (3552.0 + SECTION_W + 512.0, 0.0, SECTION_W, 3500.0);
        let loose = (0.0, 0.0, 1000.0, 5000.0); // a frame to the left, not in the column
        // Below the Button alone: its bottom + 512, same x.
        assert_eq!(below_position(button, &[button, card, loose]), (3552.0, 4512.0));
        // A second Button Section already below: the next one goes under that one.
        let second = (3552.0, 4512.0, SECTION_W, 4200.0);
        assert_eq!(below_position(button, &[button, card, loose, second]), (3552.0, 4512.0 + 4200.0 + 512.0));
        // The Card's column is its own.
        assert_eq!(below_position(card, &[button, card, loose, second]), (card.0, 3500.0 + 512.0));
    }

    #[test]
    fn the_columns_of_the_table_fill_the_useful_width() {
        assert_eq!(USEFUL_W, 1824.0);
        assert_eq!(CHANGELOG_COLS.iter().sum::<f64>(), USEFUL_W);
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
}
