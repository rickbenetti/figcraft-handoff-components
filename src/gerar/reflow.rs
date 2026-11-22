//! The Component Master and the width of the column. The frames of a Section are all 1180 wide; the room the component set has inside the
//! Master is 1180 - 2 x 48 (the frame) - 2 x 48 (the stage) = 988. A set that is wider than that is NOT moved by x, y: it is given auto
//! layout, horizontal, with wrap on (24 padding, 48 between variants and 48 between lines), a fixed width of 988 and a height that hugs the
//! lines, so the host lays the variants out in lines inside the set, in the order of the layers. A set that already has auto layout keeps its
//! own padding and gaps: only the wrap, the width and the height are written. A set that fits is not touched.
//!
//! The decision is a pure function (tested on the host); `organize` reads the set, writes the properties and reads it back. When the host
//! does not keep what was written (or a variant is still outside the set), the warning `reflow_failed` is raised and the Master widens to
//! the set as it is; the variants are never placed by x, y.
//!
//! Only when not even one variant fits in a line (the widest one plus the padding is wider than 988) the Master grows, and every frame of
//! the Section with it, to the same width, so the column stays straight.

#![warn(dead_code)] // the module above allows dead code (the 0.10.0 drawing); this one is checked

use super::section::{mark_of, FRAME_PAD, FRAME_W};
use super::*;

/// The padding of the component set around its variants, and the gap between variants (horizontal and vertical).
pub const SET_PAD: f64 = 24.0;
pub const VARIANT_GAP: f64 = 48.0;
/// The padding of the Master's stage around the set.
pub const STAGE_PAD: f64 = 48.0;
/// The room the set has inside the Master's stage in a 1180 frame: 1180 - 2 x 48 - 2 x 48.
pub const AVAILABLE_W: f64 = FRAME_W - 2.0 * FRAME_PAD - 2.0 * STAGE_PAD;
/// What a wide Master frame has more than its set: the stage's padding and the frame's, on both sides.
pub const WIDE_EXTRA: f64 = 2.0 * (FRAME_PAD + STAGE_PAD);
/// The mark the set carries once the plugin organized it (so a rerun, which finds it already fitting, still tells the Master so).
pub const ORGANIZED_MARK: &str = "handoff-organized";

/// A variant of the set: its id, its place inside the set and its size.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The auto layout a set already has (what the person made).
#[derive(Clone, Debug, PartialEq)]
pub struct Auto {
    pub horizontal: bool,
    pub wrap: bool,
    pub padding: [f64; 4],
    pub gap: f64,
    pub cross_gap: f64,
}

/// What is read of the set (or of the single component) that goes into the Master.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SetInfo {
    pub width: f64,
    pub height: f64,
    /// The variants in the order of the layers; none for a single component.
    pub cells: Vec<Cell>,
    pub auto: Option<Auto>,
    /// The sizing of the set: whether its width is fixed and its height hugs.
    pub fixed_w: bool,
    pub hug_h: bool,
    /// What an inside stroke takes from each side (auto layout counts it like padding).
    pub inset: f64,
    /// Whether the plugin organized this set in an earlier run.
    pub marked: bool,
}

/// What is written on the set.
#[derive(Clone, Debug, PartialEq)]
pub struct Organize {
    /// The fixed width of the set.
    pub width: f64,
    /// The person's own auto layout, whose padding and gaps stay as they are; none when the set had none.
    pub kept: Option<Auto>,
}

impl Organize {
    /// The properties of the `node.set`.
    pub fn props(&self) -> Value {
        let mut p = json!({"wrap": true, "sizingH": "fixed", "sizingV": "hug", "width": self.width, "pluginData": {ORGANIZED_MARK: "1"}});
        if self.kept.is_none() {
            p["layout"] = json!("horizontal");
            p["padding"] = json!(SET_PAD);
            p["gap"] = json!(VARIANT_GAP);
            p["crossGap"] = json!(VARIANT_GAP);
        }
        p
    }
}

