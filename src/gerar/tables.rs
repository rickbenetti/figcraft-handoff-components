//! The four frames of 0.11 part B, drawn as tables in the Section's frames (1920 wide, padding 48, 1824 useful): Properties, Anatomy
//! (the stage with the numbered layers, and a table of them), States and Variants (one matrix per variant axis) and Layouts
//! (padding and gap by layer). They reuse what the 0.10.0 report collected (the audit of a layer's properties, the comparison of
//! a variant with the default, the variant axes) and draw it as tables in the palette of the Section.

#![warn(dead_code)] // the module above allows dead code (the 0.10.0 drawing); this one is checked

use super::section::{Ctx, Cv, StageChoice, USEFUL_W, Walk, bold, header, judge, pal, pick_stage_for, table, table_nodes};

/// A table of the frame, noted for the answer (its name and how many rows it has).
fn put_table(c: &Ctx, parent: u64, name: &str, cols: &[f64], headers: &[String], rows: &[Vec<Cv>]) -> Result<u64> {
    c.tables.borrow_mut().push((name.to_string(), rows.len()));
    table(c.lang, parent, name, cols, headers, rows)
}
use super::*;
use crate::delta::{Diff, Kind, Label, Swatch};
use crate::variants::Axis;

/// The columns (they add up to 1824).
const PROPS_COLS: [f64; 4] = [360.0, 200.0, 900.0, 364.0];
const ANATOMY_COLS: [f64; 4] = [80.0, 160.0, 380.0, 1204.0];
const LAYOUT_COLS: [f64; 4] = [300.0, 200.0, 700.0, 624.0];
/// The first two columns of a matrix; the options share what is left.
const MATRIX_ELEMENT_W: f64 = 200.0;
const MATRIX_PROPERTY_W: f64 = 280.0;
/// The most options a matrix shows side by side; more are stacked in tables of the same frame.
const MATRIX_SLICE: usize = 5;
/// The most nodes the matrices of one run may make.
pub const MATRIX_NODE_CAP: usize = 400;
/// The most nodes the table of Layouts may make (about 10 layers of five rows): a component with hundreds of layers with spacing
/// would otherwise spend the run's whole budget on one frame.
pub const LAYOUT_NODE_CAP: usize = 400;
/// Longest value text in a matrix cell.
const CELL_CHARS: usize = 60;

fn muted_note(lang: Lang, parent: u64, text: &str) -> Result<u64> {
    text_with(lang, parent, text, LINE_SIZE, pal::GUIDE, "Note", json!({"width": USEFUL_W, "textAutoResize": "height"}))
}

fn heads(lang: Lang, keys: &[Key]) -> Vec<String> {
    keys.iter().map(|k| t(lang, *k).to_string()).collect()
}

fn cut(s: &str) -> String {
    if s.chars().count() <= CELL_CHARS { s.to_string() } else { format!("{}…", s.chars().take(CELL_CHARS - 1).collect::<String>()) }
}

/// An instance of `component` in a temporary frame far off the page, walked as the 0.10.0 report walked the instance it audited (an
/// instance shows what its nested instances have overridden; the component does not). The instance is deleted again.
fn walk_instance(lang: Lang, stage: u64, component: u64) -> Result<Vec<(Value, usize)>> {
    let made = host(lang, "instance.create", &json!({"component": component, "parent": stage}))?;
    let instance = id_of(lang, &made, "instance.create")?;
    let walked = walk_layers(lang, instance)?;
    host(lang, "edit.delete", &json!({"ids": [instance]}))?;
    Ok(walked)
}

/// The walk of the default variant's instance: the one Anatomy already made in this run, else a new one (kept for the run).
fn base_walk(c: &Ctx) -> Result<Walk> {
    if let Some(w) = c.base_walk.borrow().as_ref() {
        return Ok(w.clone());
    }
    let temp = temp_stage(c.lang)?;
    let walked = walk_instance(c.lang, temp, c.target.component)?;
    host(c.lang, "edit.delete", &json!({"ids": [temp]}))?;
    let walked = std::rc::Rc::new(walked);
    *c.base_walk.borrow_mut() = Some(walked.clone());
    Ok(walked)
}

