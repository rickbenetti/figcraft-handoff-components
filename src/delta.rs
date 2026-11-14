//! What a variant changes from the default variant: the layers of the two are matched by their path
//! of names (with an index when siblings share a name) and compared property by property.
//! Pure logic (no host calls), so it can be tested on the host.
//!
//! A layer's entries are the audit's rows (radius, padding, gap, fill, stroke, typography, with their
//! token or style) plus what the audit leaves out: size, stroke weight, opacity, effects, resizing
//! and a text layer's content. Numbers that differ by less than half a pixel are the same.

use crate::audit::{self, Prop, State, Styles, Vars, number};
use crate::i18n::{Key, Lang, plural, t, tf};
use figcraft_plugin_sdk::Value;
use std::collections::HashMap;

/// Most lines a variant's list may take (headings included); the rest is counted.
pub const MAX_LINES: usize = 12;

/// What stands between a label and its value in every line of the documentation ("Radius: 8 → 12", "Fill: #FFF"): the
/// same in both languages, and the only place it is written (`gerar.rs` uses it too). Nothing reads it back.
pub const LABEL_SEP: &str = ": ";
/// Pixels (and percentage points) below which two numbers are the same.
const TOLERANCE: f64 = 0.5;
/// Longest text content quoted in a line, in characters.
const MAX_CONTENT: usize = 70;

/// What an entry is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    Prop(Prop),
    Width,
    Height,
    StrokeWeight,
    Opacity,
    Effects,
    HSizing,
    VSizing,
    Content,
}

impl Label {
    fn key(self) -> Key {
        match self {
            Label::Prop(Prop::Radius) => Key::PropRadius,
            Label::Prop(Prop::Gap) => Key::PropGap,
            Label::Prop(Prop::PadH) => Key::PropPadH,
            Label::Prop(Prop::PadV) => Key::PropPadV,
            Label::Prop(Prop::Fill) => Key::PropFill,
            Label::Prop(Prop::Stroke) => Key::PropStroke,
            Label::Prop(Prop::Typography) => Key::PropTypography,
            Label::Width => Key::DeltaWidth,
            Label::Height => Key::DeltaHeight,
            Label::StrokeWeight => Key::DeltaStrokeWeight,
            Label::Opacity => Key::DeltaOpacity,
            Label::Effects => Key::DeltaEffects,
            Label::HSizing => Key::SpHResizing,
            Label::VSizing => Key::SpVResizing,
            Label::Content => Key::DeltaContent,
        }
    }

    /// The value of a layer that has no entry for this label.
    fn blank(self) -> Entry {
        let (value, nums) = match self {
            Label::Prop(Prop::Fill | Prop::Stroke) | Label::Effects => ("none", vec![]),
            Label::Prop(Prop::Typography) | Label::Content | Label::HSizing | Label::VSizing => ("—", vec![]),
            _ => ("0", vec![0.0]),
        };
        Entry::new(self, None, value.to_string(), nums)
    }
}

/// One property of one layer.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub label: Label,
    /// The variable or style the value is bound to.
    pub token: Option<String>,
    pub value: String,
    /// The value as numbers, when it is a measure (compared with the tolerance).
    pub nums: Vec<f64>,
    /// A change in it is listed after the others when the list is cut short (a size, a resizing mode).
    pub minor: bool,
    /// A value that follows from the layer's parent or content (a hugging or filling size): never compared.
    pub derived: bool,
}

impl Entry {
    fn new(label: Label, token: Option<String>, value: String, nums: Vec<f64>) -> Entry {
        Entry { label, token, value, nums, minor: false, derived: false }
    }
}

impl Entry {
    fn same(&self, other: &Entry) -> bool {
        if self.derived || other.derived {
            return true;
        }
        if self.token != other.token {
            return false;
        }
        if !self.nums.is_empty() && !other.nums.is_empty() {
            // One number stands for all the sides (a padding of 16 is 16 / 16).
            let n = self.nums.len().max(other.nums.len());
            let at = |v: &[f64], i: usize| if v.len() == 1 { v[0] } else { v.get(i).copied().unwrap_or(f64::NAN) };
            return (0..n).all(|i| (at(&self.nums, i) - at(&other.nums, i)).abs() < TOLERANCE);
        }
        self.value == other.value
    }
}

/// A layer of a walk, ready to be compared.
#[derive(Clone, Debug)]
pub struct Snap {
    /// The names from the root down, `/`-separated; `[k]` after a name its siblings share. The root's is empty.
    pub path: String,
    pub parent: Option<usize>,
    /// The layer's own `visible`.
    pub visible: bool,
    pub entries: Vec<Entry>,
    /// The layer's effects as the host gave them, to tell a change the entries can't read.
    pub effects_raw: String,
    /// For an instance inside the component: the name of its main component (`mainComponentName`); `None` for any other layer,
    /// for the root (it is the variant itself, placed on the stage) or when the host gave none.
    pub main: Option<String>,
}

fn float(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|f| f.is_finite())
}

fn describe_effect(lang: Lang, e: &Value) -> String {
    let kind = e.get("type").and_then(Value::as_str).unwrap_or_else(|| t(lang, Key::EffectDefault));
    let n = |k: &str| float(e, k).map(number);
    let name = match kind {
        "dropShadow" => t(lang, Key::EffectDropShadow),
        "innerShadow" => t(lang, Key::EffectInnerShadow),
        "layerBlur" => t(lang, Key::EffectLayerBlur),
        "backgroundBlur" => t(lang, Key::EffectBackgroundBlur),
        other => other,
    };
    let mut out = name.to_string();
    if let Some([x, y]) = e.get("offset").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect::<Vec<_>>()).as_deref() {
        out.push_str(&format!(" {} {}", number(*x), number(*y)));
    }
    if let Some(r) = n("radius") {
        out.push(' ');
        // A layer or background blur is named "…blur" already: only its radius follows. A shadow, or
        // a type the dictionary doesn't know, has the blur as a parameter of its own.
        if matches!(kind, "layerBlur" | "backgroundBlur") {
            out.push_str(&r);
        } else {
            out.push_str(&tf(lang, Key::EffectBlur, &[("r", &r)]));
        }
    }
    if let Some(s) = float(e, "spread").filter(|s| s.abs() > 0.005) {
        out.push(' ');
        out.push_str(&tf(lang, Key::EffectSpread, &[("n", &number(s))]));
    }
    if let Some(c) = e.get("color").filter(|c| c.is_object()) {
        let ch = |k: &str| float(c, k).unwrap_or(0.0);
        // A colour's alpha comes from hex, in steps of 1/255 (#0000000D is 5.098%): whole percents, because one
        // decimal would show noise the design tool does not. Layer and paint opacity are typed as percentages: `percent()`.
        let alpha = float(c, "a").unwrap_or(1.0);
        let hex = audit::hex(ch("r"), ch("g"), ch("b"), 1.0);
        out.push(' ');
        out.push_str(&if alpha < 0.998 { tf(lang, Key::AtPercent, &[("value", &hex), ("n", &number((alpha * 100.0).round()))]) } else { hex });
    }
    out
}

/// A layer's or a paint's opacity (typed as a percentage) in percent, to one decimal; a colour's alpha is whole (see `describe_effect`).
fn percent(opacity: f64) -> f64 {
    (opacity * 100.0 * 10.0).round() / 10.0
}

/// The entries of one layer, as `node.get` gives it (with the audit's fields and `effects`, `strokeWeight`).
pub fn entries(lang: Lang, node: &Value, root: bool, vars: &Vars, styles: &Styles) -> Vec<Entry> {
    let mut out = Vec::new();
    for r in audit::rows_for(lang, node, vars, styles) {
        let token = matches!(r.state, State::Token | State::Check | State::Style).then(|| r.token.clone());
        let mut value = r.value.clone();
        if matches!(r.property, Prop::Fill | Prop::Stroke) {
            let opacity = audit::first_paint_opacity(node, if r.property == Prop::Fill { "fills" } else { "strokes" });
            if opacity < 0.995 {
                value = tf(lang, Key::AtPercent, &[("value", &value), ("n", &number(percent(opacity)))]);
            }
        }
        let nums: Vec<f64> = if r.property.numeric() { r.value.split(" / ").filter_map(|p| p.parse().ok()).collect() } else { Vec::new() };
        out.push(Entry::new(Label::Prop(r.property), token, value, nums));
    }
    let kind = node.get("type").and_then(Value::as_str).unwrap_or_default();
    // A text or a vector follows its parent's box, and what it says is in its other lines.
    if !matches!(kind, "TEXT" | "VECTOR" | "BOOLEAN_OPERATION" | "LINE" | "STAR" | "POLYGON")
        && let Some(b) = node.get("absoluteBounds")
    {
        // A layer inside counts when it sets that size itself (it is fixed). The component's own size is its number when it is
        // fixed (or the host says nothing), and its mode when it hugs or fills: a hugging size follows from the padding and the
        // content, which have lines of their own, so two variants that both hug (or both fill) have no difference in it; a
        // variant that changes the mode says so ("Width: Hug → Fixed 320"; see `change_text`).
        for (label, key, axis) in [(Label::Width, "width", "horizontal"), (Label::Height, "height", "vertical")] {
            if let Some(v) = float(b, key) {
                let mode = node.get("sizing").and_then(|s| s.get(axis)).and_then(Value::as_str);
                let fixed = mode == Some("fixed");
                match mode {
                    Some(m @ ("hug" | "fill")) if root => out.push(Entry { minor: true, ..Entry::new(label, None, m.to_string(), Vec::new()) }),
                    _ => out.push(Entry { minor: true, derived: !root && !fixed, ..Entry::new(label, None, number(v), vec![v]) }),
                }
            }
        }
    }
    let has_stroke = node.get("strokes").and_then(Value::as_array).is_some_and(|a| !a.is_empty());
    if let (true, Some(w)) = (has_stroke, float(node, "strokeWeight")) {
        out.push(Entry::new(Label::StrokeWeight, None, number(w), vec![w]));
    }
    let opacity = percent(float(node, "opacity").unwrap_or(1.0));
    out.push(Entry::new(Label::Opacity, None, format!("{}%", number(opacity)), vec![opacity]));
    let shown: Vec<&Value> = node.get("effects").and_then(Value::as_array).map(|a| a.iter().filter(|e| e.get("visible").and_then(Value::as_bool).unwrap_or(true)).collect()).unwrap_or_default();
    let effect_style = node.get("styles").and_then(|s| s.get("effect")).and_then(Value::as_str);
    let token = effect_style.and_then(|id| styles.by_id.get(id)).map(|d| d.name.clone());
    if !shown.is_empty() || token.is_some() {
        let value = if shown.is_empty() { "none".to_string() } else { shown.iter().map(|e| describe_effect(lang, e)).collect::<Vec<_>>().join("; ") };
        out.push(Entry::new(Label::Effects, token, value, vec![]));
    }
    // The component's own mode is in its width and height (above); a layer inside has a line for its mode.
    for (label, key) in [(Label::HSizing, "horizontal"), (Label::VSizing, "vertical")].into_iter().filter(|_| !root) {
        if let Some(s) = node.get("sizing").and_then(|s| s.get(key)).and_then(Value::as_str) {
            out.push(Entry { minor: true, ..Entry::new(label, None, s.to_string(), vec![]) });
        }
    }
    if kind == "TEXT"
        && let Some(c) = node.get("characters").and_then(Value::as_str)
    {
        out.push(Entry::new(Label::Content, None, c.to_string(), vec![]));
    }
    out
}