/// What the Master does with a set.
#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    /// It fits: it stays as the person left it.
    Keep,
    /// It does not fit: it gets auto layout with wrap and the room's width.
    Organize(Organize),
    /// Not even one variant fits in a line (or there is nothing to organize): the variants wrap in a set as wide as the widest needs
    /// (`organize`), or nothing moves (`None`), and the Master grows to `width` + 192.
    Wide { width: f64, organize: Option<Organize> },
    /// The set cannot be organized (an auto layout that is not horizontal): nothing is written, the Master grows to the set.
    Unsupported { width: f64 },
}

/// The variants in the order a person reads them: top to bottom, then left to right (a pixel of difference in y is the same row).
pub fn reading_order(cells: &[Cell]) -> Vec<Cell> {
    let mut out = cells.to_vec();
    out.sort_by(|a, b| (a.y.round()).total_cmp(&b.y.round()).then(a.x.total_cmp(&b.x)));
    out
}

/// Whether the order of the layers is not the order the variants are read in now (the auto layout follows the layers).
pub fn order_differs(cells: &[Cell]) -> bool {
    reading_order(cells).iter().map(|c| c.id).ne(cells.iter().map(|c| c.id))
}

/// How many variants are in each line, top to bottom (the lines are told by the top edge, to the pixel).
pub fn per_row(cells: &[Cell]) -> Vec<usize> {
    let mut rows: Vec<(f64, usize)> = Vec::new();
    for c in reading_order(cells) {
        match rows.last_mut() {
            Some((y, n)) if (c.y - *y).abs() < 1.0 => *n += 1,
            _ => rows.push((c.y, 1)),
        }
    }
    rows.into_iter().map(|(_, n)| n).collect()
}

/// The plan for `set` in `available` (988 in a 1180 frame).
pub fn plan(set: &SetInfo, available: f64) -> Plan {
    if set.width <= available + 0.5 {
        return Plan::Keep;
    }
    if set.cells.is_empty() {
        // A single component, wider than the room: nothing to lay out, the Master grows.
        return Plan::Wide { width: set.width, organize: None };
    }
    if set.auto.as_ref().is_some_and(|a| !a.horizontal) {
        return Plan::Unsupported { width: set.width };
    }
    let (pad_l, pad_r) = set.auto.as_ref().map_or((SET_PAD, SET_PAD), |a| (a.padding[3], a.padding[1]));
    let widest = set.cells.iter().map(|c| c.w).fold(0.0, f64::max);
    let line = widest + pad_l + pad_r + 2.0 * set.inset;
    let kept = set.auto.clone();
    if line > available + 0.5 {
        return Plan::Wide { width: line, organize: Some(Organize { width: line, kept }) };
    }
    Plan::Organize(Organize { width: available, kept })
}

/// Whether what was read after writing is what `want` asked for: a horizontal auto layout with wrap, fixed width, hugging height, and no
/// variant outside the set. The error says what is missing.
pub fn check(read: &SetInfo, want: &Organize) -> std::result::Result<(), String> {
    let Some(a) = &read.auto else { return Err("no auto layout".into()) };
    if !a.horizontal {
        return Err("the layout is not horizontal".into());
    }
    if !a.wrap {
        return Err("wrap is off".into());
    }
    if !read.fixed_w || !read.hug_h {
        return Err("the sizing is not fixed width and hugging height".into());
    }
    if (read.width - want.width).abs() > 0.5 {
        return Err(format!("the width is {} and not {}", read.width, want.width));
    }
    if let Some(c) = read.cells.iter().find(|c| c.x < -0.5 || c.y < -0.5 || c.x + c.w > read.width + 0.5 || c.y + c.h > read.height + 0.5) {
        return Err(format!("variant {} is outside the set", c.id));
    }
    Ok(())
}

impl Plan {
    /// The width of every frame of the Section: 1180, or the content + 192 when the Master had to grow.
    pub fn frame_width(&self) -> f64 {
        match self {
            Plan::Wide { width, .. } | Plan::Unsupported { width } => FRAME_W.max(width + WIDE_EXTRA),
            _ => FRAME_W,
        }
    }