/// The temporary frame the instances of a frame's data are made in.
fn temp_stage(lang: Lang) -> Result<u64> {
    let r = host(lang, "frame.create", &json!({"x": -30_000, "y": STAGING_Y, "width": 2_000, "height": 2_000, "name": "Handoff · temporary", "select": false, "props": {"layout": "vertical", "sizingH": "fixed", "sizingV": "fixed"}}))?;
    id_of(lang, &r, "frame.create")
}

// ---------------------------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------------------------

/// The Properties frame: a table of the component's properties (name, type, default, options).
pub(super) fn build_properties(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    header(c, frame, t(lang, Key::SecProperties))?;
    let views = property_list(c)?;
    if views.is_empty() {
        muted_note(lang, frame, t(lang, Key::PropsNone))?;
        return Ok(());
    }
    let rows: Vec<Vec<Cv>> = views
        .iter()
        .map(|v| {
            let default = v.options.iter().find(|(_, d)| *d).map(|(o, _)| o.clone());
            let options = if v.options.len() > 1 { v.options.iter().map(|(o, _)| o.as_str()).collect::<Vec<_>>().join(", ") } else { "—".to_string() };
            vec![Cv::Bold(v.name.clone()), Cv::Muted(type_text(lang, &v.kind)), Cv::Text(default.unwrap_or_else(|| "—".to_string())), Cv::Muted(options)]
        })
        .collect();
    let headers = heads(lang, &[Key::TblProperty, Key::TblType, Key::LegendDefault, Key::TblOptions]);
    put_table(c, frame, "Properties", &PROPS_COLS, &headers, &rows)?;
    Ok(())
}

/// The component's properties as the 0.10.0 report listed them.
fn property_list(c: &Ctx) -> Result<Vec<PropView>> {
    let lang = c.lang;
    let defs = host(lang, "component.properties", &json!({"id": c.target.component})).ok();
    let mut component_name = |id: u64| host(lang, "node.get", &json!({"id": id, "fields": ["name"]})).ok().and_then(|n| n.get("name").and_then(Value::as_str).map(ToString::to_string));
    Ok(defs.as_ref().map(|d| property_views(lang, d, c.target.set.as_ref().map(|sp| &sp.default), &mut component_name)).unwrap_or_default())
}

// ---------------------------------------------------------------------------------------------
// Layouts
// ---------------------------------------------------------------------------------------------

/// What a layer's auto layout says about its spacing: the four sides of its padding and the gap, with their tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct Spacing {
    pub name: String,
    /// Top, right, bottom, left, gap: (token, value).
    pub sides: [(Option<String>, String); 5],
}

/// One layer's spacing from its node and the tokens the audit found on it (`pad_v` is the vertical padding's, `pad_h` the horizontal
/// one's, `gap` the gap's). `None` when the layer has no auto layout, or no padding and no gap and no token.
pub fn spacing_of(node: &Value, pad_v: Option<String>, pad_h: Option<String>, gap_token: Option<String>) -> Option<Spacing> {
    let layout = node.get("layout").filter(|l| matches!(l.get("mode").and_then(Value::as_str), Some("horizontal" | "vertical" | "grid")))?;
    let pad: Vec<f64> = layout.get("padding").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
    let [top, right, bottom, left] = pad[..] else { return None };
    let gap = layout.get("gap").and_then(Value::as_f64).unwrap_or(0.0);
    let any = [top, right, bottom, left, gap].iter().any(|v| v.abs() > 0.005) || pad_v.is_some() || pad_h.is_some() || gap_token.is_some();
    if !any {
        return None;
    }
    let px = |v: f64| format!("{}px", audit::number(v));
    Some(Spacing {
        name: node.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        sides: [(pad_v.clone(), px(top)), (pad_h.clone(), px(right)), (pad_v, px(bottom)), (pad_h, px(left)), (gap_token, px(gap))],
    })
}

/// Layers that are alike (the same name and the same spacing) are one, with how many they are.
pub fn merge_spacing(list: Vec<Spacing>) -> Vec<(Spacing, usize)> {
    let mut out: Vec<(Spacing, usize)> = Vec::new();
    for s in list {
        match out.iter_mut().find(|(o, _)| *o == s) {
            Some((_, n)) => *n += 1,
            None => out.push((s, 1)),
        }
    }
    out
}

