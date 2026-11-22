//! The four frames of 0.11 part B, drawn as tables in the Section's frames (1180 wide, padding 48, 1084 useful): Properties, Anatomy
//! (the stage with the numbered layers, and a table of them), States and Variants (one matrix per variant axis) and Layouts
//! (padding and gap by layer). They reuse what the 0.10.0 report collected (the audit of a layer's properties, the comparison of
//! a variant with the default, the variant axes) and draw it as tables in the palette of the Section.

#![warn(dead_code)] // the module above allows dead code (the 0.10.0 drawing); this one is checked

use super::section::{Ctx, Cv, StageChoice, USEFUL_W, Walk, bold, header, judge, pal, pick_stage_for, table};

/// A table of the frame, noted for the answer (its name and how many rows it has).
pub(super) fn put_table(c: &Ctx, parent: u64, name: &str, cols: &[f64], headers: &[String], rows: &[Vec<Cv>]) -> Result<u64> {
    c.tables.borrow_mut().push((name.to_string(), rows.len()));
    table(c.lang, parent, name, &reflow::scale_columns(cols, c.useful.get()), headers, rows)
}
use super::*;
use crate::delta::{Diff, Kind, Label};
use crate::variants::Axis;

/// The columns (they add up to 1084).
const PROPS_COLS: [f64; 4] = [240.0, 140.0, 460.0, 244.0];
const ANATOMY_COLS: [f64; 4] = [60.0, 120.0, 280.0, 624.0];

pub(super) fn muted_note(lang: Lang, parent: u64, text: &str) -> Result<u64> {
    text_with(lang, parent, text, LINE_SIZE, pal::GUIDE, "Note", json!({"width": USEFUL_W, "textAutoResize": "height"}))
}