/// The layers of a walk (depth first, each with its depth), ready to compare.
pub fn snapshot(lang: Lang, walked: &[(Value, usize)], vars: &Vars, styles: &Styles) -> Vec<Snap> {
    let name_of = |i: usize| walked.get(i).and_then(|(n, _)| n.get("name")).and_then(Value::as_str).unwrap_or_default();
    let mut parents: Vec<Option<usize>> = Vec::with_capacity(walked.len());
    let mut stack: Vec<usize> = Vec::new();
    for (i, (_, d)) in walked.iter().enumerate() {
        stack.truncate(*d);
        parents.push(stack.last().copied());
        stack.push(i);
    }
    let mut totals: HashMap<(Option<usize>, &str), usize> = HashMap::new();
    for (i, p) in parents.iter().enumerate() {
        *totals.entry((*p, name_of(i))).or_default() += 1;
    }
    let mut seen: HashMap<(Option<usize>, &str), usize> = HashMap::new();
    let mut paths: Vec<String> = Vec::with_capacity(walked.len());
    for (i, p) in parents.iter().enumerate() {
        let Some(parent) = p else {
            paths.push(String::new());
            continue;
        };
        let name = name_of(i);
        let k = seen.entry((*p, name)).or_default();
        let segment = if totals.get(&(*p, name)).copied().unwrap_or(0) > 1 { format!("{name}[{k}]") } else { name.to_string() };
        *k += 1;
        paths.push(format!("{}/{segment}", paths.get(*parent).map_or("", String::as_str)));
    }
    walked
        .iter()
        .enumerate()
        .map(|(i, (n, _))| Snap {
            path: paths.get(i).cloned().unwrap_or_default(),
            parent: parents.get(i).copied().flatten(),
            visible: n.get("visible").and_then(Value::as_bool).unwrap_or(true),
            entries: entries(lang, n, i == 0, vars, styles),
            effects_raw: n.get("effects").map(ToString::to_string).unwrap_or_default(),
            main: (i > 0 && n.get("type").and_then(Value::as_str) == Some("INSTANCE")).then(|| n.get("mainComponentName").and_then(Value::as_str)).flatten().filter(|m| !m.is_empty()).map(ToString::to_string),
        })
        .collect()
}

/// Whether a layer is drawn: it and everything above it is visible.
fn drawn(snaps: &[Snap]) -> Vec<bool> {
    let mut out: Vec<bool> = Vec::with_capacity(snaps.len());
    for s in snaps {
        out.push(s.visible && s.parent.is_none_or(|p| out.get(p).copied().unwrap_or(true)));
    }
    out
}

/// One side of a change.
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub token: Option<String>,
    pub value: String,
}

impl Side {
    fn of(e: &Entry) -> Side {
        Side { token: e.token.clone(), value: e.value.clone() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Changed { label: Label, from: Side, to: Side },
    /// The layer is hidden (false) or shown (true) in the variant.
    Shown(bool),
    /// The layer is an instance of another component in the variant: the text says what changed (see [`swap_text`]).
    Swapped(String),
}

/// A change on a layer; `layer` is its path (`None`: the root).
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub layer: Option<String>,
    pub kind: Kind,
    /// Listed after the others when the list is cut short.
    pub minor: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diff {
    pub changes: Vec<Change>,
    /// Some layer's effects differ in a way the entries can't tell.
    pub unreadable_effect: bool,
}

/// A variant name as its "Key=value" pairs; `None` when it is not that (a piece without exactly one `=`, which is
/// also how a value with a comma shows up), or a key repeats.
fn variant_pairs(name: &str) -> Option<Vec<(&str, &str)>> {
    let mut out: Vec<(&str, &str)> = Vec::new();
    for piece in name.split(',') {
        let mut it = piece.split('=');
        let (k, v) = (it.next()?.trim(), it.next()?.trim());
        if it.next().is_some() || k.is_empty() || out.iter().any(|(o, _)| *o == k) {
            return None;
        }
        out.push((k, v));
    }
    Some(out)
}

/// What an instance's swap says: when both names are variant names with the same keys, only the keys that change
/// ("Name: caret-up → caret-down", several joined by ", "); otherwise both names as they came. Names and values are the
/// file's, so they are not translated.
fn swap_text(from: &str, to: &str) -> String {
    let whole = || format!("{from} → {to}");
    let (Some(a), Some(b)) = (variant_pairs(from), variant_pairs(to)) else {
        return whole();
    };
    if a.len() != b.len() || !a.iter().all(|(k, _)| b.iter().any(|(o, _)| o == k)) {
        return whole();
    }
    let changed: Vec<String> = a.iter().filter_map(|(k, v)| b.iter().find(|(o, _)| o == k).filter(|(_, w)| w != v).map(|(_, w)| format!("{k}: {v} → {w}"))).collect();
    if changed.is_empty() { whole() } else { changed.join(", ") }
}

/// What `variant` changes from `base`: the layers matched by path, the root by position.
pub fn compare(base: &[Snap], variant: &[Snap]) -> Diff {
    let mut diff = Diff::default();
    let by_path: HashMap<&str, usize> = base.iter().enumerate().skip(1).map(|(i, s)| (s.path.as_str(), i)).collect();
    let (base_drawn, var_drawn) = (drawn(base), drawn(variant));
    let mut matched = vec![false; base.len()];
    let mut mapped = vec![false; variant.len()];
    // The instances that are another component in the variant: what is inside them is the old or the new component's, not a change.
    let mut swapped: Vec<String> = Vec::new();
    let inside = |swapped: &[String], path: &str| swapped.iter().any(|s| path.strip_prefix(s.as_str()).is_some_and(|rest| rest.starts_with('/')));
    for (j, v) in variant.iter().enumerate() {
        let i = if j == 0 { (!base.is_empty()).then_some(0) } else { by_path.get(v.path.as_str()).copied() };
        let layer = (j > 0).then(|| v.path.clone());
        if j > 0 && inside(&swapped, &v.path) {
            if let Some(i) = i {
                matched[i] = true;
            }
            mapped[j] = true;
            continue;
        }
        let Some(i) = i else {
            // A layer the default doesn't have: it appears (when it is drawn).
            // Only the layer itself, not what is inside it.
            if var_drawn.get(j).copied().unwrap_or(false) && v.parent.is_none_or(|p| mapped.get(p).copied().unwrap_or(false)) {
                diff.changes.push(Change { layer, kind: Kind::Shown(true), minor: false });
            }
            continue;
        };
        if let Some(m) = matched.get_mut(i) {
            *m = true;
        }
        if let Some(m) = mapped.get_mut(j) {
            *m = true;
        }
        let b = &base[i];
        let (bd, vd) = (base_drawn.get(i).copied().unwrap_or(true), var_drawn.get(j).copied().unwrap_or(true));
        if bd != vd {
            // Only the layer that was switched, not the ones under it.
            let parent_same = v.parent.zip(b.parent).is_none_or(|(vp, bp)| var_drawn.get(vp) == base_drawn.get(bp));
            if parent_same && v.parent.is_none_or(|p| var_drawn.get(p).copied().unwrap_or(true)) {
                diff.changes.push(Change { layer, kind: Kind::Shown(vd), minor: false });
            }
            continue;
        }
        if !vd {
            continue;
        }
        if let (Some(from), Some(to)) = (&b.main, &v.main)
            && from != to
        {
            diff.changes.push(Change { layer: layer.clone(), kind: Kind::Swapped(swap_text(from, to)), minor: false });
            swapped.push(v.path.clone());
        }
        let mut labels: Vec<Label> = b.entries.iter().map(|e| e.label).collect();
        for e in &v.entries {
            if !labels.contains(&e.label) {
                labels.push(e.label);
            }
        }
        for label in labels {
            let find = |s: &Snap| s.entries.iter().find(|e| e.label == label).cloned().unwrap_or_else(|| label.blank());
            let (from, to) = (find(b), find(v));
            if !from.same(&to) {
                diff.changes.push(Change { layer: layer.clone(), kind: Kind::Changed { label, from: Side::of(&from), to: Side::of(&to) }, minor: from.minor || to.minor });
            }
        }
        if b.effects_raw != v.effects_raw
            && !diff.changes.iter().any(|c| c.layer == layer && matches!(&c.kind, Kind::Changed { label: Label::Effects, .. }))
        {
            diff.unreadable_effect = true;
        }
    }
    // A layer of the default that the variant doesn't have is gone.
    for (i, b) in base.iter().enumerate().skip(1) {
        let gone = !matched.get(i).copied().unwrap_or(false) && !inside(&swapped, &b.path);
        if gone && base_drawn.get(i).copied().unwrap_or(false) && b.parent.is_none_or(|p| matched.get(p).copied().unwrap_or(false)) {
            diff.changes.push(Change { layer: Some(b.path.clone()), kind: Kind::Shown(false), minor: false });
        }
    }
    diff
}

/// A line of a variant's list.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    /// A layer's name over its changes.
    pub heading: bool,
    /// A change of a property ("Label: from → to"), as against a heading or a layer shown or hidden.
    pub entry: bool,
    /// The change as its parts, when it has a token or a colour on one of its sides (the stacked format draws it; `text` is
    /// the same change on one line, for when it does not).
    pub cells: Option<Cells>,
}

/// How a colour is drawn: a square of the colour, or (when the colour has an alpha) an empty square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Swatch {
    Solid,
    Hollow,
}

/// One side of a change as two lines: its token (when it has one) and its value.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub token: Option<String>,
    /// The value as it is said: the colour's hex (with its alpha, written), a number, a text, the word for "none", or a dash
    /// when the token is all there is to say.
    pub value: String,
    /// Set when the value is a colour: its square.
    pub colour: Option<Swatch>,
}

/// A change split into its label and its two sides (see [`Line::cells`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Cells {
    pub label: String,
    pub from: Cell,
    pub to: Cell,
}

/// What stands where a side has no token, or no value to say.
pub const DASH: &str = "—";

/// The colour a value says: `#RRGGBB` is solid; one with an alpha (`#RRGGBBAA` below FF) or an opacity after it ("at 50%") is
/// hollow; anything else (a word, a number, a gradient's kind) is no colour.
fn colour_of(value: &str) -> Option<Swatch> {
    let rest = value.strip_prefix('#')?;
    let digits = rest.chars().take_while(char::is_ascii_hexdigit).count();
    if digits != 6 && digits != 8 {
        return None;
    }
    let tail = &rest[digits..];
    if !tail.is_empty() && !tail.starts_with(' ') {
        return None;
    }
    let opaque = digits == 6 || rest[6..8].eq_ignore_ascii_case("ff");
    Some(if opaque && tail.is_empty() { Swatch::Solid } else { Swatch::Hollow })
}