/// The table rows of the spacing of the layers: five rows (Top, Right, Bottom, Left, Gap) for each layer.
pub fn layout_rows(lang: Lang, layers: &[(Spacing, usize)]) -> Vec<Vec<Cv>> {
    let sides = [Key::SideTop, Key::SideRight, Key::SideBottom, Key::SideLeft, Key::PropGap];
    layers
        .iter()
        .flat_map(|(s, n)| {
            let name = if *n > 1 { format!("{} (x{n})", s.name) } else { s.name.clone() };
            sides.iter().zip(&s.sides).map(move |(side, (token, value))| {
                vec![Cv::Bold(name.clone()), Cv::Muted(t(lang, *side).to_string()), token.clone().map_or_else(|| Cv::Guide("—".to_string()), Cv::Text), Cv::Text(value.clone())]
            })
        })
        .collect()
}

/// The rows of the layers that fit in [`LAYOUT_NODE_CAP`] nodes (a layer's five rows go together), and how many rows were left out.
pub fn capped_layout_rows(lang: Lang, layers: &[(Spacing, usize)]) -> (Vec<Vec<Cv>>, usize) {
    let all = layout_rows(lang, layers);
    let mut used = table_nodes(4, &[]);
    let mut taken = 0;
    for group in all.chunks(5) {
        let cost: usize = group.iter().map(|r| 1 + r.iter().map(Cv::nodes).sum::<usize>()).sum();
        if used + cost > LAYOUT_NODE_CAP {
            break;
        }
        used += cost;
        taken += group.len();
    }
    let left = all.len() - taken;
    (all.into_iter().take(taken).collect(), left)
}