pub(super) fn heads(lang: Lang, keys: &[Key]) -> Vec<String> {
    keys.iter().map(|k| t(lang, *k).to_string()).collect()
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
pub(super) fn base_walk(c: &Ctx) -> Result<Walk> {
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
// States and Variants: a gallery of instances
// ---------------------------------------------------------------------------------------------

/// The room of a tile around its instance, the least tile width, the radius, and the gap between cells (in a line and between lines).
const TILE_PAD: f64 = 24.0;
const TILE_MIN_W: f64 = 160.0;
const TILE_RADIUS: f64 = 12.0;
const CELL_GAP: f64 = 48.0;
/// The nodes a cell makes besides its instance: the cell, the tile and the two texts (and a third when it has a note).
const CELL_OVERHEAD: usize = 4;
/// The nodes of instances the gallery may make, the most cells of an axis and of the frame.
pub const GALLERY_NODE_BUDGET: usize = 200;
pub const GALLERY_AXIS_CAP: usize = 24;
pub const GALLERY_CELL_CAP: usize = 80;
/// The most names "changes: …" lists.
const CHANGE_NAMES: usize = 4;

/// The node budget of the gallery for a component whose instance has `cell` nodes (the instance and what its cell adds): 200, or three
/// cells when even one does not fit in 200.
pub fn gallery_budget(cell: usize) -> usize {
    if cell > GALLERY_NODE_BUDGET { 3 * cell } else { GALLERY_NODE_BUDGET }
}

/// Which options of each axis the gallery draws. `options[a]` is how many options axis `a` has and `defaults[a]` the index of its
/// default; every cell costs `cell` nodes. The default of each axis always goes in; then the axes take an option each in turn
/// (rotation) while the next one fits in `budget` nodes, the axis's cap and the frame's. Returns, for each axis, the indices kept (in
/// order); the options not kept are the ones left out.
pub fn plan_cells(options: &[usize], defaults: &[Option<usize>], cell: usize, budget: usize, axis_cap: usize, cap: usize) -> Vec<Vec<usize>> {
    let mut kept: Vec<Vec<usize>> = vec![Vec::new(); options.len()];
    let mut used = 0usize;
    let mut total = 0usize;
    for (a, d) in defaults.iter().enumerate() {
        if let Some(d) = d.filter(|d| *d < options[a]) {
            kept[a].push(d);
            used += cell;
            total += 1;
        }
    }
    let mut next = vec![0usize; options.len()];
    let mut open: Vec<bool> = vec![true; options.len()];
    loop {
        let mut progressed = false;
        for a in 0..options.len() {
            if !open[a] {
                continue;
            }
            while next[a] < options[a] && kept[a].contains(&next[a]) {
                next[a] += 1;
            }
            if next[a] >= options[a] || kept[a].len() >= axis_cap || total >= cap || used + cell > budget {
                open[a] = false;
                continue;
            }
            kept[a].push(next[a]);
            next[a] += 1;
            used += cell;
            total += 1;
            progressed = true;
        }
        if !progressed {
            break;
        }
    }
    for k in &mut kept {
        k.sort_unstable();
    }
    kept
}

/// The variant a cell shows for option `option` of axis `prop`: the one with that option and the other axes at the default, else (a
/// set with holes) the one that has the option and the most other axes at the default; the flag says it is the nearer one, not the exact.
pub fn cell_variant(info: &crate::variants::SetInfo, default: &crate::variants::Pairs, prop: &str, option: &str) -> Option<(crate::variants::Variant, bool)> {
    let mut exact = default.clone();
    if let Some(slot) = exact.iter_mut().find(|(p, _)| p == prop) {
        slot.1 = option.to_string();
    }
    if let Some(v) = info.find(&exact) {
        return Some((v.clone(), false));
    }
    let score = |v: &crate::variants::Variant| {
        let pairs = crate::variants::parse_name(&v.name);
        if !pairs.iter().any(|(k, o)| k == prop && o == option) {
            return None;
        }
        Some(pairs.iter().filter(|(k, o)| k != prop && default.iter().any(|(dk, dv)| dk == k && dv == o)).count())
    };
    info.variants.iter().filter_map(|v| score(v).map(|n| (n, v))).max_by(|a, b| a.0.cmp(&b.0).then(std::cmp::Ordering::Greater)).map(|(_, v)| (v.clone(), true))
}

/// What the reader calls a change, for "changes: …", in the order they are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    Fill,
    Border,
    Opacity,
    Radius,
    Spacing,
    Typography,
    Visibility,
    Effect,
    Size,
    Text,
    Swap,
}

impl Group {
    fn key(self) -> Key {
        match self {
            Group::Fill => Key::PropFill,
            Group::Border => Key::GroupBorder,
            Group::Opacity => Key::DeltaOpacity,
            Group::Radius => Key::PropRadius,
            Group::Spacing => Key::GroupSpacing,
            Group::Typography => Key::PropTypography,
            Group::Visibility => Key::PropVisibility,
            Group::Effect => Key::GroupEffect,
            Group::Size => Key::GroupSize,
            Group::Text => Key::DeltaContent,
            Group::Swap => Key::TypeInstance,
        }
    }

    fn of(kind: &Kind) -> Group {
        match kind {
            Kind::Shown(_) => Group::Visibility,
            Kind::Swapped(_) => Group::Swap,
            Kind::Changed { label, .. } => match label {
                Label::Prop(Prop::Fill) => Group::Fill,
                Label::Prop(Prop::Stroke) | Label::StrokeWeight => Group::Border,
                Label::Opacity => Group::Opacity,
                Label::Prop(Prop::Radius) => Group::Radius,
                Label::Prop(Prop::Gap | Prop::PadH | Prop::PadV) => Group::Spacing,
                Label::Prop(Prop::Typography) => Group::Typography,
                Label::Effects => Group::Effect,
                Label::Width | Label::Height | Label::HSizing | Label::VSizing => Group::Size,
                Label::Content => Group::Text,
            },
        }
    }
}

/// The line under a tile: "changes: Fill, Border" (names only, at most four and "…" for more), "default" for the default option, "label
/// only" when the option differs in nothing the plugin reads.
pub fn changes_line(lang: Lang, is_default: bool, diff: &Diff) -> String {
    if is_default {
        return t(lang, Key::GalleryDefault).to_string();
    }
    let mut groups: Vec<Group> = diff.changes.iter().map(|c| Group::of(&c.kind)).collect();
    groups.sort_unstable();
    groups.dedup();
    if groups.is_empty() {
        return t(lang, Key::GalleryLabelOnly).to_string();
    }
    let mut names: Vec<String> = groups.iter().take(CHANGE_NAMES).map(|g| t(lang, g.key()).to_string()).collect();
    if groups.len() > CHANGE_NAMES {
        names.push("…".to_string());
    }
    tf(lang, Key::GalleryChanges, &[("list", &names.join(", "))])
}

/// The States and Variants frame: a gallery, one block for each variant axis with a tile for each option (an instance of the component on
/// the stage's colour), its name under it and what it changes from the default.
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
    let base_nodes: Vec<Value> = base_walked.iter().map(|(n, _)| n.clone()).collect();
    let vars = c.vars_for(&base_nodes)?;
    let styles = c.styles_for(&base_nodes)?;
    let base_snap = delta::snapshot(lang, &base_walked, &vars, &styles);
    // 1. Which variant each option shows (the default option shows the default variant), and which options go in.
    struct Pick {
        option: String,
        is_default: bool,
        variant: Option<(variants::Variant, bool)>,
    }
    let picks: Vec<Vec<Pick>> = axes
        .iter()
        .map(|axis| {
            axis.options
                .iter()
                .map(|o| Pick { option: o.option.clone(), is_default: o.is_default, variant: if o.is_default { o.variant.clone().map(|v| (v, false)) } else { cell_variant(&sp.info, &sp.default, &axis.prop, &o.option) } })
                .collect()
        })
        .collect();
    let cell_nodes = base_walked.len() + CELL_OVERHEAD;
    let budget = gallery_budget(cell_nodes);
    // An option with no variant at all is not defined (said under its axis), never a cell.
    let options: Vec<usize> = picks.iter().map(Vec::len).collect();
    let defaults: Vec<Option<usize>> = picks.iter().map(|p| p.iter().position(|x| x.is_default && x.variant.is_some())).collect();
    let mut kept = plan_cells(&options, &defaults, cell_nodes, budget, GALLERY_AXIS_CAP, GALLERY_CELL_CAP);
    for (a, k) in kept.iter_mut().enumerate() {
        k.retain(|i| picks[a][*i].variant.is_some());
    }
    // 2. Each axis is drawn: its name, then its cells in a grid that wraps at the useful width.
    let mut nodes_used = 0usize;
    let mut cells_made = 0usize;
    for ((axis, pk), keep) in axes.iter().zip(&picks).zip(&kept) {
        let ax_name = tf(lang, Key::LayerAxis, &[("name", &axis.prop)]);
        let mut ax = FlowSpec::new(&ax_name, USEFUL_W, "vertical");
        ax.gap = 24.0;
        let ax = flow_frame(lang, frame, ax)?;
        text_with(lang, ax, &axis.prop, theme::SECTION_SIZE, pal::TEXT, "Axis", bold())?;
        // An axis with only its default option (a component with a variant property and nothing to vary) says what the frame always said.
        if pk.len() <= 1 {
            muted_note(lang, ax, t(lang, Key::DeltaNone))?;
            continue;
        }
        let grid_name = tf(lang, Key::LayerGallery, &[("name", &axis.prop)]);
        let mut grid = FlowSpec::new(&grid_name, USEFUL_W, "horizontal");
        grid.gap = CELL_GAP;
        let grid = flow_frame(lang, ax, grid)?;
        host(lang, "node.set", &json!({"ids": [grid], "props": {"wrap": true, "crossGap": CELL_GAP, "sizingH": "fixed", "sizingV": "hug", "width": USEFUL_W}}))?;
        for i in keep {
            let Some((variant, nearest)) = &pk[*i].variant else { continue };
            let made = draw_cell(c, grid, &pk[*i].option, pk[*i].is_default, variant, *nearest, &base_snap, &vars, &styles)?;
            nodes_used += made;
            cells_made += 1;
        }
        let left_out = pk.iter().enumerate().filter(|(i, p)| p.variant.is_some() && !keep.contains(i)).count();
        if left_out > 0 {
            muted_note(lang, ax, &tf(lang, plural(left_out, Key::GalleryLeftOutOne, Key::GalleryLeftOut), &[("n", &left_out.to_string())]))?;
        }
        let missing: Vec<&str> = pk.iter().filter(|p| p.variant.is_none()).map(|p| p.option.as_str()).collect();
        if !missing.is_empty() {
            muted_note(lang, ax, &tf(lang, Key::NotDefined, &[("list", &missing.join(", "))]))?;
        }
    }
    *c.gallery.borrow_mut() = Some(json!({"cells": cells_made, "nodes": nodes_used, "budget": budget, "cell_estimate": cell_nodes, "raised": budget != GALLERY_NODE_BUDGET, "kept": kept.iter().map(Vec::len).collect::<Vec<_>>()}));
    Ok(())
}