/// A side as a cell.
fn cell_of(s: &Side, label: Label, lang: Lang) -> Cell {
    let shown = show_value(&s.value, label, lang);
    let alone = shown.trim().is_empty() || shown == DASH || s.token.as_deref() == Some(shown.as_str());
    let colour = matches!(label, Label::Prop(Prop::Fill | Prop::Stroke)).then(|| colour_of(&shown)).flatten();
    Cell { token: s.token.clone(), value: if alone { DASH.to_string() } else { shown }, colour }
}

/// The parts of a change, when it is drawn stacked: a fill or stroke with a colour on a side, or any property with a token on a
/// side. A text's change (it has quotes of its own) and the rest (a size, an opacity, a stroke weight) are one line.
fn cells_of(lang: Lang, label: Label, from: &Side, to: &Side) -> Option<Cells> {
    if label == Label::Content {
        return None;
    }
    let (a, b) = (cell_of(from, label, lang), cell_of(to, label, lang));
    (a.token.is_some() || b.token.is_some() || a.colour.is_some() || b.colour.is_some()).then(|| Cells { label: t(lang, label.key()).to_string(), from: a, to: b })
}

/// The lines a variant shows: at most [`MAX_LINES`], and how many changes did not fit.
#[derive(Clone, Debug, PartialEq)]
pub struct Listing {
    pub lines: Vec<Line>,
    pub more: usize,
    /// What to say when there is nothing to list.
    pub none: Option<String>,
}

impl Listing {
    /// Every text, in order (the extra line included), for the regression record.
    pub fn texts(&self, lang: Lang) -> Vec<String> {
        let mut out: Vec<String> = self.lines.iter().map(|l| l.text.clone()).collect();
        if self.more > 0 {
            out.push(tf(lang, plural(self.more, Key::DeltaMoreOne, Key::DeltaMore), &[("n", &self.more.to_string())]));
        }
        out.extend(self.none.clone());
        out
    }
}

/// A side's value as the line words it: the sizing words, "none" for a fill, stroke or effects that a layer lacks (it is
/// worded here); anything else as it is.
fn show_value(value: &str, label: Label, lang: Lang) -> String {
    match label {
        Label::HSizing | Label::VSizing => sizing_word(lang, value),
        Label::Width | Label::Height if matches!(value, "hug" | "fill") => sizing_word(lang, value),
        Label::Prop(Prop::Fill | Prop::Stroke) | Label::Effects if value == "none" => t(lang, Key::ValueNone).to_string(),
        _ => value.to_string(),
    }
}

/// One side of the arrow: a side with a token is "token (value)"; one with none is its value. A token whose value is
/// empty, a dash or the token's own text is shown alone, without empty or repeated parentheses.
fn show_side(s: &Side, label: Label, lang: Lang) -> String {
    let value = show_value(&s.value, label, lang);
    match &s.token {
        Some(token) if value.trim().is_empty() || value == "—" || value == *token => token.clone(),
        Some(token) => format!("{token} ({value})"),
        None => value,
    }
}

fn sizing_word(lang: Lang, value: &str) -> String {
    match value {
        "hug" => t(lang, Key::ResizeHug).to_string(),
        "fixed" => t(lang, Key::ResizeFixed).to_string(),
        "fill" => t(lang, Key::ResizeFill).to_string(),
        other => other.to_string(),
    }
}

/// The marks of a cut: the text of the file may have an ellipsis of its own, so a cut is not "…".
const CUT_START: &str = "[…] ";
const CUT_END: &str = " […]";
/// Characters of context kept before the first difference.
const CONTEXT: usize = 10;

/// The text as a line quotes it: line breaks are spaces.
fn flat(text: &str) -> Vec<char> {
    text.replace('\n', " ").chars().collect()
}

/// Two texts cut to one window of at most [`MAX_CONTENT`] characters (counted as Unicode scalar values) that holds
/// the first place where they differ, with some context before it. The window starts and ends between words (at a
/// space) when it can; a word too long for the window is cut at its edge. A cut is marked `[…]`; a text that ends
/// inside the window is whole, with no mark. Both sides use the same start.
fn windows(a: &str, b: &str) -> (String, String) {
    let (a, b) = (flat(a), flat(b));
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if a.len().max(b.len()) <= MAX_CONTENT {
        return (a.into_iter().collect(), b.into_iter().collect());
    }
    // The start: CONTEXT characters before the difference, moved back to the start of its word, unless that word is so
    // long that the difference would be far from the window's start.
    let near = common.saturating_sub(CONTEXT);
    let start = if near == 0 || a[near - 1] == ' ' || a[near] == ' ' {
        near
    } else {
        match a[..near].iter().rposition(|c| *c == ' ') {
            Some(space) if common - (space + 1) <= MAX_CONTENT / 2 => space + 1,
            // The difference is inside the text's first word: the window starts with the text.
            None if common <= MAX_CONTENT / 2 => 0,
            _ => near,
        }
    };
    let cut = |v: &[char]| {
        let limit = (start + MAX_CONTENT).min(v.len());
        let end = if limit == v.len() || v[limit] == ' ' {
            limit
        } else {
            // Back to the last space of the window, as long as the difference stays inside.
            match v[start..limit].iter().rposition(|c| *c == ' ') {
                Some(space) if start + space > common => start + space,
                _ => limit,
            }
        };
        let body: String = v[start..end].iter().collect();
        format!("{}{body}{}", if start > 0 { CUT_START } else { "" }, if end < v.len() { CUT_END } else { "" })
    };
    (cut(&a), cut(&b))
}