    pub fn wide(&self) -> bool {
        matches!(self, Plan::Wide { .. } | Plan::Unsupported { .. })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Plan::Keep => "keep",
            Plan::Organize(_) => "organize",
            Plan::Wide { .. } => "wide",
            Plan::Unsupported { .. } => "unsupported",
        }
    }

    fn organize(&self) -> Option<&Organize> {
        match self {
            Plan::Organize(o) => Some(o),
            Plan::Wide { organize, .. } => organize.as_ref(),
            _ => None,
        }
    }
}

/// What the Master does, as it came out: the plan that was carried out (or the one the host did not keep), the frame's width, the lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub plan: &'static str,
    pub frame_w: f64,
    pub wide: bool,
    /// The plan was written but the host did not keep it (the warning `reflow_failed`), or it could not be written.
    pub failed: Option<String>,
    /// The set was organized in this run.
    pub organized: bool,
    /// The set carries the mark of an earlier run or of this one: the Master says so.
    pub marked: bool,
    pub width: f64,
    pub height: f64,
    pub rows: Vec<usize>,
    pub variants: usize,
    pub order_differs: bool,
    /// The padding and the gaps of a set that had its own auto layout.
    pub preserved: Option<Auto>,
}

impl Outcome {
    pub fn json(&self) -> Value {
        json!({
            "plan": self.plan, "frame_w": self.frame_w, "organized": self.organized, "marked": self.marked, "width": self.width, "height": self.height,
            "rows": self.rows.len(), "per_row": self.rows, "variants": self.variants, "order_differs": self.order_differs, "failed": self.failed,
            "preserved": self.preserved.as_ref().map(|a| json!({"padding": a.padding, "gap": a.gap, "cross_gap": a.cross_gap})),
        })
    }
}

/// The columns of a table scaled from the 1084 of a 1180 frame to `useful` wide, in whole pixels adding up to exactly `useful`.
pub fn scale_columns(cols: &[f64], useful: f64) -> Vec<f64> {
    let total: f64 = cols.iter().sum();
    if (total - useful).abs() < 0.5 || total <= 0.0 {
        return cols.to_vec();
    }
    let mut out: Vec<f64> = cols.iter().map(|c| (c * useful / total).floor()).collect();
    let left = (useful - out.iter().sum::<f64>()).round() as usize;
    for i in 0..left {
        let n = out.len();
        out[i % n] += 1.0;
    }
    out
}

/// What `node.get` is asked for when reading the set.
const SET_FIELDS: [&str; 8] = ["absoluteBounds", "children", "layout", "sizing", "strokes", "strokeWeight", "strokeAlign", "pluginData"];

fn arr4(v: Option<&Value>) -> [f64; 4] {
    let a: Vec<f64> = v.and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
    match a.as_slice() {
        [t, r, b, l] => [*t, *r, *b, *l],
        [v] => [*v; 4],
        _ => [0.0; 4],
    }
}

/// Read the set (or the component) that goes into the Master, in one call.
pub fn read_set(lang: Lang, holder: u64, is_set: bool) -> Result<SetInfo> {
    let node = host(lang, "node.get", &json!({"id": holder, "fields": SET_FIELDS}))?;
    let set = bounds(&node).unwrap_or_default();
    let cells: Vec<Cell> = if is_set {
        node.get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|c| Some(Cell { id: c.get("id").and_then(Value::as_u64)?, x: bounds(c)?.x - set.x, y: bounds(c)?.y - set.y, w: bounds(c)?.w, h: bounds(c)?.h }))
            .collect()
    } else {
        Vec::new()
    };
    let layout = node.get("layout").filter(|l| l.get("mode").and_then(Value::as_str).is_some_and(|m| m != "none"));
    let auto = layout.map(|l| Auto {
        horizontal: l.get("mode").and_then(Value::as_str) == Some("horizontal"),
        wrap: l.get("wrap").and_then(Value::as_bool).unwrap_or(false),
        padding: arr4(l.get("padding")),
        gap: num(l, "gap").unwrap_or(0.0),
        cross_gap: num(l, "crossGap").unwrap_or(0.0),
    });
    let sizing = node.get("sizing");
    let side = |k: &str| sizing.and_then(|s| s.get(k)).and_then(Value::as_str);
    let stroked = node.get("strokes").and_then(Value::as_array).is_some_and(|s| !s.is_empty());
    let included = layout.and_then(|l| l.get("strokesIncluded")).and_then(Value::as_bool).unwrap_or(true);
    let inside = node.get("strokeAlign").and_then(Value::as_str).is_none_or(|a| a == "inside");
    let inset = if stroked && included && inside { num(&node, "strokeWeight").unwrap_or(1.0) } else { 0.0 };
    Ok(SetInfo { width: set.w, height: set.h, cells, auto, fixed_w: side("horizontal") == Some("fixed"), hug_h: side("vertical") == Some("hug"), inset, marked: mark_of(&node, ORGANIZED_MARK).is_some() })
}