/// One cell of the gallery in `grid`: the tile (the stage's colour, 12 radius, 24 of room) with an instance of `variant` in it, the option's
/// name under it (bold), and "changes: …" and, when it is the nearest combination, a note. Returns how many nodes it made.
#[allow(clippy::too_many_arguments)]
fn draw_cell(c: &Ctx, grid: u64, option: &str, is_default: bool, variant: &variants::Variant, nearest: bool, base: &[delta::Snap], vars: &Vars, styles: &Styles) -> Result<usize> {
    let lang = c.lang;
    let mut spec = FlowSpec::new(option, TILE_MIN_W, "vertical");
    spec.gap = 12.0;
    spec.extra = json!({"sizingH": "hug"});
    let cell = flow_frame(lang, grid, spec)?;
    let tile_name = tf(lang, Key::LayerTile, &[("name", option)]);
    let mut tile = FlowSpec::new(&tile_name, TILE_MIN_W, "vertical");
    tile.padding = json!(TILE_PAD);
    tile.fill = Some(c.stage.fill());
    tile.extra = json!({"cornerRadius": TILE_RADIUS, "crossAlign": "center", "sizingH": "fixed", "sizingV": "hug"});
    let tile = flow_frame(lang, cell, tile)?;
    let made = host(lang, "instance.create", &json!({"component": variant.id, "parent": tile}))?;
    let instance = id_of(lang, &made, "instance.create")?;
    let walked = walk_layers(lang, instance)?;
    let (iw, _) = bounds(walked.first().map(|(n, _)| n).unwrap_or(&Value::Null)).map_or((variant.w, variant.h), |b| (b.w, b.h));
    // The tile is as wide as its instance and the room (the least 160); one wider than the frame is not scaled: it says so.
    let wide = iw + 2.0 * TILE_PAD > USEFUL_W;
    let tile_w = (iw + 2.0 * TILE_PAD).clamp(TILE_MIN_W, USEFUL_W);
    if (tile_w - TILE_MIN_W).abs() > 0.01 {
        host(lang, "node.set", &json!({"ids": [tile], "props": {"width": tile_w}}))?;
    }
    let diff = if is_default { Diff::default() } else { delta::compare(base, &delta::snapshot(lang, &walked, vars, styles)) };
    text_with(lang, cell, option, FONT, pal::TEXT, "Option", json!({"fontStyle": "Bold", "width": tile_w, "textAutoResize": "height"}))?;
    text_with(lang, cell, &changes_line(lang, is_default, &diff), FONT, pal::TEXT2, "Changes", json!({"width": tile_w, "textAutoResize": "height"}))?;
    let mut notes = 0;
    for (on, key) in [(nearest, Key::GalleryNearest), (wide, Key::GalleryWide)] {
        if on {
            text_with(lang, cell, t(lang, key), FONT, pal::GUIDE, "Note", json!({"width": tile_w, "textAutoResize": "height"}))?;
            notes += 1;
        }
    }
    Ok(walked.len() + CELL_OVERHEAD - 1 + notes)
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
pub(super) fn build_anatomy(c: &Ctx, frame: u64) -> Result<(bool, usize)> {
    let lang = c.lang;
    let target = c.target;
    header(c, frame, t(lang, Key::SecAnatomy))?;
    let size = bounds(&target.node).unwrap_or(Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 });
    // The stage is as wide as the frame's room (the frames have a fixed width): a marker that has no room at the sides is left out, the stage grows in height.
    let stage_w = c.useful.get();
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
    let depths: Vec<usize> = walked.iter().map(|(_, d)| *d).collect();
    let numbered: Vec<(usize, usize)> = blocks_list.iter().filter_map(|b| b.badge.map(|n| b.nodes.iter().filter(|i| drawn.get(**i).copied().unwrap_or(false)).map(move |i| (n, *i)))).flatten().collect();
    // The stage grows when the effects need room, or the markers (which stand at different distances) need more above and below than the
    // stage has, and the instance stays in the middle.
    let room = markers_room(sb0, ib0, &Tree::new(&nodes, depths.clone()), &numbered);
    let h1 = h0.max(stage_height_for_effects(size.h, reach)).max(size.h + 2.0 * room);
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
    let tree = Tree::new(&nodes, depths);
    let badges = draw_badges(lang, stage, sb, ib, &tree, &numbered, &red)?;
    *c.markers.borrow_mut() = Some(json!({"markers": badges.info, "rounds": badges.rounds, "dropped": badges.dropped, "stage": [sb.x, sb.y, sb.w, sb.h], "component": [ib.x, ib.y, ib.w, ib.h], "texts": tree.text_rects()}));
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
        // A number whose marker did not fit in the drawing says so in the Note column.
        let sub = if badges.dropped.contains(&n) { Some(t(lang, Key::AnatNoMarker)) } else { sub };
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
    Ok((choice.too_low(), badges.dropped.len()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::delta::{Change, Side};

    fn changed(layer: Option<&str>, label: Label, from: &str, to: &str) -> Change {
        let side = |v: &str| Side { token: None, value: v.to_string() };
        Change { layer: layer.map(ToString::to_string), kind: Kind::Changed { label, from: side(from), to: side(to) }, minor: false }
    }

    #[test]
    fn the_columns_add_up_to_the_useful_width() {
        for cols in [PROPS_COLS, ANATOMY_COLS] {
            assert_eq!(cols.iter().sum::<f64>(), USEFUL_W);
        }
        assert_eq!(PROPS_COLS, [240.0, 140.0, 460.0, 244.0]);
        assert_eq!(ANATOMY_COLS, [60.0, 120.0, 280.0, 624.0]);
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






    #[test]
    fn the_default_of_each_axis_always_goes_in_and_the_axes_share_what_is_left_in_turn() {
        // Two axes of 6 options, cells of 10 nodes, 100 nodes: both defaults, then 4 more cells each... 100 / 10 = 10 cells.
        let kept = plan_cells(&[6, 6], &[Some(0), Some(2)], 10, 100, 24, 80);
        assert_eq!(kept.iter().map(Vec::len).sum::<usize>(), 10);
        assert!(kept[0].contains(&0) && kept[1].contains(&2));
        assert_eq!((kept[0].len(), kept[1].len()), (5, 5), "in rotation, neither axis takes it all: {kept:?}");
        // A budget that holds only the defaults keeps only them.
        let kept = plan_cells(&[6, 6], &[Some(0), Some(0)], 10, 20, 24, 80);
        assert_eq!(kept, vec![vec![0], vec![0]]);
        // The caps: 24 an axis, 80 in the frame.
        let kept = plan_cells(&[40], &[Some(0)], 1, 10_000, 24, 80);
        assert_eq!(kept[0].len(), 24);
        let kept = plan_cells(&[24, 24, 24, 24], &[Some(0); 4], 1, 10_000, 24, 80);
        assert_eq!(kept.iter().map(Vec::len).sum::<usize>(), 80);
        // A default that is not among the options (or an axis with none) is not forced in.
        assert_eq!(plan_cells(&[0, 3], &[None, Some(1)], 5, 100, 24, 80), vec![vec![], vec![0, 1, 2]]);
        // The indices come back in order.
        let kept = plan_cells(&[5], &[Some(3)], 1, 3, 24, 80);
        assert_eq!(kept, vec![vec![0, 1, 3]]);
    }

    #[test]
    fn the_budget_is_200_nodes_or_three_cells_when_one_does_not_fit() {
        assert_eq!(gallery_budget(12), 200);
        assert_eq!(gallery_budget(200), 200);
        assert_eq!(gallery_budget(250), 750);
    }

    fn set_of(names: &[&str]) -> crate::variants::SetInfo {
        let variants = names.iter().enumerate().map(|(i, n)| crate::variants::Variant { id: i as u64 + 1, name: (*n).to_string(), w: 10.0, h: 10.0 }).collect();
        crate::variants::SetInfo::new(100, "Set".into(), variants)
    }

    #[test]
    fn a_cell_shows_the_variant_with_the_option_and_the_other_axes_at_their_default() {
        let info = set_of(&["Type=A, Size=S", "Type=B, Size=S", "Type=A, Size=L", "Type=B, Size=L"]);
        let default = crate::variants::parse_name("Type=A, Size=S");
        let (v, nearest) = cell_variant(&info, &default, "Size", "L").unwrap();
        assert_eq!((v.name.as_str(), nearest), ("Type=A, Size=L", false));
        let (v, nearest) = cell_variant(&info, &default, "Type", "B").unwrap();
        assert_eq!((v.name.as_str(), nearest), ("Type=B, Size=S", false));
        // A set with holes: no "Type=C, Size=S": the nearest that has Type=C and the most axes at the default.
        let info = set_of(&["Type=A, Size=S, State=On", "Type=C, Size=L, State=Off", "Type=C, Size=S, State=Off"]);
        let default = crate::variants::parse_name("Type=A, Size=S, State=On");
        let (v, nearest) = cell_variant(&info, &default, "Type", "C").unwrap();
        assert_eq!((v.name.as_str(), nearest), ("Type=C, Size=S, State=Off", true), "two of the other axes at the default? Size only: the one with Size=S");
        // An option no variant has: none.
        assert!(cell_variant(&info, &default, "Type", "Z").is_none());
    }

    #[test]
    fn the_line_under_a_tile_lists_names_only_in_a_fixed_order_and_is_honest_about_nothing() {
        let diff = Diff { changes: vec![changed(None, Label::Prop(Prop::Radius), "4", "8"), changed(None, Label::Prop(Prop::Fill), "#111", "#222"), changed(Some("Icon"), Label::Prop(Prop::Fill), "#111", "#222"), changed(None, Label::Prop(Prop::Gap), "4", "8")], unreadable_effect: false };
        assert_eq!(changes_line(Lang::En, false, &diff), "changes: Fill, Radius, Spacing");
        assert_eq!(changes_line(Lang::Pt, false, &diff), "muda: Preenchimento, Raio, Espaçamento");
        assert_eq!(changes_line(Lang::En, true, &diff), "default");
        assert_eq!(changes_line(Lang::Pt, true, &Diff::default()), "padrão");
        assert_eq!(changes_line(Lang::En, false, &Diff::default()), "label only");
        assert_eq!(changes_line(Lang::Pt, false, &Diff::default()), "só o rótulo");
        // At most four names, then an ellipsis.
        let many = Diff {
            changes: vec![
                changed(None, Label::Prop(Prop::Fill), "a", "b"),
                changed(None, Label::StrokeWeight, "1", "2"),
                changed(None, Label::Opacity, "1", "0.5"),
                changed(None, Label::Prop(Prop::Radius), "1", "2"),
                changed(None, Label::Prop(Prop::PadH), "1", "2"),
                changed(None, Label::Width, "1", "2"),
            ],
            unreadable_effect: false,
        };
        assert_eq!(changes_line(Lang::En, false, &many), "changes: Fill, Border, Opacity, Radius, …");
        // No value ever shows in the line.
        assert!(!changes_line(Lang::En, false, &many).contains('#'));
    }
}