/// A text with its whitespace made plain: line breaks, tabs and runs of spaces become one space, the ends are trimmed.
fn plain_whitespace(text: &str) -> String {
    text.split([' ', '\n', '\r', '\t']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// "Label: from → to", each side with its value in parentheses after the name of its token (see [`show_side`]).
fn change_text(lang: Lang, label: Label, from: &Side, to: &Side) -> String {
    // A component's width or height that changes mode: the mode before and after, with the number of the fixed one.
    let mode = |s: &Side| matches!(s.value.as_str(), "hug" | "fill");
    if matches!(label, Label::Width | Label::Height) && mode(from) != mode(to) {
        let said = |s: &Side| if mode(s) { sizing_word(lang, &s.value) } else { format!("{} {}", sizing_word(lang, "fixed"), s.value) };
        return format!("{}{LABEL_SEP}{} → {}", t(lang, label.key()), said(from), said(to));
    }
    let (a, b) = if label == Label::Content {
        // The two texts differ, but only in how they are spaced: say so rather than show two texts that look the same.
        if plain_whitespace(&from.value) == plain_whitespace(&to.value) {
            return format!("{}{LABEL_SEP}{}", t(lang, label.key()), t(lang, Key::DeltaWhitespaceOnly));
        }
        let (a, b) = windows(&from.value, &to.value);
        (format!("\"{a}\""), format!("\"{b}\""))
    } else {
        (show_side(from, label, lang), show_side(to, label, lang))
    };
    format!("{}{LABEL_SEP}{a} → {b}", t(lang, label.key()))
}

/// A layer's path as a title: "Left icon / Cube".
fn title(path: &str) -> String {
    path.trim_start_matches('/').split('/').collect::<Vec<_>>().join(" / ")
}

/// Most layer names a grouped title lists; the others are counted ("+2").
const MAX_GROUP_NAMES: usize = 3;

/// What a change says, apart from the layer it is on: two layers whose changes have the same signatures, in the same order,
/// are the same change. A long text counts with its whole text, not with the part a line quotes.
fn signature(c: &Change, lang: Lang) -> String {
    let kind = match &c.kind {
        Kind::Changed { label: Label::Content, from, to } => format!("content|{}|{}", from.value, to.value),
        Kind::Changed { label, from, to } => format!("changed|{}", change_text(lang, *label, from, to)),
        Kind::Shown(on) => format!("shown|{on}"),
        Kind::Swapped(text) => format!("swapped|{text}"),
    };
    format!("{kind}|{}", c.minor)
}

/// The title of a group: the names of its layers, at most [`MAX_GROUP_NAMES`], then "+N" for the rest.
fn group_name(lang: Lang, paths: &[&String]) -> String {
    let names: Vec<String> = paths.iter().take(MAX_GROUP_NAMES).map(|p| title(p)).collect();
    let mut out = names.join(t(lang, Key::DeltaNameSeparator));
    if paths.len() > MAX_GROUP_NAMES {
        out.push_str(&format!(" +{}", paths.len() - MAX_GROUP_NAMES));
    }
    out
}

/// The list of a variant's changes, at most [`MAX_LINES`] lines. Changes come grouped by layer, the
/// root first (no heading); a layer with several changes has a "↳ name" heading, one with a single
/// switch of visibility is one line. Layers with the same changes, in the same order, are one group under a title with
/// all their names, at the place of the first of them (see [`signature`]); the lines count after that.
pub fn list(lang: Lang, diff: &Diff) -> Listing {
    if diff.changes.is_empty() {
        let none = t(lang, if diff.unreadable_effect { Key::DeltaEffectOnly } else { Key::DeltaNone }).to_string();
        return Listing { lines: Vec::new(), more: 0, none: Some(none) };
    }
    // Group by layer, in order of first appearance.
    let mut order: Vec<Option<&String>> = Vec::new();
    for c in &diff.changes {
        if !order.contains(&c.layer.as_ref()) {
            order.push(c.layer.as_ref());
        }
    }
    // The layers (not the root) with the same changes are one: the first stands for the others.
    let signatures: Vec<(Option<&String>, Vec<String>)> =
        order.iter().map(|l| (*l, diff.changes.iter().filter(|c| c.layer.as_ref() == *l).map(|c| signature(c, lang)).collect())).collect();
    let mut groups: Vec<(Option<&String>, Vec<&String>)> = Vec::new();
    for (layer, sig) in &signatures {
        let twin = layer.and_then(|_| groups.iter().position(|(rep, _)| rep.is_some() && signatures.iter().any(|(l, s)| l == rep && s == sig)));
        match (twin, layer) {
            (Some(i), Some(path)) => groups[i].1.push(path),
            (_, Some(path)) => groups.push((*layer, vec![path])),
            (_, None) => groups.push((None, Vec::new())),
        }
    }
    let order: Vec<Option<&String>> = groups.iter().map(|(rep, _)| *rep).collect();
    let changes: Vec<&Change> = diff.changes.iter().filter(|c| order.contains(&c.layer.as_ref())).collect();
    // A layer with a single switch of visibility is one line; so is a swapped instance, whose line names the layer for
    // its other changes too; any other gets a heading (the root none).
    let headed = |layer: Option<&String>| {
        let mut own = changes.iter().filter(|c| c.layer.as_ref() == layer);
        let swapped = own.clone().any(|c| matches!(c.kind, Kind::Swapped(_)));
        layer.is_some() && !swapped && !(own.next().is_some_and(|c| matches!(c.kind, Kind::Shown(_))) && own.next().is_none())
    };
    // The budget goes to the main changes first and then to the minor ones (sizes), in tree order.
    let mut picked = vec![false; changes.len()];
    let mut with_heading: Vec<Option<&String>> = Vec::new();
    let mut used = 0usize;
    for minor in [false, true] {
        for (i, c) in changes.iter().enumerate().filter(|(_, c)| c.minor == minor) {
            let layer = c.layer.as_ref();
            let heading = headed(layer) && !with_heading.contains(&layer);
            let cost = 1 + usize::from(heading);
            if used + cost > MAX_LINES {
                continue;
            }
            used += cost;
            picked[i] = true;
            if heading {
                with_heading.push(layer);
            }
        }
    }
    let mut lines: Vec<Line> = Vec::new();
    let mut shown = 0usize;
    for (layer, paths) in &groups {
        let name = if layer.is_some() { group_name(lang, paths) } else { String::new() };
        let mut first = true;
        for c in changes.iter().zip(&picked).filter(|(c, p)| **p && c.layer.as_ref() == *layer).map(|(c, _)| c) {
            if first && headed(*layer) {
                lines.push(Line { text: format!("↳ {name}"), heading: true, entry: false, cells: None });
            }
            first = false;
            let text = match &c.kind {
                Kind::Changed { label, from, to } => change_text(lang, *label, from, to),
                Kind::Shown(on) => format!("↳ {name}{LABEL_SEP}{}", t(lang, if *on { Key::DeltaVisible } else { Key::DeltaHidden })),
                Kind::Swapped(text) => format!("↳ {name}{LABEL_SEP}{text}"),
            };
            let cells = match &c.kind {
                Kind::Changed { label, from, to } => cells_of(lang, *label, from, to),
                _ => None,
            };
            let entry = matches!(c.kind, Kind::Changed { .. });
            lines.push(Line { text, heading: false, entry, cells });
            shown += 1;
        }
    }
    Listing { lines, more: changes.len() - shown, none: None }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use figcraft_plugin_sdk::json;

    fn button(radius: f64, pad: f64, fill: &str, size: f64) -> Vec<(Value, usize)> {
        vec![
            (
                json!({"name": "Button", "type": "COMPONENT", "cornerRadius": radius, "fills": [{"type": "solid", "color": fill}],
                    "layout": {"mode": "horizontal", "gap": 8.0, "padding": [10.0, pad, 10.0, pad]},
                    "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 100.0, "height": 40.0}, "sizing": {"horizontal": "fixed", "vertical": "hug"}}),
                0,
            ),
            (json!({"name": "Icon", "type": "INSTANCE", "visible": true, "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 20.0, "height": 20.0}}), 1),
            (
                json!({"name": "Label", "type": "INSTANCE", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 60.0, "height": 24.0}}),
                1,
            ),
            (
                json!({"name": "Text", "type": "TEXT", "characters": "Button", "fontFamily": "Work Sans", "fontSize": size, "fontStyle": "Medium", "lineHeight": "150%", "letterSpacing": "0%"}),
                2,
            ),
        ]
    }

    fn diff_of(a: &[(Value, usize)], b: &[(Value, usize)]) -> Diff {
        let (vars, styles) = (Vars::default(), Styles::default());
        compare(&snapshot(Lang::En, a, &vars, &styles), &snapshot(Lang::En, b, &vars, &styles))
    }

    fn texts(d: &Diff) -> Vec<String> {
        list(Lang::En, d).texts(Lang::En)
    }

    /// A component with an instance "Chevron" (of `main`) that has a vector inside (`inner`, visible or not), a sibling "Label"
    /// and, when `width` is given, a size for the instance.
    fn with_instance(main: &str, inner: &str, width: f64) -> Vec<(Value, usize)> {
        vec![
            (json!({"name": "Row", "type": "COMPONENT", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 100.0, "height": 40.0}}), 0),
            (json!({"name": "Chevron", "type": "INSTANCE", "mainComponentName": main, "sizing": {"horizontal": "fixed", "vertical": "fixed"},
                "absoluteBounds": {"x": 0.0, "y": 0.0, "width": width, "height": 16.0}}), 1),
            (json!({"name": inner, "type": "VECTOR", "visible": true, "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 8.0, "height": 8.0}}), 2),
            (json!({"name": "Label", "type": "TEXT", "characters": "Hi"}), 1),
        ]
    }

    fn swap(a: &str, b: &str) -> Vec<String> {
        texts(&diff_of(&with_instance(a, "up", 16.0), &with_instance(b, "down", 16.0)))
    }

    #[test]
    fn a_swapped_instance_says_only_the_keys_that_change() {
        // (a) one key differs.
        assert_eq!(swap("Name=caret-up, Size=16", "Name=caret-down, Size=16"), vec!["↳ Chevron: Name: caret-up → caret-down"]);
        // (b) two keys differ, in the order of the first name.
        assert_eq!(swap("Name=a, Size=16", "Name=b, Size=24"), vec!["↳ Chevron: Name: a → b, Size: 16 → 24"]);
        // The order of the keys in the second name does not matter.
        assert_eq!(swap("Name=a, Size=16", "Size=24, Name=a"), vec!["↳ Chevron: Size: 16 → 24"]);
    }

    #[test]
    fn a_swap_that_is_not_a_clean_variant_change_says_both_names() {
        // (c) other keys, a key more, a name that is not a variant name.
        assert_eq!(swap("Name=a, Size=16", "Kind=a, Size=16"), vec!["↳ Chevron: Name=a, Size=16 → Kind=a, Size=16"]);
        assert_eq!(swap("Name=a", "Name=a, Size=16"), vec!["↳ Chevron: Name=a → Name=a, Size=16"]);
        assert_eq!(swap("Icon / Plus", "Icon / Minus"), vec!["↳ Chevron: Icon / Plus → Icon / Minus"]);
        assert_eq!(swap("Name=a", "Icon"), vec!["↳ Chevron: Name=a → Icon"]);
        // (d) a value with a comma is ambiguous: whole names. So is a repeated key, and names that differ only in order.
        assert_eq!(swap("Name=a,b, Size=16", "Name=c, Size=16"), vec!["↳ Chevron: Name=a,b, Size=16 → Name=c, Size=16"]);
        assert_eq!(swap("Name=a, Name=b", "Name=a, Name=c"), vec!["↳ Chevron: Name=a, Name=b → Name=a, Name=c"]);
        assert_eq!(swap("Name=a, Size=1", "Size=1, Name=a"), vec!["↳ Chevron: Name=a, Size=1 → Size=1, Name=a"]);
    }

    #[test]
    fn what_is_inside_a_swapped_instance_is_not_listed_but_its_own_changes_and_its_siblings_are() {
        // (e) the vector inside changes name ("up" gone, "down" new): no lines for it. (f) the instance keeps its own size change.
        let d = diff_of(&with_instance("Name=up", "up", 16.0), &with_instance("Name=down", "down", 24.0));
        assert_eq!(texts(&d), vec!["↳ Chevron: Name: up → down", "Width: 16 → 24"]);
        // A change outside the instance is still listed, in the tree's order.
        let mut b = with_instance("Name=down", "down", 16.0);
        b[3].0 = json!({"name": "Label", "type": "TEXT", "characters": "Bye"});
        assert_eq!(texts(&diff_of(&with_instance("Name=up", "up", 16.0), &b)), vec!["↳ Chevron: Name: up → down", "↳ Label", "Text: \"Hi\" → \"Bye\""]);
    }

    #[test]
    fn an_instance_of_the_same_component_keeps_the_old_behaviour() {
        // (g) same main component: the inner layers are listed as before (one hidden, one shown), no swap line.
        let a = with_instance("Name=up", "up", 16.0);
        let mut b = with_instance("Name=up", "down", 16.0);
        b[2].0 = json!({"name": "down", "type": "VECTOR", "visible": true, "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 8.0, "height": 8.0}});
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Chevron / down: visible", "↳ Chevron / up: hidden"]);
        // No main component name on either side (the host gave none): nothing to compare.
        let mut no_main_a = with_instance("", "up", 16.0);
        no_main_a[1].0 = json!({"name": "Chevron", "type": "INSTANCE", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 16.0, "height": 16.0}});
        assert!(!texts(&diff_of(&no_main_a, &with_instance("Name=down", "down", 16.0))).iter().any(|l| l.contains("→ Name")));
    }

    #[test]
    fn the_root_is_never_a_swap_even_when_it_is_an_instance_of_each_variant() {
        // The walk's root is the variant placed on the stage: an instance of a different main component for each variant.
        let root = |main: &str, w: f64| (json!({"name": "Row", "type": "INSTANCE", "mainComponentName": main, "absoluteBounds": {"x": 0.0, "y": 0.0, "width": w, "height": 40.0}, "sizing": {"horizontal": "fixed", "vertical": "fixed"}}), 0);
        let d = diff_of(&[root("Size=Small", 100.0)], &[root("Size=Large", 120.0)]);
        assert_eq!(texts(&d), vec!["Width: 100 → 120"]);
    }

    #[test]
    fn only_instances_are_swapped_and_a_layer_that_appears_or_goes_is_still_shown_or_hidden() {
        // (h) a frame whose children change is not a swap, whatever it is called.
        let frame = |inner: &str| {
            vec![
                (json!({"name": "Row", "type": "COMPONENT", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 100.0, "height": 40.0}}), 0),
                (json!({"name": "Chevron", "type": "FRAME", "mainComponentName": "Name=x", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 16.0, "height": 16.0}}), 1),
                (json!({"name": inner, "type": "VECTOR", "visible": true}), 2),
            ]
        };
        assert_eq!(texts(&diff_of(&frame("up"), &frame("down"))), vec!["↳ Chevron / down: visible", "↳ Chevron / up: hidden"]);
        // (i) an instance that is hidden in one variant and shown in the other, or that only one has: visible / hidden.
        let mut hidden = with_instance("Name=up", "up", 16.0);
        hidden[1].0 = json!({"name": "Chevron", "type": "INSTANCE", "mainComponentName": "Name=up", "visible": false});
        assert_eq!(texts(&diff_of(&with_instance("Name=up", "up", 16.0), &hidden)), vec!["↳ Chevron: hidden"]);
        let without = vec![with_instance("Name=up", "up", 16.0).remove(0)];
        // (the sibling "Label" of the helper comes and goes with it)
        // (the two layers say the same, so they are one group)
        assert_eq!(texts(&diff_of(&without, &with_instance("Name=up", "up", 16.0))), vec!["↳ Chevron · Label: visible"]);
        assert_eq!(texts(&diff_of(&with_instance("Name=up", "up", 16.0), &without)), vec!["↳ Chevron · Label: hidden"]);
    }

    /// A component with `rows` text layers; `to(i)` is the text of row `i` in the variant (every row says "a" in the default).
    fn rows_of(rows: &[&str], texts: &[&str]) -> Vec<(Value, usize)> {
        let mut v = vec![(json!({"name": "Row", "type": "COMPONENT", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 100.0, "height": 40.0}}), 0)];
        for (name, text) in rows.iter().zip(texts) {
            v.push((json!({"name": name, "type": "TEXT", "characters": text, "fontFamily": "W", "fontSize": 14.0, "fontStyle": "R", "lineHeight": "150%", "letterSpacing": "0%"}), 1));
        }
        v
    }

    fn grouped(lang: Lang, names: &[&str], from: &[&str], to: &[&str]) -> Vec<String> {
        list(lang, &diff_of(&rows_of(names, from), &rows_of(names, to))).texts(lang)
    }

    #[test]
    fn layers_with_the_same_changes_are_one_group_under_all_their_names() {
        // Two, three.
        assert_eq!(grouped(Lang::En, &["A", "B"], &["a", "a"], &["b", "b"]), vec!["↳ A · B", "Text: \"a\" → \"b\""]);
        assert_eq!(grouped(Lang::En, &["A", "B", "C"], &["a", "a", "a"], &["b", "b", "b"]), vec!["↳ A · B · C", "Text: \"a\" → \"b\""]);
        // Four, five: three names, then "+N".
        assert_eq!(grouped(Lang::En, &["A", "B", "C", "D"], &["a"; 4], &["b"; 4]), vec!["↳ A · B · C +1", "Text: \"a\" → \"b\""]);
        assert_eq!(grouped(Lang::En, &["A", "B", "C", "D", "E"], &["a"; 5], &["b"; 5]), vec!["↳ A · B · C +2", "Text: \"a\" → \"b\""]);
    }

    #[test]
    fn the_separator_is_the_languages_and_a_layer_alone_is_as_it_was() {
        assert_eq!(grouped(Lang::Pt, &["A", "B", "C", "D", "E"], &["a"; 5], &["b"; 5]), vec!["↳ A, B, C +2", "Texto: \"a\" → \"b\""]);
        assert_eq!(grouped(Lang::Pt, &["A", "B"], &["a", "a"], &["b", "b"]), vec!["↳ A, B", "Texto: \"a\" → \"b\""]);
        // One layer: the heading, the line, nothing else.
        assert_eq!(grouped(Lang::En, &["A"], &["a"], &["b"]), vec!["↳ A", "Text: \"a\" → \"b\""]);
        assert_eq!((t(Lang::En, Key::DeltaNameSeparator), t(Lang::Pt, Key::DeltaNameSeparator)), (" · ", ", "));
    }

    #[test]
    fn the_group_takes_the_place_of_its_first_layer_and_the_others_keep_their_order() {
        // A and C say the same; B says something else in between: A·C comes first, then B.
        assert_eq!(grouped(Lang::En, &["A", "B", "C"], &["a", "a", "a"], &["b", "x", "b"]), vec!["↳ A · C", "Text: \"a\" → \"b\"", "↳ B", "Text: \"a\" → \"x\""]);
        // A different change first: B and D group at B's place, after A and C stay as they were.
        assert_eq!(
            grouped(Lang::En, &["A", "B", "C", "D"], &["a"; 4], &["p", "q", "r", "q"]),
            vec!["↳ A", "Text: \"a\" → \"p\"", "↳ B · D", "Text: \"a\" → \"q\"", "↳ C", "Text: \"a\" → \"r\""]
        );
    }

    #[test]
    fn different_lists_do_not_group_not_even_with_the_same_label_or_token() {
        // The same label with another value.
        assert_eq!(grouped(Lang::En, &["A", "B"], &["a", "a"], &["b", "c"]), vec!["↳ A", "Text: \"a\" → \"b\"", "↳ B", "Text: \"a\" → \"c\""]);
        // The same token, another value: two lines that read differently.
        let vars = Vars::parse(&json!({
            "collections": [{"id": "C", "name": "color", "modes": [{"id": "m", "name": "A"}]}],
            "variables": [{"id": "V1", "collection": "C", "name": "primary/500", "resolvedType": "color", "values": {"m": {"type": "color", "value": {"a": 1.0, "r": 0.231, "g": 0.357, "b": 0.859}}}}]
        }));
        let styles = Styles::default();
        let with_fill = |c1: &str, c2: &str| {
            let mut v = vec![(json!({"name": "Row", "type": "COMPONENT", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 100.0, "height": 40.0}}), 0)];
            for (name, c) in [("A", c1), ("B", c2)] {
                v.push((json!({"name": name, "type": "FRAME", "fills": [{"type": "solid", "color": c}], "boundVariables": {"fill": "V1"}, "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0}}), 1));
            }
            v
        };
        let a = with_fill("#3B5BDB", "#3B5BDB");
        let b = with_fill("#2F4AB5", "#111111");
        let d = compare(&snapshot(Lang::En, &a, &vars, &styles), &snapshot(Lang::En, &b, &vars, &styles));
        let said = list(Lang::En, &d).texts(Lang::En);
        assert!(said.iter().any(|l| l == "↳ A") && said.iter().any(|l| l == "↳ B") && !said.iter().any(|l| l.contains(" · ")), "{said:?}");
    }

    #[test]
    fn shown_and_swapped_changes_group_like_any_other() {
        let frame = |main_a: &str, main_b: &str, vis: bool| {
            vec![
                (json!({"name": "Row", "type": "COMPONENT", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 100.0, "height": 40.0}}), 0),
                (json!({"name": "Left", "type": "INSTANCE", "mainComponentName": main_a, "visible": vis}), 1),
                (json!({"name": "Right", "type": "INSTANCE", "mainComponentName": main_b, "visible": vis}), 1),
            ]
        };
        // Two instances that become another component the same way: one line.
        assert_eq!(texts(&diff_of(&frame("Name=a", "Name=a", true), &frame("Name=b", "Name=b", true))), vec!["↳ Left · Right: Name: a → b"]);
        // The same swap to different components: two lines.
        assert_eq!(texts(&diff_of(&frame("Name=a", "Name=a", true), &frame("Name=b", "Name=c", true))), vec!["↳ Left: Name: a → b", "↳ Right: Name: a → c"]);
        // Both hidden: one line.
        assert_eq!(texts(&diff_of(&frame("Name=a", "Name=a", true), &frame("Name=a", "Name=a", false))), vec!["↳ Left · Right: hidden"]);
    }

    #[test]
    fn a_long_text_groups_only_when_the_whole_text_is_the_same() {
        // The two texts differ at the end, far from the first difference: the quoted window is the same, the texts are not.
        let long = |tail: &str| format!("Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon {tail}");
        let (x, y) = (long("one"), long("two"));
        let from = [long("same").as_str().to_string(), long("same")];
        let from: Vec<&str> = from.iter().map(String::as_str).collect();
        let a = rows_of(&["A", "B"], &from);
        let mut to = [x.replace("Alpha", "ALPHA"), y.replace("Alpha", "ALPHA")];
        let (wa, wb) = (windows(from[0], &to[0]), windows(from[1], &to[1]));
        assert_eq!(wa.1, wb.1, "the quoted window of the new text is the same for both");
        let said = texts(&diff_of(&a, &rows_of(&["A", "B"], &to.iter().map(String::as_str).collect::<Vec<_>>())));
        assert!(said.iter().any(|l| l == "↳ A") && said.iter().any(|l| l == "↳ B"), "{said:?}");
        // The whole text equal: one group.
        to[1] = to[0].clone();
        let said = texts(&diff_of(&a, &rows_of(&["A", "B"], &to.iter().map(String::as_str).collect::<Vec<_>>())));
        assert_eq!(said[0], "↳ A · B", "{said:?}");
    }

    #[test]
    fn the_twelve_lines_count_after_grouping_and_the_rest_is_counted_right() {
        // Thirty layers, ten each of three texts: three groups of two lines, nothing left out.
        let names: Vec<String> = (0..30).map(|i| format!("L{i:02}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let to: Vec<&str> = (0..30).map(|i| ["p", "q", "r"][i % 3]).collect();
        let l = list(Lang::En, &diff_of(&rows_of(&names, &["a"; 30]), &rows_of(&names, &to)));
        assert_eq!((l.lines.len(), l.more), (6, 0));
        assert_eq!(l.lines[0].text, "↳ L00 · L03 · L06 +7");
        // Twenty distinct layers: as before, 6 groups of heading + line, and the 14 left are counted.
        let names: Vec<String> = (0..20).map(|i| format!("L{i:02}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let to: Vec<String> = (0..20).map(|i| format!("t{i}")).collect();
        let to: Vec<&str> = to.iter().map(String::as_str).collect();
        let l = list(Lang::En, &diff_of(&rows_of(&names, &["a"; 20]), &rows_of(&names, &to)));
        assert_eq!((l.lines.len(), l.more), (12, 14));
        assert_eq!(l.texts(Lang::En).last().unwrap(), "+14 more differences");
        // A group of ten layers and eight distinct ones: the group is 2 lines, each distinct layer 2 more.
        let names: Vec<String> = (0..18).map(|i| format!("L{i:02}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let to: Vec<String> = (0..18).map(|i| if i < 10 { "same".to_string() } else { format!("t{i}") }).collect();
        let to: Vec<&str> = to.iter().map(String::as_str).collect();
        let l = list(Lang::En, &diff_of(&rows_of(&names, &["a"; 18]), &rows_of(&names, &to)));
        assert_eq!(l.lines[0].text, "↳ L00 · L01 · L02 +7");
        assert_eq!((l.lines.len(), l.more), (12, 3), "the group and 5 of the 8 distinct layers fill 2 + 5×2 = 12 lines; 3 changes are left out");
    }

    #[test]
    fn radius_padding_and_typography_changes_are_listed() {
        let d = diff_of(&button(8.0, 16.0, "#3B5BDB", 14.0), &button(12.0, 20.0, "#3B5BDB", 16.0));
        assert_eq!(
            texts(&d),
            vec![
                "Radius: 8 → 12",
                "Horizontal padding: 16 → 20",
                "↳ Label / Text",
                "Typography: Work Sans 14/150% Medium → Work Sans 16/150% Medium",
            ]
        );
    }

    fn side(token: Option<&str>, value: &str) -> Side {
        Side { token: token.map(ToString::to_string), value: value.to_string() }
    }

    fn said(lang: Lang, label: Label, from: Side, to: Side) -> String {
        change_text(lang, label, &from, &to)
    }

    #[test]
    fn each_side_carries_its_value_after_its_token() {
        let typo = Label::Prop(Prop::Typography);
        // token -> value
        assert_eq!(
            said(Lang::En, typo, side(Some("text/label-m"), "Inter 14/20 Medium"), side(None, "Inter 12/16 Medium")),
            "Typography: text/label-m (Inter 14/20 Medium) → Inter 12/16 Medium"
        );
        assert_eq!(
            said(Lang::En, typo, side(Some("text/h3"), "Inter 24/32 SemiBold, letter spacing -1%"), side(None, "Inter 24/32 SemiBold, letter spacing -1%")),
            "Typography: text/h3 (Inter 24/32 SemiBold, letter spacing -1%) → Inter 24/32 SemiBold, letter spacing -1%"
        );
        // token -> token
        assert_eq!(
            said(Lang::En, typo, side(Some("text/label-m"), "Inter 14/20 Medium"), side(Some("text/label-l"), "Inter 16/24 Medium")),
            "Typography: text/label-m (Inter 14/20 Medium) → text/label-l (Inter 16/24 Medium)"
        );
        // value -> token, and the other labels
        assert_eq!(said(Lang::En, Label::Prop(Prop::Stroke), side(None, "none"), side(Some("Colors/color/border"), "#E3E7E8")), "Stroke: none → Colors/color/border (#E3E7E8)");
        assert_eq!(said(Lang::En, Label::Prop(Prop::Radius), side(Some("Radius/radius/default"), "10"), side(None, "8")), "Radius: Radius/radius/default (10) → 8");
        assert_eq!(said(Lang::En, Label::Prop(Prop::Fill), side(Some("Colors/color/primary"), "#C6005C"), side(None, "none")), "Fill: Colors/color/primary (#C6005C) → none");
        assert_eq!(said(Lang::En, Label::Prop(Prop::Gap), side(Some("Spacing/space/8"), "8"), side(None, "6")), "Gap: Spacing/space/8 (8) → 6");
        // Words stay translated.
        assert_eq!(
            said(Lang::Pt, typo, side(Some("text/h3"), "Inter 24/32 SemiBold, espaçamento entre letras -1%"), side(None, "Inter 24/32 SemiBold, espaçamento entre letras -1%")),
            "Tipografia: text/h3 (Inter 24/32 SemiBold, espaçamento entre letras -1%) → Inter 24/32 SemiBold, espaçamento entre letras -1%"
        );
        assert_eq!(said(Lang::Pt, Label::Prop(Prop::Fill), side(Some("Colors/color/primary"), "#C6005C"), side(None, "none")), "Preenchimento: Colors/color/primary (#C6005C) → nenhum");
    }

    #[test]
    fn a_line_with_no_token_is_what_it_was_and_effects_carry_their_description() {
        // value -> value
        assert_eq!(said(Lang::En, Label::Width, side(None, "100"), side(None, "120")), "Width: 100 → 120");
        assert_eq!(said(Lang::En, Label::Prop(Prop::Fill), side(None, "#3B5BDB"), side(None, "#2F4AB5")), "Fill: #3B5BDB → #2F4AB5");
        assert_eq!(said(Lang::En, Label::Content, side(None, "Hi"), side(None, "Bye")), "Text: \"Hi\" → \"Bye\"");
        // Effects: none on one side, a style on the other.
        assert_eq!(
            said(Lang::En, Label::Effects, side(None, "none"), side(Some("shadow/sm"), "Drop shadow 0 1 blur 2 #000000 at 5%")),
            "Effects: none → shadow/sm (Drop shadow 0 1 blur 2 #000000 at 5%)"
        );
        assert_eq!(
            said(Lang::Pt, Label::Effects, side(None, "none"), side(Some("shadow/sm"), "Sombra projetada 0 1 desfoque 2 #000000 a 5%")),
            "Efeitos: nenhum → shadow/sm (Sombra projetada 0 1 desfoque 2 #000000 a 5%)"
        );
    }

    #[test]
    fn a_token_with_an_empty_or_dash_value_or_its_own_text_is_shown_alone() {
        let fill = Label::Prop(Prop::Fill);
        assert_eq!(said(Lang::En, fill, side(Some("Colors/a"), ""), side(None, "#FFF")), "Fill: Colors/a → #FFF");
        assert_eq!(said(Lang::En, fill, side(Some("Colors/a"), "  "), side(Some("Colors/b"), "—")), "Fill: Colors/a → Colors/b");
        assert_eq!(said(Lang::En, fill, side(Some("Colors/a"), "Colors/a"), side(None, "#FFF")), "Fill: Colors/a → #FFF");
    }

    #[test]
    fn a_token_is_named_before_its_value() {
        let vars = Vars::parse(&json!({
            "collections": [{"id": "C", "name": "color", "modes": [{"id": "m", "name": "A"}]}],
            "variables": [
                {"id": "V1", "collection": "C", "name": "primary/500", "resolvedType": "color", "values": {"m": {"type": "color", "value": {"a": 1.0, "r": 0.231, "g": 0.357, "b": 0.859}}}},
                {"id": "V2", "collection": "C", "name": "primary/700", "resolvedType": "color", "values": {"m": {"type": "color", "value": {"a": 1.0, "r": 0.184, "g": 0.29, "b": 0.71}}}}
            ]
        }));
        let styles = Styles::default();
        let mut a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let mut b = button(8.0, 16.0, "#2F4AB5", 14.0);
        a[0].0["boundVariables"] = json!({"fill": "V1"});
        b[0].0["boundVariables"] = json!({"fill": "V2"});
        let d = compare(&snapshot(Lang::En, &a, &vars, &styles), &snapshot(Lang::En, &b, &vars, &styles));
        assert_eq!(texts(&d), vec!["Fill: color/primary/500 (#3B5BDB) → color/primary/700 (#2F4AB5)"]);
    }

    #[test]
    fn a_layer_that_is_switched_off_or_on_is_one_line() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let mut b = a.clone();
        b[1].0["visible"] = json!(false);
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Icon: hidden"]);
        assert_eq!(texts(&diff_of(&b, &a)), vec!["↳ Icon: visible"]);
    }

    #[test]
    fn what_is_under_a_hidden_layer_is_not_compared() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let mut b = button(8.0, 16.0, "#3B5BDB", 18.0);
        b[2].0["visible"] = json!(false);
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Label: hidden"]);
    }

    #[test]
    fn under_half_a_pixel_is_no_difference() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let b = button(8.3, 16.4, "#3B5BDB", 14.0);
        assert!(diff_of(&a, &b).changes.is_empty());
        let c = button(8.6, 16.0, "#3B5BDB", 14.0);
        assert_eq!(texts(&diff_of(&a, &c)), vec!["Radius: 8 → 8.6"]);
        assert_eq!(texts(&diff_of(&a, &a)), vec!["No differences from default"]);
    }

    #[test]
    fn effects_are_read_and_an_unreadable_one_is_said() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let mut b = a.clone();
        b[0].0["effects"] = json!([{"type": "dropShadow", "offset": [0.0, 8.0], "radius": 8.0, "spread": 0.0, "visible": true, "color": {"r": 0.231, "g": 0.357, "b": 0.859, "a": 0.3}}]);
        assert_eq!(texts(&diff_of(&a, &b)), vec!["Effects: none → Drop shadow 0 8 blur 8 #3B5BDB at 30%"]);
        let mut c = a.clone();
        let mut d = a.clone();
        c[0].0["effects"] = json!([{"type": "noise", "density": 0.2, "visible": true}]);
        d[0].0["effects"] = json!([{"type": "noise", "density": 0.6, "visible": true}]);
        assert_eq!(texts(&diff_of(&c, &d)), vec!["No readable differences (visual effect only)"]);
    }

    #[test]
    fn effects_and_opacity_are_worded_in_portuguese_with_the_english_ones_unchanged() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let mut b = a.clone();
        b[0].0["effects"] = json!([
            {"type": "dropShadow", "offset": [0.0, 8.0], "radius": 8.0, "spread": 2.0, "visible": true, "color": {"r": 0.231, "g": 0.357, "b": 0.859, "a": 0.3}},
            {"type": "innerShadow", "offset": [1.0, 1.0], "radius": 4.0, "spread": 0.0, "visible": true, "color": {"r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0}},
            {"type": "layerBlur", "radius": 6.0, "visible": true},
            {"type": "backgroundBlur", "radius": 12.0, "visible": true}
        ]);
        let say = |lang: Lang| list(lang, &compare(&snapshot(lang, &a, &Vars::default(), &Styles::default()), &snapshot(lang, &b, &Vars::default(), &Styles::default()))).texts(lang);
        assert_eq!(say(Lang::En), vec!["Effects: none → Drop shadow 0 8 blur 8 spread 2 #3B5BDB at 30%; Inner shadow 1 1 blur 4 #000000; Layer blur 6; Background blur 12"]);
        assert_eq!(say(Lang::Pt), vec!["Efeitos: nenhum → Sombra projetada 0 8 desfoque 8 expansão 2 #3B5BDB a 30%; Sombra interna 1 1 desfoque 4 #000000; Desfoque de camada 6; Desfoque de fundo 12"]);
        // Opacity of a fill: "… at 50%" / "… a 50%".
        let mut c = a.clone();
        c[0].0["fills"] = json!([{"type": "solid", "color": "#3B5BDB", "opacity": 0.5}]);
        let fill = |lang: Lang| list(lang, &compare(&snapshot(lang, &a, &Vars::default(), &Styles::default()), &snapshot(lang, &c, &Vars::default(), &Styles::default()))).texts(lang);
        assert_eq!(fill(Lang::En), vec!["Fill: #3B5BDB → #3B5BDB at 50%"]);
        assert_eq!(fill(Lang::Pt), vec!["Preenchimento: #3B5BDB → #3B5BDB a 50%"]);
    }

    #[test]
    fn colour_alpha_is_whole_percent_and_layer_and_paint_opacity_keep_one_decimal() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let say = |lang: Lang, b: &[(Value, usize)]| list(lang, &compare(&snapshot(lang, &a, &Vars::default(), &Styles::default()), &snapshot(lang, b, &Vars::default(), &Styles::default()))).texts(lang);
        let shadow = |alpha: f64| {
            let mut b = a.clone();
            b[0].0["effects"] = json!([{"type": "dropShadow", "offset": [0.0, 1.0], "radius": 2.0, "visible": true, "color": {"r": 0.0, "g": 0.0, "b": 0.0, "a": alpha}}]);
            b
        };
        // An effect colour's alpha: whole percents (13/255 is 5.098%, written 5%).
        for (alpha, text) in [(13.0 / 255.0, "5%"), (0.05098, "5%"), (0.333, "33%"), (0.3, "30%"), (0.5, "50%")] {
            assert_eq!(say(Lang::En, &shadow(alpha)), vec![format!("Effects: none → Drop shadow 0 1 blur 2 #000000 at {text}")], "{alpha}");
            assert_eq!(say(Lang::Pt, &shadow(alpha)), vec![format!("Efeitos: nenhum → Sombra projetada 0 1 desfoque 2 #000000 a {text}")], "{alpha}");
        }
        // The limit of the suffix: from 0.998 up the colour is written alone; 0.997 and 0.0004 are below it.
        assert_eq!(say(Lang::En, &shadow(0.9996)), vec!["Effects: none → Drop shadow 0 1 blur 2 #000000"]);
        assert_eq!(say(Lang::En, &shadow(0.998)), vec!["Effects: none → Drop shadow 0 1 blur 2 #000000"]);
        assert_eq!(say(Lang::En, &shadow(0.997)), vec!["Effects: none → Drop shadow 0 1 blur 2 #000000 at 100%"]);
        assert_eq!(say(Lang::En, &shadow(0.0004)), vec!["Effects: none → Drop shadow 0 1 blur 2 #000000 at 0%"]);
        // A paint: one decimal, the limit stays at 0.995.
        let paint = |opacity: f64| {
            let mut b = a.clone();
            b[0].0["fills"] = json!([{"type": "solid", "color": "#3B5BDB", "opacity": opacity}]);
            b
        };
        assert_eq!(say(Lang::En, &paint(0.333)), vec!["Fill: #3B5BDB → #3B5BDB at 33.3%"]);
        assert_eq!(say(Lang::Pt, &paint(0.333)), vec!["Preenchimento: #3B5BDB → #3B5BDB a 33.3%"], "the decimal point is not localised");
        assert_eq!(say(Lang::En, &paint(0.9949)), vec!["Fill: #3B5BDB → #3B5BDB at 99.5%"]);
        assert_eq!(say(Lang::En, &paint(0.995)), vec!["No differences from default"], "0.995 is written as no opacity at all");
        // The layer: one decimal.
        let mut layer = a.clone();
        layer[0].0["opacity"] = json!(0.333);
        assert_eq!(say(Lang::En, &layer), vec!["Opacity: 100% → 33.3%"]);
        assert_eq!(say(Lang::Pt, &layer), vec!["Opacidade: 100% → 33.3%"]);
    }

    #[test]
    fn an_effect_the_dictionary_does_not_know_is_shown_as_the_host_names_it() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        for kind in ["noise", "texture", "glass"] {
            let mut b = a.clone();
            b[0].0["effects"] = json!([{"type": kind, "visible": true}]);
            for lang in [Lang::En, Lang::Pt] {
                let said = list(lang, &compare(&snapshot(lang, &a, &Vars::default(), &Styles::default()), &snapshot(lang, &b, &Vars::default(), &Styles::default()))).texts(lang);
                assert_eq!(said, vec![format!("{}: {} → {kind}", t(lang, Key::DeltaEffects), t(lang, Key::ValueNone))], "{kind} {lang:?}");
            }
        }
    }

    #[test]
    fn a_list_stops_at_twelve_lines_and_counts_the_rest() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        // Twenty layers whose text changes, each to its own text: a heading and a line each (the same change would be one group).
        let mut base = a.clone();
        let mut var = a.clone();
        for i in 0..20 {
            let n = json!({"name": format!("Row {i}"), "type": "TEXT", "characters": "a", "fontFamily": "W", "fontSize": 14.0, "fontStyle": "R", "lineHeight": "150%", "letterSpacing": "0%"});
            let mut m = n.clone();
            m["characters"] = json!(format!("b{i}"));
            base.push((n, 1));
            var.push((m, 1));
        }
        let d = diff_of(&base, &var);
        assert_eq!(d.changes.len(), 20);
        let l = list(Lang::En, &d);
        assert_eq!(l.lines.len(), MAX_LINES);
        assert_eq!(l.more, 20 - 6);
        assert_eq!(l.texts(Lang::En).last().unwrap(), "+14 more differences");
        assert!(l.lines.first().unwrap().heading);
    }

    #[test]
    fn repeated_names_are_told_apart_by_index() {
        let node = |name: &str, size: f64| (json!({"name": name, "type": "TEXT", "characters": "a", "fontFamily": "W", "fontSize": size, "fontStyle": "R", "lineHeight": "150%", "letterSpacing": "0%"}), 1);
        let root = (json!({"name": "R", "type": "COMPONENT"}), 0);
        let a = vec![root.clone(), node("Cell", 14.0), node("Cell", 14.0)];
        let b = vec![root, node("Cell", 14.0), node("Cell", 16.0)];
        let snaps = snapshot(Lang::En, &a, &Vars::default(), &Styles::default());
        assert_eq!(snaps[1].path, "/Cell[0]");
        assert_eq!(snaps[2].path, "/Cell[1]");
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Cell[1]", "Typography: W 14/150% R → W 16/150% R"]);
    }

    const WORDS: &str = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor";

    fn swapped_word(text: &str, from: &str, to: &str) -> String {
        text.replacen(from, to, 1)
    }

    fn said_text(lang: Lang, a: &str, b: &str) -> String {
        change_text(lang, Label::Content, &Side { token: None, value: a.to_string() }, &Side { token: None, value: b.to_string() })
    }

    #[test]
    fn a_text_of_70_characters_or_less_is_whole_with_no_mark() {
        let a = "x".repeat(70);
        let b = format!("{}y", "x".repeat(69));
        assert_eq!(windows(&a, &b), (a.clone(), b));
        assert_eq!(windows("Button", "Cancel"), ("Button".to_string(), "Cancel".to_string()));
        assert!(!said_text(Lang::En, &a, "short").contains('['));
    }

    #[test]
    fn the_window_starts_and_ends_between_words_and_is_marked() {
        // A difference in the middle of a long text: a word boundary before and after, a mark on each side.
        let (a, b) = windows(WORDS, &swapped_word(WORDS, "kilo", "KILO"));
        assert_eq!(a, "[…] india juliet kilo lima mike november oscar papa quebec romeo sierra […]");
        assert_eq!(b, "[…] india juliet KILO lima mike november oscar papa quebec romeo sierra […]");
        assert!(a.chars().count() <= MAX_CONTENT + CUT_START.chars().count() + CUT_END.chars().count());
        // Only the start is cut (the difference is near the end).
        let (a, b) = windows(WORDS, &swapped_word(WORDS, "victor", "VICTOR"));
        assert!(a.starts_with("[…] ") && !a.ends_with("[…]"), "{a}");
        assert!(a.ends_with("uniform victor") && b.ends_with("uniform VICTOR"), "{a} / {b}");
        // A difference inside the very first word: no cut at the start, not even a part of that word.
        let (a, _) = windows(WORDS, &swapped_word(WORDS, "alpha", "alpHa"));
        assert!(a.starts_with("alpha bravo") && !a.starts_with('['), "{a}");
        // Only the end is cut (the difference is at the start).
        let (a, b) = windows(WORDS, &swapped_word(WORDS, "alpha", "ALPHA"));
        assert!(!a.starts_with('[') && a.ends_with(" […]") && b.starts_with("ALPHA bravo"), "{a} / {b}");
        // A cut never falls inside a word that fits.
        for w in [&a, &b] {
            let body = w.trim_end_matches(" […]");
            assert!(WORDS.to_uppercase().contains(&body.to_uppercase()) && body.split(' ').all(|x| WORDS.split(' ').any(|y| y.eq_ignore_ascii_case(x))), "{body}");
        }
    }

    #[test]
    fn the_first_difference_is_always_inside_the_window_on_both_sides() {
        let base: Vec<String> = (0..60).map(|i| format!("w{i:02}")).collect();
        let text = base.join(" ");
        for i in [0usize, 1, 5, 17, 30, 44, 58, 59] {
            let mut other = base.clone();
            other[i] = format!("X{i:02}");
            let (a, b) = windows(&text, &other.join(" "));
            assert!(a.contains(&base[i]), "{i}: {a}");
            assert!(b.contains(&format!("X{i:02}")), "{i}: {b}");
        }
        // The text of one side is the start of the other: the longer one is cut, the shorter one is whole.
        let (a, b) = windows(&text, &text[..100]);
        assert!(b.trim_end().ends_with("w24"), "{b}");
        assert!(a.ends_with(" […]") && !b.ends_with("[…]"), "{a} / {b}");
    }

    #[test]
    fn a_word_longer_than_the_window_is_cut_at_its_edge() {
        let a = format!("{}X", "a".repeat(200));
        let b = format!("{}Y", "a".repeat(200));
        let (wa, wb) = windows(&a, &b);
        // The word has no space to cut at: the window opens CONTEXT characters before the difference, inside the word.
        assert_eq!((wa.as_str(), wb.as_str()), ("[…] aaaaaaaaaaX", "[…] aaaaaaaaaaY"));
        // No space anywhere, difference far from the start, and none to the right: no loop, window of MAX_CONTENT.
        let (wa, wb) = windows(&"a".repeat(300), &format!("{}b", "a".repeat(299)));
        assert!(wa.chars().count() <= MAX_CONTENT + 8 && wb.contains('b'), "{wa} / {wb}");
    }

    #[test]
    fn the_cut_marks_are_not_the_designers_ellipsis_which_stays() {
        // The Card of the demo: the short variant ends with an ellipsis of its own.
        let long = "Design systems earn their keep when the people who build with them can see exactly what a component is made of: which tokens it reads, how much room it keeps around its content, and what changes between one variant and the next.";
        let short = format!("{}…", &long[..long.find("how m").unwrap() + 5]);
        let (a, b) = windows(long, &short);
        assert_eq!(a, "[…] reads, how much room it keeps around its content, and what changes […]");
        assert_eq!(b, "[…] reads, how m…", "the last … is the file's own: it is not a cut mark, and it is not touched");
        // Both marks on one side, an ellipsis of the file inside the window.
        let (a, _) = windows(&format!("{WORDS}…"), &swapped_word(WORDS, "kilo", "KILO"));
        assert!(a.starts_with("[…] ") && a.ends_with(" […]"), "{a}");
    }

    #[test]
    fn a_change_only_in_spacing_or_line_breaks_says_so_in_both_languages() {
        for (a, b) in [("Line one\nLine two", "Line one Line two"), ("Save", "Save "), ("a  b", "a b"), ("a\r\nb", "a b"), ("\tHi there", "Hi there")] {
            assert_eq!(said_text(Lang::En, a, b), "Text: whitespace or line breaks only", "{a:?} {b:?}");
            assert_eq!(said_text(Lang::Pt, a, b), "Texto: só espaços ou quebras de linha", "{a:?} {b:?}");
        }
        // It still counts as one line of text in a variant's list.
        let text = |s: &str| vec![(json!({"name": "Row", "type": "COMPONENT", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 100.0, "height": 40.0}}), 0), (json!({"name": "T", "type": "TEXT", "characters": s}), 1)];
        assert_eq!(texts(&diff_of(&text("a\nb"), &text("a b"))), vec!["↳ T", "Text: whitespace or line breaks only"]);
    }

    #[test]
    fn changes_of_case_and_punctuation_are_still_changes() {
        assert_eq!(said_text(Lang::En, "Save changes", "Save Changes"), "Text: \"Save changes\" → \"Save Changes\"");
        assert_eq!(said_text(Lang::En, "Hello world.", "Hello world"), "Text: \"Hello world.\" → \"Hello world\"");
        assert_eq!(said_text(Lang::Pt, "Olá, mundo", "Olá mundo"), "Texto: \"Olá, mundo\" → \"Olá mundo\"");
    }

    #[test]
    fn long_text_is_cut_around_where_it_differs() {
        let (a, b) = windows("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor", "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do incididunt ut");
        assert_ne!(a, b);
        assert!(a.contains("eiusmod") && b.contains("incididunt"), "{a} / {b}");
        assert_eq!(windows("Button", "Cancel"), ("Button".to_string(), "Cancel".to_string()));
    }

    #[test]
    fn one_padding_number_stands_for_both_sides() {
        let one = Entry::new(Label::Prop(Prop::PadV), None, "16".into(), vec![16.0]);
        let two = Entry::new(Label::Prop(Prop::PadV), None, "16 / 24".into(), vec![16.0, 24.0]);
        let same = Entry::new(Label::Prop(Prop::PadV), None, "16 / 16.2".into(), vec![16.0, 16.2]);
        assert!(!one.same(&two));
        assert!(one.same(&same));
    }

    #[test]
    fn sizes_that_follow_the_parent_are_not_compared() {
        let node = |w: f64, sizing: &str| {
            (json!({"name": "Box", "type": "FRAME", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": w, "height": 10.0}, "sizing": {"horizontal": sizing, "vertical": "fixed"}}), 1)
        };
        let root = (json!({"name": "R", "type": "COMPONENT"}), 0);
        let (a, b) = (vec![root.clone(), node(100.0, "fill")], vec![root.clone(), node(140.0, "fill")]);
        assert!(diff_of(&a, &b).changes.is_empty());
        let (a, b) = (vec![root.clone(), node(100.0, "fixed")], vec![root, node(140.0, "fixed")]);
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Box", "Width: 100 → 140"]);
    }

    /// A component root `w` × `h` with the given sizing modes (`None`: the host says nothing about sizing).
    fn sized(w: f64, h: f64, sizing: Option<(&str, &str)>) -> Vec<(Value, usize)> {
        let mut n = json!({"name": "Box", "type": "COMPONENT", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": w, "height": h}});
        if let (Some((sh, sv)), Some(o)) = (sizing, n.as_object_mut()) {
            o.insert("sizing".into(), json!({"horizontal": sh, "vertical": sv}));
        }
        vec![(n, 0)]
    }

    #[test]
    fn a_hugging_or_filling_component_has_no_size_difference() {
        // Both hug: the size follows from the padding and the content, which have lines of their own.
        assert!(diff_of(&sized(100.0, 36.0, Some(("hug", "hug"))), &sized(123.0, 48.0, Some(("hug", "hug")))).changes.is_empty());
        // Both fill: the same.
        assert!(diff_of(&sized(100.0, 36.0, Some(("fill", "fill"))), &sized(140.0, 50.0, Some(("fill", "fill")))).changes.is_empty());
        // Only the axis that is fixed counts: a fixed width and a hugging height (the Card).
        let (a, b) = (sized(384.0, 422.0, Some(("fixed", "hug"))), sized(320.0, 238.0, Some(("fixed", "hug"))));
        assert_eq!(texts(&diff_of(&a, &b)), vec!["Width: 384 → 320"]);
    }

    #[test]
    fn a_fixed_component_keeps_its_numbers() {
        let (a, b) = (sized(100.0, 40.0, Some(("fixed", "fixed"))), sized(120.0, 48.0, Some(("fixed", "fixed"))));
        assert_eq!(texts(&diff_of(&a, &b)), vec!["Width: 100 → 120", "Height: 40 → 48"]);
    }

    #[test]
    fn a_change_of_mode_says_the_modes_with_the_fixed_number() {
        let hug = sized(100.0, 36.0, Some(("hug", "hug")));
        let fixed = sized(320.0, 36.0, Some(("fixed", "hug")));
        assert_eq!(texts(&diff_of(&hug, &fixed)), vec!["Width: Hug → Fixed 320"]);
        assert_eq!(texts(&diff_of(&fixed, &hug)), vec!["Width: Fixed 320 → Hug"]);
        // The height too, and Fill is a mode like Hug; no separate "resizing" line says the same again.
        let (a, b) = (sized(100.0, 36.0, Some(("fill", "hug"))), sized(100.0, 90.0, Some(("fill", "fixed"))));
        assert_eq!(texts(&diff_of(&a, &b)), vec!["Height: Hug → Fixed 90"]);
        let (a, b) = (sized(100.0, 36.0, Some(("hug", "hug"))), sized(100.0, 36.0, Some(("fill", "hug"))));
        assert_eq!(texts(&diff_of(&a, &b)), vec!["Width: Hug → Fill"]);
        // In Portuguese, with the words the file already uses for the modes.
        let d = diff_of(&hug, &fixed);
        assert_eq!(list(Lang::Pt, &d).texts(Lang::Pt), vec!["Largura: Ajustar ao conteúdo → Fixo 320"]);
    }

    #[test]
    fn a_component_with_no_sizing_in_the_data_counts_as_fixed() {
        // The host gives no sizing for some components (an icon): its size is a number, as it always was.
        assert_eq!(texts(&diff_of(&sized(16.0, 16.0, None), &sized(24.0, 24.0, None))), vec!["Width: 16 → 24", "Height: 16 → 24"]);
        // The default has none and the variant says hug: the number of the one is not compared with the mode of the other
        // as a number: it is a change of mode.
        assert_eq!(texts(&diff_of(&sized(100.0, 36.0, None), &sized(100.0, 36.0, Some(("hug", "fixed"))))), vec!["Width: Fixed 100 → Hug"]);
    }

    #[test]
    fn the_resizing_line_of_a_layer_inside_stays() {
        // The rule is about the component's own size; a layer inside keeps its mode line (and its size only when it is fixed).
        let child = |w: f64, sh: &str| (json!({"name": "Box", "type": "FRAME", "absoluteBounds": {"x": 0.0, "y": 0.0, "width": w, "height": 10.0}, "sizing": {"horizontal": sh, "vertical": "fixed"}}), 1);
        let root = || sized(100.0, 36.0, Some(("hug", "hug"))).remove(0);
        let (a, b) = (vec![root(), child(100.0, "hug")], vec![root(), child(100.0, "fill")]);
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Box", "Horizontal resizing: Hug → Fill"]);
    }

    fn side_with(token: Option<&str>, value: &str) -> Side {
        Side { token: token.map(ToString::to_string), value: value.to_string() }
    }

    fn changed(label: Label, from: Side, to: Side) -> Listing {
        let diff = Diff { changes: vec![Change { layer: None, kind: Kind::Changed { label, from, to }, minor: false }], unreadable_effect: false };
        list(Lang::En, &diff)
    }

    #[test]
    fn a_change_with_a_token_or_a_colour_comes_as_its_parts_and_its_text_stays() {
        let fill = Label::Prop(Prop::Fill);
        let l = changed(fill, side_with(Some("Colors/color/primary"), "#C6005C"), side_with(Some("Colors/color/secondary"), "#F4F4F5"));
        let line = &l.lines[0];
        assert!(line.entry && !line.heading);
        let c = line.cells.as_ref().unwrap();
        assert_eq!(c.label, "Fill");
        assert_eq!(c.from, Cell { token: Some("Colors/color/primary".into()), value: "#C6005C".into(), colour: Some(Swatch::Solid) });
        assert_eq!(c.to.token.as_deref(), Some("Colors/color/secondary"));
        // The text, which the baseline records, is what it was.
        assert_eq!(l.texts(Lang::En), vec!["Fill: Colors/color/primary (#C6005C) → Colors/color/secondary (#F4F4F5)"]);
        // A property with a token and no colour is also stacked: a number is a value.
        let r = changed(Label::Prop(Prop::Radius), side_with(Some("Radius/radius/default"), "10"), side_with(None, "8"));
        let c = r.lines[0].cells.as_ref().unwrap();
        assert_eq!((c.from.value.as_str(), c.to.value.as_str(), c.from.colour), ("10", "8", None));
        assert_eq!(r.texts(Lang::En), vec!["Radius: Radius/radius/default (10) → 8"]);
    }

    #[test]
    fn a_change_with_no_token_and_no_colour_is_one_line() {
        for (label, a, b) in [(Label::StrokeWeight, "0", "1"), (Label::Opacity, "100%", "50%"), (Label::Width, "384", "320"), (Label::Prop(Prop::Typography), "Inter 14/20", "Inter 12/16")] {
            let l = changed(label, side_with(None, a), side_with(None, b));
            assert!(l.lines[0].entry, "{label:?}");
            assert_eq!(l.lines[0].cells, None, "{label:?}");
        }
        // A text's change has quotes of its own: one line, whatever the sides are.
        let t = changed(Label::Content, side_with(None, "Button"), side_with(None, "Cancel"));
        assert_eq!(t.lines[0].cells, None);
        // A fill that goes from none to none (no colour on either side) has none to draw.
        assert_eq!(changed(Label::Prop(Prop::Fill), side_with(None, "none"), side_with(None, "none")).lines[0].cells, None);
    }

    #[test]
    fn a_side_with_no_token_says_so_and_a_token_alone_has_a_dash_for_its_value() {
        let l = changed(Label::Prop(Prop::Fill), side_with(Some("Colors/color/primary"), "#C6005C"), side_with(None, "none"));
        let c = l.lines[0].cells.as_ref().unwrap();
        assert_eq!(c.to.token, None);
        assert_eq!((c.to.value.as_str(), c.to.colour), ("none", None));
        // A token whose value is its own name (or empty) has nothing else to say.
        let t = changed(Label::Prop(Prop::Radius), side_with(Some("Radius/radius/default"), "Radius/radius/default"), side_with(Some("Radius/radius/small"), ""));
        let c = t.lines[0].cells.as_ref().unwrap();
        assert_eq!((c.from.value.as_str(), c.to.value.as_str()), (DASH, DASH));
    }

    #[test]
    fn a_colour_with_an_alpha_is_an_empty_square_and_the_alpha_is_in_the_hex() {
        assert_eq!(colour_of("#C6005C"), Some(Swatch::Solid));
        assert_eq!(colour_of("#c6005cff"), Some(Swatch::Solid));
        assert_eq!(colour_of("#0000000D"), Some(Swatch::Hollow));
        assert_eq!(colour_of("#C6005C at 50%"), Some(Swatch::Hollow));
        let l = changed(Label::Prop(Prop::Fill), side_with(None, "#0000000D"), side_with(None, "#FFFFFF"));
        let c = l.lines[0].cells.as_ref().unwrap();
        assert_eq!((c.from.value.as_str(), c.from.colour), ("#0000000D", Some(Swatch::Hollow)));
        assert_eq!(c.to.colour, Some(Swatch::Solid));
    }

    #[test]
    fn a_gradient_an_image_or_none_has_no_square_only_its_word() {
        for word in ["linearGradient", "image", "none", "#12", "#GGGGGG", "red"] {
            assert_eq!(colour_of(word), None, "{word}");
        }
        let l = changed(Label::Prop(Prop::Fill), side_with(None, "linearGradient"), side_with(None, "#FFFFFF"));
        let c = l.lines[0].cells.as_ref().unwrap();
        assert_eq!((c.from.value.as_str(), c.from.colour), ("linearGradient", None));
        // A colour is only a colour in a fill or a stroke.
        let r = changed(Label::Prop(Prop::Radius), side_with(Some("Radius/x"), "#FFFFFF"), side_with(None, "8"));
        assert_eq!(r.lines[0].cells.as_ref().unwrap().from.colour, None);
    }

    #[test]
    fn headings_and_layers_shown_or_hidden_are_not_entries() {
        let diff = Diff {
            changes: vec![
                Change { layer: Some("/Box".into()), kind: Kind::Changed { label: Label::StrokeWeight, from: side_with(None, "0"), to: side_with(None, "1") }, minor: false },
                Change { layer: Some("/Box".into()), kind: Kind::Changed { label: Label::Opacity, from: side_with(None, "100%"), to: side_with(None, "50%") }, minor: false },
                Change { layer: Some("/Other".into()), kind: Kind::Shown(false), minor: false },
            ],
            unreadable_effect: false,
        };
        let l = list(Lang::En, &diff);
        let kinds: Vec<(bool, bool)> = l.lines.iter().map(|x| (x.heading, x.entry)).collect();
        assert_eq!(kinds, vec![(true, false), (false, true), (false, true), (false, false)]);
    }
}