/// Read the set, decide, write what the plan says and read it back. Never places a variant by x, y. When the host does not keep what was
/// written, the outcome says so and the Master widens to the set as it is.
pub fn organize(lang: Lang, holder: u64, is_set: bool) -> Result<Outcome> {
    let before = read_set(lang, holder, is_set)?;
    let plan = plan(&before, AVAILABLE_W);
    let differs = order_differs(&before.cells);
    let mut out = Outcome {
        plan: plan.name(), frame_w: plan.frame_width(), wide: plan.wide(), failed: None, organized: false, marked: before.marked, width: before.width, height: before.height,
        rows: per_row(&before.cells), variants: before.cells.len(), order_differs: differs, preserved: None,
    };
    if let Plan::Unsupported { .. } = plan {
        out.failed = Some("the set has an auto layout that is not horizontal".into());
        return Ok(out);
    }
    let Some(want) = plan.organize() else { return Ok(out) };
    out.preserved = want.kept.clone();
    // The write: one call. A refused property fails the call; either way the Master widens to the set as it is.
    let written = host(lang, "node.set", &json!({"ids": [holder], "props": want.props()}));
    let after = read_set(lang, holder, is_set)?;
    out.width = after.width;
    out.height = after.height;
    out.rows = per_row(&after.cells);
    match written.map_err(|e| e.to_string()).and_then(|_| check(&after, want)) {
        Ok(()) => {
            out.organized = true;
            out.marked = true;
            // A set that had to grow keeps the width that was asked for.
            if let Plan::Wide { .. } = plan {
                out.frame_w = FRAME_W.max(after.width + WIDE_EXTRA);
            }
        }
        Err(why) => {
            out.failed = Some(why);
            out.plan = "failed";
            out.wide = true;
            out.frame_w = FRAME_W.max(after.width + WIDE_EXTRA);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn cell(id: u64, x: f64, y: f64, w: f64, h: f64) -> Cell {
        Cell { id, x, y, w, h }
    }

    /// The Card of the demo: three variants side by side, 1528 wide (20 + 384 + 156 + 320 + 200 + 448), no auto layout, a 1 px stroke.
    fn card() -> SetInfo {
        SetInfo { width: 1528.0, height: 462.0, cells: vec![cell(667, 20.0, 20.0, 384.0, 422.0), cell(630, 540.0, 20.0, 320.0, 238.0), cell(704, 1060.0, 20.0, 448.0, 382.0)], inset: 1.0, ..SetInfo::default() }
    }

    /// A set the person made with auto layout: six 300 wide variants in a row, padding 24, gap 24, row gap 10.
    fn auto_row() -> SetInfo {
        let cells = (0..6).map(|i| cell(i + 1, 25.0 + i as f64 * 324.0, 25.0, 300.0, 72.0)).collect();
        let a = Auto { horizontal: true, wrap: false, padding: [24.0; 4], gap: 24.0, cross_gap: 10.0 };
        SetInfo { width: 1970.0, height: 122.0, cells, auto: Some(a), fixed_w: false, hug_h: true, inset: 1.0, marked: false }
    }

    #[test]
    fn the_room_is_988_in_a_1180_frame() {
        assert_eq!(AVAILABLE_W, 988.0);
        assert_eq!(WIDE_EXTRA, 192.0);
    }

    #[test]
    fn a_set_that_fits_is_not_touched() {
        let mut s = card();
        s.width = 988.0;
        assert_eq!(plan(&s, AVAILABLE_W), Plan::Keep);
        s.width = 400.0;
        assert_eq!(plan(&s, AVAILABLE_W), Plan::Keep);
        assert_eq!(plan(&SetInfo { width: 1528.0, ..SetInfo::default() }, AVAILABLE_W).frame_width(), 1528.0 + 192.0, "a single component that is too wide: no layout, the Master grows");
    }

    #[test]
    fn a_set_that_does_not_fit_gets_horizontal_auto_layout_with_wrap_padding_24_gaps_48_fixed_988_and_hug_height() {
        let Plan::Organize(o) = plan(&card(), AVAILABLE_W) else { panic!("the Card does not fit in 988") };
        assert_eq!(o.width, 988.0);
        assert!(o.kept.is_none());
        let p = o.props();
        assert_eq!((p["layout"].clone(), p["wrap"].clone(), p["padding"].clone(), p["gap"].clone(), p["crossGap"].clone()), (json!("horizontal"), json!(true), json!(24.0), json!(48.0), json!(48.0)));
        assert_eq!((p["sizingH"].clone(), p["sizingV"].clone(), p["width"].clone()), (json!("fixed"), json!("hug"), json!(988.0)));
        assert_eq!(Plan::Organize(o).frame_width(), 1180.0, "the frame stays 1180");
    }

    #[test]
    fn a_set_with_auto_layout_keeps_its_padding_and_gaps_and_gets_only_wrap_width_and_hug() {
        let Plan::Organize(o) = plan(&auto_row(), AVAILABLE_W) else { panic!("1970 does not fit in 988") };
        assert_eq!(o.kept.as_ref().map(|a| (a.padding, a.gap, a.cross_gap)), Some(([24.0; 4], 24.0, 10.0)));
        let p = o.props();
        for k in ["layout", "padding", "gap", "crossGap"] {
            assert!(p.get(k).is_none(), "{k} stays the person's");
        }
        assert_eq!((p["wrap"].clone(), p["sizingH"].clone(), p["sizingV"].clone(), p["width"].clone()), (json!(true), json!("fixed"), json!("hug"), json!(988.0)));
    }

    #[test]
    fn an_auto_layout_that_is_not_horizontal_is_not_written_and_the_master_grows() {
        let mut s = auto_row();
        s.auto.as_mut().unwrap().horizontal = false;
        let p = plan(&s, AVAILABLE_W);
        assert_eq!(p, Plan::Unsupported { width: 1970.0 });
        assert!(p.wide());
        assert_eq!(p.frame_width(), 1970.0 + 192.0);
    }

    #[test]
    fn when_one_variant_is_wider_than_a_line_the_set_is_as_wide_as_it_needs_and_the_master_grows() {
        // Two 1100 variants: 24 + 1100 + 24 + 2 x 1 (the stroke) = 1150 > 988.
        let s = SetInfo { width: 2300.0, height: 300.0, cells: vec![cell(1, 25.0, 25.0, 1100.0, 200.0), cell(2, 1173.0, 25.0, 1100.0, 200.0)], inset: 1.0, ..SetInfo::default() };
        let p = plan(&s, AVAILABLE_W);
        let Plan::Wide { width, organize } = &p else { panic!() };
        assert_eq!(*width, 1150.0);
        assert_eq!(organize.as_ref().unwrap().width, 1150.0);
        assert_eq!(p.frame_width(), 1150.0 + 192.0);
        // A variant of 938 plus the padding and the stroke is exactly 988: it fits in a line and the frame stays 1180.
        let tight = SetInfo { width: 2000.0, cells: vec![cell(1, 25.0, 25.0, 938.0, 100.0), cell(2, 1000.0, 25.0, 200.0, 100.0)], inset: 1.0, ..SetInfo::default() };
        assert_eq!(plan(&tight, AVAILABLE_W).frame_width(), 1180.0);
    }

    #[test]
    fn what_the_host_gives_back_is_checked_against_what_was_asked() {
        let want = Organize { width: 988.0, kept: None };
        let ok = SetInfo {
            width: 988.0,
            height: 400.0,
            cells: vec![cell(1, 25.0, 25.0, 320.0, 100.0), cell(2, 393.0, 25.0, 320.0, 100.0)],
            auto: Some(Auto { horizontal: true, wrap: true, padding: [24.0; 4], gap: 48.0, cross_gap: 48.0 }),
            fixed_w: true,
            hug_h: true,
            inset: 1.0,
            marked: true,
        };
        assert_eq!(check(&ok, &want), Ok(()));
        let mut no_wrap = ok.clone();
        no_wrap.auto.as_mut().unwrap().wrap = false;
        assert_eq!(check(&no_wrap, &want), Err("wrap is off".to_string()));
        let mut no_layout = ok.clone();
        no_layout.auto = None;
        assert!(check(&no_layout, &want).is_err());
        let mut not_hug = ok.clone();
        not_hug.hug_h = false;
        assert!(check(&not_hug, &want).unwrap_err().contains("sizing"));
        let mut narrow = ok.clone();
        narrow.width = 1970.0;
        assert!(check(&narrow, &want).unwrap_err().contains("width"));
        let mut outside = ok.clone();
        outside.cells.push(cell(3, 900.0, 25.0, 320.0, 100.0));
        assert_eq!(check(&outside, &want), Err("variant 3 is outside the set".to_string()), "a variant that still sticks out of the set is a failure");
    }

    #[test]
    fn the_lines_are_counted_top_to_bottom_and_a_changed_layer_order_is_told() {
        // The Card laid out by the host: 320, 320 | 320, 480 | 480 | 480 | 640 | 640 | 640 (seven lines).
        let ys = [25.0, 25.0, 439.0, 439.0, 853.0, 1227.0, 1601.0, 1955.0, 2309.0];
        let xs = [25.0, 393.0, 25.0, 393.0, 25.0, 25.0, 25.0, 25.0, 25.0];
        let cells: Vec<Cell> = (0..9).map(|i| cell(i as u64 + 1, xs[i], ys[i], 320.0, 300.0)).collect();
        assert_eq!(per_row(&cells), vec![2, 2, 1, 1, 1, 1, 1]);
        assert!(!order_differs(&cells), "the layers are in the order they are read");
        let mut shuffled = cells.clone();
        shuffled.swap(0, 3);
        assert!(order_differs(&shuffled));
        // 0.4 px of difference in y is the same line.
        assert_eq!(per_row(&[cell(1, 0.0, 0.0, 10.0, 10.0), cell(2, 50.0, 0.4, 10.0, 10.0)]), vec![2]);
    }

    #[test]
    fn organizing_twice_changes_nothing() {
        // After the write the set is 988 wide: it fits, so the plan of the second run is to keep it.
        let mut after = card();
        after.width = 988.0;
        assert_eq!(plan(&after, AVAILABLE_W), Plan::Keep);
        let mut auto_after = auto_row();
        auto_after.width = 988.0;
        assert_eq!(plan(&auto_after, AVAILABLE_W), Plan::Keep);
    }

    #[test]
    fn table_columns_scale_to_a_wider_frame_in_whole_pixels_that_add_up() {
        let cols = [240.0, 140.0, 460.0, 244.0];
        assert_eq!(scale_columns(&cols, 1084.0), cols.to_vec());
        for useful in [1300.0, 1500.0, 1777.0] {
            let s = scale_columns(&cols, useful);
            assert_eq!(s.iter().sum::<f64>(), useful);
            assert!(s.iter().all(|c| c.fract() == 0.0));
        }
        assert_eq!(scale_columns(&[160.0, 180.0, 744.0], 1084.0 * 2.0).iter().sum::<f64>(), 2168.0);
    }
}