/// The Layouts frame: a table of the padding and gap of each layer of the default variant, with their tokens.
pub(super) fn build_layouts(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    header(c, frame, t(lang, Key::SecLayouts))?;
    let walked = base_walk(c)?;
    let nodes: Vec<Value> = walked.iter().map(|(n, _)| n.clone()).collect();
    let vars = c.vars_for(&nodes)?;
    let styles = Styles::default();
    let drawn = rendered(&walked);
    let mut list: Vec<Spacing> = Vec::new();
    for (i, (n, _)) in walked.iter().enumerate() {
        if !drawn.get(i).copied().unwrap_or(false) {
            continue;
        }
        // Only a layer with auto layout has padding or a gap: the others are not audited (a row costs fuel in a big file).
        if !matches!(n.get("layout").and_then(|l| l.get("mode")).and_then(Value::as_str), Some("horizontal" | "vertical" | "grid")) {
            continue;
        }
        let rows = audit::rows_for(lang, n, &vars, &styles);
        let token = |p: Prop| rows.iter().find(|r| r.property == p && r.state != State::Unbound).map(|r| r.token.clone());
        if let Some(mut s) = spacing_of(n, token(Prop::PadV), token(Prop::PadH), token(Prop::Gap)) {
            // The root is the component itself: its name, not the name of the variant that was walked.
            if i == 0 {
                s.name = c.target.title.clone();
            }
            list.push(s);
        }
    }
    let merged = merge_spacing(list);
    if merged.is_empty() {
        muted_note(lang, frame, t(lang, Key::LayoutsNone))?;
        return Ok(());
    }
    let (rows, left_out) = capped_layout_rows(lang, &merged);
    let headers = heads(lang, &[Key::TblLayer, Key::TblSide, Key::TblToken, Key::TblValue]);
    put_table(c, frame, "Layouts", &LAYOUT_COLS, &headers, &rows)?;
    if left_out > 0 {
        muted_note(lang, frame, &tf(lang, plural(left_out, Key::MatrixLeftOutOne, Key::MatrixLeftOut), &[("n", &left_out.to_string())]))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// States and Variants
// ---------------------------------------------------------------------------------------------

/// What a row of a matrix is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rk {
    Label(Label),
    Shown,
    Swap,
}

/// A row of an axis's matrix: the element, the property and a cell for each option.
#[derive(Clone, Debug, PartialEq)]
pub struct Mrow {
    pub element: String,
    pub property: String,
    pub cells: Vec<Cv>,
}

fn hex_of(value: &str) -> String {
    let rest = value.strip_prefix('#').unwrap_or_default();
    let n = rest.chars().take_while(char::is_ascii_hexdigit).count();
    format!("#{}", &rest[..n.min(8)])
}

fn side_cell(lang: Lang, label: Label, side: &delta::Side) -> Cv {
    let c = delta::cell_of(side, label, lang);
    let swatch = c.colour.map(|s| (hex_of(&c.value), s == Swatch::Hollow));
    let text = cut(&c.value);
    if c.token.is_none() && swatch.is_none() { Cv::Text(text) } else { Cv::Val { token: c.token, text, swatch } }
}

/// The rows of the matrix of an axis. `options[i]` is what option `i` changes from the default option (`None` for the default
/// itself, which is the base). A row is a layer and a property that some option changes; its cell in an option that doesn't change
/// it is the default's value. `root` names the component's own box (the layer `None`).
pub fn matrix(lang: Lang, root: &str, options: &[Option<&Diff>]) -> Vec<Mrow> {
    struct Row {
        layer: Option<String>,
        rk: Rk,
        base: Cv,
        cells: Vec<Option<Cv>>,
    }
    let mut rows: Vec<Row> = Vec::new();
    for (i, diff) in options.iter().enumerate() {
        let Some(diff) = diff else { continue };
        for ch in &diff.changes {
            let (rk, base, cell) = match &ch.kind {
                Kind::Changed { label, from, to } => (Rk::Label(*label), side_cell(lang, *label, from), side_cell(lang, *label, to)),
                Kind::Shown(on) => {
                    let word = |v: bool| Cv::Text(t(lang, if v { Key::DeltaVisible } else { Key::DeltaHidden }).to_string());
                    (Rk::Shown, word(!*on), word(*on))
                }
                Kind::Swapped(text) => (Rk::Swap, Cv::Text(delta::DASH.to_string()), Cv::Text(cut(text))),
            };
            let at = match rows.iter().position(|r| r.layer == ch.layer && r.rk == rk) {
                Some(p) => p,
                None => {
                    rows.push(Row { layer: ch.layer.clone(), rk, base, cells: vec![None; options.len()] });
                    rows.len() - 1
                }
            };
            rows[at].cells[i] = Some(cell);
        }
    }
    rows.into_iter()
        .map(|r| Mrow {
            element: r.layer.as_deref().map_or_else(|| root.to_string(), delta::title),
            property: match r.rk {
                Rk::Label(l) => t(lang, l.key()).to_string(),
                Rk::Shown => t(lang, Key::PropVisibility).to_string(),
                Rk::Swap => t(lang, Key::TypeInstance).to_string(),
            },
            cells: r.cells.into_iter().map(|c| c.unwrap_or_else(|| r.base.clone())).collect(),
        })
        .collect()
}

/// How many rows of a matrix fit in `left` nodes when it is drawn in `tables` tables of `heads` header cells each, and what they cost.
pub fn rows_that_fit(rows: &[Mrow], tables: usize, heads: usize, left: usize) -> (usize, usize) {
    let base = tables * table_nodes(heads, &[]);
    if base > left {
        return (0, 0);
    }
    let mut used = base;
    let mut taken = 0;
    for r in rows {
        let cost = tables * (1 + r.cells.iter().map(Cv::nodes).sum::<usize>() + 2 * 2 + 1);
        if used + cost > left {
            break;
        }
        used += cost;
        taken += 1;
    }
    (taken, used)
}

/// The columns of a matrix of `n` options.
fn matrix_cols(n: usize) -> Vec<f64> {
    let each = (USEFUL_W - MATRIX_ELEMENT_W - MATRIX_PROPERTY_W) / n as f64;
    let mut cols = vec![MATRIX_ELEMENT_W, MATRIX_PROPERTY_W];
    cols.extend(std::iter::repeat_n(each, n));
    cols
}

/// The States and Variants frame: one matrix for each variant axis.
pub(super) fn build_variants(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    header(c, frame, t(lang, Key::SecVariants))?;
    let Some(sp) = &c.target.set else {
        muted_note(lang, frame, t(lang, Key::VariantsNone))?;
        return Ok(());
    };
    let views = property_list(c)?;
    let order: Vec<(String, Vec<String>)> = views.iter().filter(|v| v.kind == "variant").map(|v| (v.name.clone(), v.options.iter().map(|(o, _)| o.clone()).collect())).collect();
    let axes: Vec<Axis> = variants::axes(&sp.info, &sp.default, &order);
    if axes.is_empty() {
        muted_note(lang, frame, t(lang, Key::VariantsNone))?;
        return Ok(());
    }
    let base_walked = base_walk(c)?;
    let temp = temp_stage(lang)?;
    let base_nodes: Vec<Value> = base_walked.iter().map(|(n, _)| n.clone()).collect();
    let vars = c.vars_for(&base_nodes)?;
    let styles = c.styles_for(&base_nodes)?;
    let base_snap = delta::snapshot(lang, &base_walked, &vars, &styles);
    let mut left = MATRIX_NODE_CAP;
    let mut left_out = 0usize;
    let mut walked_options = 0usize;
    let mut not_shown = 0usize;
    for axis in &axes {
        // The diff of each option that has a variant (the default has none: it is the base).
        let mut present: Vec<(String, bool, Option<Diff>)> = Vec::new();
        let mut missing: Vec<String> = Vec::new();
        for opt in &axis.options {
            let Some(variant) = &opt.variant else {
                missing.push(opt.option.clone());
                continue;
            };
            if opt.is_default {
                present.push((opt.option.clone(), true, None));
                continue;
            }
            if walked_options >= MAX_AXIS_ROWS {
                not_shown += 1;
                continue;
            }
            walked_options += 1;
            let walked = walk_instance(lang, temp, variant.id)?;
            let diff = delta::compare(&base_snap, &delta::snapshot(lang, &walked, &vars, &styles));
            present.push((opt.option.clone(), false, Some(diff)));
        }
        let diffs: Vec<Option<&Diff>> = present.iter().map(|(_, _, d)| d.as_ref()).collect();
        let rows = matrix(lang, &c.target.title, &diffs);
        let ax_name = tf(lang, Key::LayerAxis, &[("name", &axis.prop)]);
        let mut ax = FlowSpec::new(&ax_name, USEFUL_W, "vertical");
        ax.gap = 16.0;
        let ax = flow_frame(lang, frame, ax)?;
        text_with(lang, ax, &axis.prop, theme::SECTION_SIZE, pal::TEXT, "Axis", bold())?;
        let slices: Vec<&[(String, bool, Option<Diff>)]> = present.chunks(MATRIX_SLICE).collect();
        let (taken, used) = rows_that_fit(&rows, slices.len().max(1), 2 + MATRIX_SLICE.min(present.len()), left);
        left = left.saturating_sub(used);
        left_out += rows.len() - taken;
        if rows.is_empty() {
            muted_note(lang, ax, t(lang, Key::DeltaNone))?;
        }
        if taken > 0 {
            for (si, slice) in slices.iter().enumerate() {
                let first = si * MATRIX_SLICE;
                let mut headers = heads(lang, &[Key::TblElement, Key::TblProperty]);
                headers.extend(slice.iter().map(|(name, is_default, _)| if *is_default { format!("{name} · {}", t(lang, Key::LegendDefault)) } else { name.clone() }));
                let body: Vec<Vec<Cv>> = rows[..taken]
                    .iter()
                    .map(|r| {
                        let mut cells = vec![Cv::Bold(r.element.clone()), Cv::Muted(r.property.clone())];
                        cells.extend(r.cells[first..first + slice.len()].iter().cloned());
                        cells
                    })
                    .collect();
                put_table(c, ax, &format!("{} {}", axis.prop, si + 1), &matrix_cols(slice.len()), &headers, &body)?;
            }
        }
        if !missing.is_empty() {
            muted_note(lang, ax, &tf(lang, Key::NotDefined, &[("list", &missing.join(", "))]))?;
        }
    }
    host(lang, "edit.delete", &json!({"ids": [temp]}))?;
    if not_shown > 0 {
        muted_note(lang, frame, &tf(lang, plural(not_shown, Key::MoreVariantsOne, Key::MoreVariants), &[("n", &not_shown.to_string())]))?;
    }
    if left_out > 0 {
        muted_note(lang, frame, &tf(lang, plural(left_out, Key::MatrixLeftOutOne, Key::MatrixLeftOut), &[("n", &left_out.to_string())]))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Anatomy
// ---------------------------------------------------------------------------------------------

/// The rows of the Anatomy table: number, type, layer and note ("—" when the layer has none).
pub fn anatomy_row(number: usize, kind: &str, name: &str, layers: usize, content: Option<&str>, sub: Option<&str>) -> Vec<Cv> {
    let mut layer = name.to_string();
    if layers > 1 {
        layer = format!("{layer} (x{layers})");
    }
    if let Some(c) = content {
        layer = format!("{layer} {c}");
    }
    vec![Cv::Bold(number.to_string()), Cv::Muted(kind.to_string()), Cv::Text(layer), match sub {
        Some(s) if !s.trim().is_empty() => Cv::Muted(s.to_string()),
        _ => Cv::Guide("—".to_string()),
    }]
}

/// The Anatomy frame: the stage with the component in the middle and a number on each part, and a table of the numbers. Returns
/// whether the automatic stage left a layer under 3:1.
pub(super) fn build_anatomy(c: &Ctx, frame: u64) -> Result<bool> {
    let lang = c.lang;
    let target = c.target;
    header(c, frame, t(lang, Key::SecAnatomy))?;
    let size = bounds(&target.node).unwrap_or(Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 });
    let stage_w = USEFUL_W;
    // The stage, far off the page (an instance can only be placed in a top-level frame); it moves into the frame when it is done.
    let h0 = (size.h + STAGE_ROOM).max(MIN_STAGE_H);
    let sb0 = Rect { x: -30_000.0, y: STAGING_Y, w: stage_w, h: h0 };
    let stage = make_stage(lang, sb0, false)?;
    let (instance, ib0) = place_instance(lang, target.component, stage, sb0, size)?;
    let walked = walk_layers(lang, instance)?;
    *c.base_walk.borrow_mut() = Some(std::rc::Rc::new(walked.clone()));
    let mut nodes: Vec<Value> = walked.iter().map(|(n, _)| n.clone()).collect();
    let vars = c.vars_for(&nodes)?;
    let styles = c.styles_for(&nodes)?;
    let mut all_rows: Vec<Row> = Vec::new();
    for (i, (n, depth)) in walked.iter().enumerate() {
        for mut r in audit::rows_for(lang, n, &vars, &styles) {
            r.node = i;
            r.depth = *depth;
            all_rows.push(r);
        }
    }
    let drawn = rendered(&walked);
    let reach = effect_reach(effect_overflow(&walked, &drawn));
    let all_rows = audit::keep_shown(all_rows, &drawn);
    let mut rows = audit::drop_zero_unbound(all_rows);
    audit::mark_checks(&mut rows);
    let infos = layer_infos(lang, &walked, &drawn)?;
    let grouped = audit::group_layers(&rows, &infos);
    let (shown, _hidden) = limit_groups(&grouped, MAX_AUDIT_LINES);
    let mut blocks_list = blocks(&shown, |l| !is_spacing(l.property), true);
    let unnumbered = number_blocks(&mut blocks_list, &drawn);
    // The stage grows when the effects need room, and the instance stays in the middle.
    let h1 = h0.max(stage_height_for_effects(size.h, reach));
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
    // The stage's colour: the Stage setting, or (Auto) the one of light, mid-grey and dark that suits this variant best.
    let choice: StageChoice = pick_stage_for(c.mode, std::slice::from_ref(&judge(&nodes)), set_backdrop(lang, target)?);
    host(lang, "node.set", &json!({"ids": [stage], "props": {"fill": choice.fill(), "stroke": theme::BORDER, "strokeWeight": 1, "strokeAlign": "inside", "width": stage_w}}))?;
    let red = red_style(choice.stage != contrast::Stage::Light);
    let tree = Tree::new(&nodes, walked.iter().map(|(_, d)| *d).collect());
    let numbered: Vec<(usize, usize)> = blocks_list.iter().filter_map(|b| b.badge.map(|n| b.nodes.iter().filter(|i| drawn.get(**i).copied().unwrap_or(false)).map(move |i| (n, *i)))).flatten().collect();
    draw_badges(lang, stage, sb, ib, &tree, &numbered, &red)?;
    host(lang, "node.reparent", &json!({"ids": [stage], "parent": frame}))?;
    // The table of the numbers.
    let mut table_rows: Vec<Vec<Cv>> = Vec::new();
    for b in &blocks_list {
        let Some(n) = b.badge else { continue };
        let node = walked.get(b.node).map(|(v, _)| v).unwrap_or(&Value::Null);
        let name = node.get("name").and_then(Value::as_str).unwrap_or_default();
        let kind = kind_word(lang, node.get("type").and_then(Value::as_str).unwrap_or_default());
        let content = group_content(&walked, &b.nodes, name);
        // The line under the title of a part says "Instance of Icon"; one that only repeats the type says nothing new.
        let sub = b.sub.as_deref().filter(|s| *s != kind);
        table_rows.push(anatomy_row(n, &kind, name, b.nodes.len(), content.as_deref(), sub));
    }
    if table_rows.is_empty() {
        muted_note(lang, frame, t(lang, Key::LayoutsNone))?;
    } else {
        let headers = heads(lang, &[Key::TblNumber, Key::TblType, Key::TblLayer, Key::TblNote]);
        put_table(c, frame, "Anatomy", &ANATOMY_COLS, &headers, &table_rows)?;
    }
    if unnumbered > 0 {
        muted_note(lang, frame, &tf(lang, plural(unnumbered, Key::UnnumberedOne, Key::Unnumbered), &[("n", &unnumbered.to_string())]))?;
    }
    Ok(choice.too_low())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{Change, Side};

    fn changed(layer: Option<&str>, label: Label, from: &str, to: &str) -> Change {
        let side = |v: &str| Side { token: None, value: v.to_string() };
        Change { layer: layer.map(ToString::to_string), kind: Kind::Changed { label, from: side(from), to: side(to) }, minor: false }
    }

    #[test]
    fn the_columns_add_up_to_the_useful_width() {
        for cols in [PROPS_COLS, ANATOMY_COLS, LAYOUT_COLS] {
            assert_eq!(cols.iter().sum::<f64>(), USEFUL_W);
        }
        for n in 1..=5 {
            let cols = matrix_cols(n);
            assert_eq!(cols[0], 200.0);
            assert_eq!(cols[1], 280.0);
            assert!((cols.iter().sum::<f64>() - USEFUL_W).abs() < 1e-9, "{n} options");
        }
    }

    #[test]
    fn a_matrix_has_a_row_for_each_layer_and_property_an_option_changes_and_a_column_for_each_option() {
        // Axis State: Enabled (the default), Hover and Pressed change the fill of the root; Pressed also the radius of "Icon".
        let hover = Diff { changes: vec![changed(None, Label::Prop(Prop::Fill), "#C6005C", "#B10652")], unreadable_effect: false };
        let pressed = Diff { changes: vec![changed(None, Label::Prop(Prop::Fill), "#C6005C", "#9E0749"), changed(Some("/Icon"), Label::Prop(Prop::Radius), "4", "8")], unreadable_effect: false };
        let rows = matrix(Lang::En, "Button", &[None, Some(&hover), Some(&pressed)]);
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].element.as_str(), rows[0].property.as_str()), ("Button", "Fill"));
        assert_eq!(rows[0].cells.len(), 3);
        // The default's cell is the value the changes come from; an option that doesn't change it holds the default's value.
        assert!(matches!(&rows[0].cells[0], Cv::Val { text, .. } if text == "#C6005C"));
        assert!(matches!(&rows[0].cells[1], Cv::Val { text, .. } if text == "#B10652"));
        assert_eq!((rows[1].element.as_str(), rows[1].property.as_str()), ("Icon", "Radius"));
        assert_eq!(rows[1].cells, [Cv::Text("4".into()), Cv::Text("4".into()), Cv::Text("8".into())]);
    }

    #[test]
    fn an_axis_that_only_switches_a_layer_on_and_off_has_a_matrix() {
        let on = Diff { changes: vec![Change { layer: Some("/Left icon".into()), kind: Kind::Shown(true), minor: false }], unreadable_effect: false };
        let rows = matrix(Lang::En, "Button", &[None, Some(&on)]);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].element.as_str(), rows[0].property.as_str()), ("Left icon", "Visibility"));
        assert_eq!(rows[0].cells, [Cv::Text("hidden".into()), Cv::Text("visible".into())]);
    }

    #[test]
    fn an_axis_with_no_change_has_no_rows() {
        assert!(matrix(Lang::En, "Button", &[None, Some(&Diff::default())]).is_empty());
    }

    #[test]
    fn the_nodes_a_matrix_may_make_are_capped_and_the_rest_is_counted() {
        let row = |n: usize| Mrow { element: format!("L{n}"), property: "Fill".into(), cells: vec![Cv::Text("a".into()); 3] };
        let rows: Vec<Mrow> = (0..200).map(row).collect();
        let (taken, used) = rows_that_fit(&rows, 1, 5, MATRIX_NODE_CAP);
        assert!(taken > 0 && taken < 200, "{taken}");
        assert!(used <= MATRIX_NODE_CAP, "{used}");
        // Each row costs its cell frame and their texts, and a separator: 1 + 3 x 2 + 1 = 8 here (the cells of the element and the
        // property count too).
        let (all, _) = rows_that_fit(&rows, 1, 5, 1_000_000);
        assert_eq!(all, 200);
        // Stacked tables repeat the rows: with 2 tables fewer rows fit.
        let (two, _) = rows_that_fit(&rows, 2, 5, MATRIX_NODE_CAP);
        assert!(two < taken);
        // No room even for the headers: nothing.
        assert_eq!(rows_that_fit(&rows, 1, 5, 3), (0, 0));
    }

    #[test]
    fn the_table_of_layouts_stops_at_its_node_cap_and_counts_the_rows_left_out() {
        let layer = |n: usize| Spacing { name: format!("L{n}"), sides: [(None, "8px".into()), (None, "8px".into()), (None, "8px".into()), (None, "8px".into()), (None, "4px".into())] };
        let layers: Vec<(Spacing, usize)> = (0..200).map(|n| (layer(n), 1)).collect();
        let (rows, left) = capped_layout_rows(Lang::En, &layers);
        assert_eq!(rows.len() % 5, 0, "a layer's five rows go together");
        assert!(!rows.is_empty() && left > 0 && rows.len() + left == 1000, "{} + {left}", rows.len());
        assert!(table_nodes(4, &rows) <= LAYOUT_NODE_CAP);
        let (few, none) = capped_layout_rows(Lang::En, &layers[..3]);
        assert_eq!((few.len(), none), (15, 0));
    }

    #[test]
    fn the_anatomy_note_is_a_dash_when_there_is_nothing_to_say_and_equal_layers_are_one_row() {
        let plain = anatomy_row(1, "Frame", "Button", 1, None, None);
        assert_eq!(plain[3], Cv::Guide("—".into()));
        let many = anatomy_row(2, "Text", "Label", 3, Some("\"Go\""), Some("Instance of Icon"));
        assert_eq!(many[2], Cv::Text("Label (x3) \"Go\"".into()));
        assert_eq!(many[3], Cv::Muted("Instance of Icon".into()));
        assert_eq!(plain[0], Cv::Bold("1".into()));
    }

    fn node(name: &str, padding: [f64; 4], gap: f64) -> Value {
        json!({"name": name, "layout": {"mode": "horizontal", "padding": padding, "gap": gap}})
    }

    #[test]
    fn a_layer_with_padding_has_five_rows_and_alike_layers_are_merged() {
        let a = spacing_of(&node("Button", [8.0, 16.0, 8.0, 16.0], 4.0), Some("space/2".into()), Some("space/4".into()), None);
        let b = a.clone();
        let none = spacing_of(&node("Flat", [0.0; 4], 0.0), None, None, None);
        assert!(none.is_none(), "no padding, no gap, no token: no row");
        assert!(spacing_of(&json!({"name": "Text"}), None, None, None).is_none(), "no auto layout");
        let merged = merge_spacing(vec![a.unwrap_or_else(unreachable_spacing), b.unwrap_or_else(unreachable_spacing)]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].1, 2);
        let rows = layout_rows(Lang::En, &merged);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0][0], Cv::Bold("Button (x2)".into()));
        assert_eq!(rows[0][1], Cv::Muted("Top".into()));
        assert_eq!(rows[0][2], Cv::Text("space/2".into()));
        assert_eq!(rows[0][3], Cv::Text("8px".into()));
        assert_eq!(rows[1][1], Cv::Muted("Right".into()));
        assert_eq!(rows[1][3], Cv::Text("16px".into()));
        assert_eq!(rows[4][1], Cv::Muted("Gap".into()));
        assert_eq!(rows[4][2], Cv::Guide("—".into()), "no token: a dash");
    }

    fn unreachable_spacing() -> Spacing {
        Spacing { name: String::new(), sides: Default::default() }
    }
}
