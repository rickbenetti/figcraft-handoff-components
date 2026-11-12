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
/// Pixels (and percentage points) below which two numbers are the same.
const TOLERANCE: f64 = 0.5;
/// Longest text content quoted in a line.
const MAX_CONTENT: usize = 40;

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
        let alpha = float(c, "a").unwrap_or(1.0);
        let hex = audit::hex(ch("r"), ch("g"), ch("b"), 1.0);
        out.push(' ');
        out.push_str(&if alpha < 0.998 { tf(lang, Key::AtPercent, &[("value", &hex), ("n", &number((alpha * 100.0).round()))]) } else { hex });
    }
    out
}

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
        // The component's own size always counts; a layer inside counts when it sets that size itself.
        for (label, key, axis) in [(Label::Width, "width", "horizontal"), (Label::Height, "height", "vertical")] {
            if let Some(v) = float(b, key) {
                let fixed = node.get("sizing").and_then(|s| s.get(axis)).and_then(Value::as_str) == Some("fixed");
                out.push(Entry { minor: true, derived: !root && !fixed, ..Entry::new(label, None, number(v), vec![v]) });
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
    for (label, key) in [(Label::HSizing, "horizontal"), (Label::VSizing, "vertical")] {
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

/// Two texts, each cut to [`MAX_CONTENT`] characters around the place where they first differ.
fn windows(a: &str, b: &str) -> (String, String) {
    let (a, b): (Vec<char>, Vec<char>) = (a.replace('\n', " ").chars().collect(), b.replace('\n', " ").chars().collect());
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let start = if a.len().max(b.len()) > MAX_CONTENT { common.saturating_sub(MAX_CONTENT / 4) } else { 0 };
    let cut = |v: &[char]| {
        let end = (start + MAX_CONTENT).min(v.len());
        let body: String = v.get(start..end).unwrap_or_default().iter().collect();
        format!("{}{body}{}", if start > 0 { "…" } else { "" }, if end < v.len() { "…" } else { "" })
    };
    (cut(&a), cut(&b))
}

/// "Label : from → to", each side with its value in parentheses after the name of its token (see [`show_side`]).
fn change_text(lang: Lang, label: Label, from: &Side, to: &Side) -> String {
    let (a, b) = if label == Label::Content {
        let (a, b) = windows(&from.value, &to.value);
        (format!("\"{a}\""), format!("\"{b}\""))
    } else {
        (show_side(from, label, lang), show_side(to, label, lang))
    };
    format!("{} : {a} → {b}", t(lang, label.key()))
}

/// A layer's path as a title: "Left icon / Cube".
fn title(path: &str) -> String {
    path.trim_start_matches('/').split('/').collect::<Vec<_>>().join(" / ")
}

/// The list of a variant's changes, at most [`MAX_LINES`] lines. Changes come grouped by layer, the
/// root first (no heading); a layer with several changes has a "↳ name" heading, one with a single
/// switch of visibility is one line.
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
    // A layer with a single switch of visibility is one line; so is a swapped instance, whose line names the layer for
    // its other changes too; any other gets a heading (the root none).
    let headed = |layer: Option<&String>| {
        let mut own = diff.changes.iter().filter(|c| c.layer.as_ref() == layer);
        let swapped = own.clone().any(|c| matches!(c.kind, Kind::Swapped(_)));
        layer.is_some() && !swapped && !(own.next().is_some_and(|c| matches!(c.kind, Kind::Shown(_))) && own.next().is_none())
    };
    // The budget goes to the main changes first and then to the minor ones (sizes), in tree order.
    let mut picked = vec![false; diff.changes.len()];
    let mut with_heading: Vec<Option<&String>> = Vec::new();
    let mut used = 0usize;
    for minor in [false, true] {
        for (i, c) in diff.changes.iter().enumerate().filter(|(_, c)| c.minor == minor) {
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
    for layer in order {
        let name = layer.map(|p| title(p)).unwrap_or_default();
        let mut first = true;
        for c in diff.changes.iter().zip(&picked).filter(|(c, p)| **p && c.layer.as_ref() == layer).map(|(c, _)| c) {
            if first && headed(layer) {
                lines.push(Line { text: format!("↳ {name}"), heading: true });
            }
            first = false;
            let text = match &c.kind {
                Kind::Changed { label, from, to } => change_text(lang, *label, from, to),
                Kind::Shown(on) => format!("↳ {name} : {}", t(lang, if *on { Key::DeltaVisible } else { Key::DeltaHidden })),
                Kind::Swapped(text) => format!("↳ {name} : {text}"),
            };
            lines.push(Line { text, heading: false });
            shown += 1;
        }
    }
    Listing { lines, more: diff.changes.len() - shown, none: None }
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
        assert_eq!(swap("Name=caret-up, Size=16", "Name=caret-down, Size=16"), vec!["↳ Chevron : Name: caret-up → caret-down"]);
        // (b) two keys differ, in the order of the first name.
        assert_eq!(swap("Name=a, Size=16", "Name=b, Size=24"), vec!["↳ Chevron : Name: a → b, Size: 16 → 24"]);
        // The order of the keys in the second name does not matter.
        assert_eq!(swap("Name=a, Size=16", "Size=24, Name=a"), vec!["↳ Chevron : Size: 16 → 24"]);
    }

    #[test]
    fn a_swap_that_is_not_a_clean_variant_change_says_both_names() {
        // (c) other keys, a key more, a name that is not a variant name.
        assert_eq!(swap("Name=a, Size=16", "Kind=a, Size=16"), vec!["↳ Chevron : Name=a, Size=16 → Kind=a, Size=16"]);
        assert_eq!(swap("Name=a", "Name=a, Size=16"), vec!["↳ Chevron : Name=a → Name=a, Size=16"]);
        assert_eq!(swap("Icon / Plus", "Icon / Minus"), vec!["↳ Chevron : Icon / Plus → Icon / Minus"]);
        assert_eq!(swap("Name=a", "Icon"), vec!["↳ Chevron : Name=a → Icon"]);
        // (d) a value with a comma is ambiguous: whole names. So is a repeated key, and names that differ only in order.
        assert_eq!(swap("Name=a,b, Size=16", "Name=c, Size=16"), vec!["↳ Chevron : Name=a,b, Size=16 → Name=c, Size=16"]);
        assert_eq!(swap("Name=a, Name=b", "Name=a, Name=c"), vec!["↳ Chevron : Name=a, Name=b → Name=a, Name=c"]);
        assert_eq!(swap("Name=a, Size=1", "Size=1, Name=a"), vec!["↳ Chevron : Name=a, Size=1 → Size=1, Name=a"]);
    }

    #[test]
    fn what_is_inside_a_swapped_instance_is_not_listed_but_its_own_changes_and_its_siblings_are() {
        // (e) the vector inside changes name ("up" gone, "down" new): no lines for it. (f) the instance keeps its own size change.
        let d = diff_of(&with_instance("Name=up", "up", 16.0), &with_instance("Name=down", "down", 24.0));
        assert_eq!(texts(&d), vec!["↳ Chevron : Name: up → down", "Width : 16 → 24"]);
        // A change outside the instance is still listed, in the tree's order.
        let mut b = with_instance("Name=down", "down", 16.0);
        b[3].0 = json!({"name": "Label", "type": "TEXT", "characters": "Bye"});
        assert_eq!(texts(&diff_of(&with_instance("Name=up", "up", 16.0), &b)), vec!["↳ Chevron : Name: up → down", "↳ Label", "Text : \"Hi\" → \"Bye\""]);
    }

    #[test]
    fn an_instance_of_the_same_component_keeps_the_old_behaviour() {
        // (g) same main component: the inner layers are listed as before (one hidden, one shown), no swap line.
        let a = with_instance("Name=up", "up", 16.0);
        let mut b = with_instance("Name=up", "down", 16.0);
        b[2].0 = json!({"name": "down", "type": "VECTOR", "visible": true, "absoluteBounds": {"x": 0.0, "y": 0.0, "width": 8.0, "height": 8.0}});
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Chevron / down : visible", "↳ Chevron / up : hidden"]);
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
        assert_eq!(texts(&d), vec!["Width : 100 → 120"]);
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
        assert_eq!(texts(&diff_of(&frame("up"), &frame("down"))), vec!["↳ Chevron / down : visible", "↳ Chevron / up : hidden"]);
        // (i) an instance that is hidden in one variant and shown in the other, or that only one has: visible / hidden.
        let mut hidden = with_instance("Name=up", "up", 16.0);
        hidden[1].0 = json!({"name": "Chevron", "type": "INSTANCE", "mainComponentName": "Name=up", "visible": false});
        assert_eq!(texts(&diff_of(&with_instance("Name=up", "up", 16.0), &hidden)), vec!["↳ Chevron : hidden"]);
        let without = vec![with_instance("Name=up", "up", 16.0).remove(0)];
        // (the sibling "Label" of the helper comes and goes with it)
        assert_eq!(texts(&diff_of(&without, &with_instance("Name=up", "up", 16.0))), vec!["↳ Chevron : visible", "↳ Label : visible"]);
        assert_eq!(texts(&diff_of(&with_instance("Name=up", "up", 16.0), &without)), vec!["↳ Chevron : hidden", "↳ Label : hidden"]);
    }

    #[test]
    fn radius_padding_and_typography_changes_are_listed() {
        let d = diff_of(&button(8.0, 16.0, "#3B5BDB", 14.0), &button(12.0, 20.0, "#3B5BDB", 16.0));
        assert_eq!(
            texts(&d),
            vec![
                "Radius : 8 → 12",
                "Horizontal padding : 16 → 20",
                "↳ Label / Text",
                "Typography : Work Sans 14/150% Medium → Work Sans 16/150% Medium",
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
            "Typography : text/label-m (Inter 14/20 Medium) → Inter 12/16 Medium"
        );
        assert_eq!(
            said(Lang::En, typo, side(Some("text/h3"), "Inter 24/32 SemiBold, letter spacing -1%"), side(None, "Inter 24/32 SemiBold, letter spacing -1%")),
            "Typography : text/h3 (Inter 24/32 SemiBold, letter spacing -1%) → Inter 24/32 SemiBold, letter spacing -1%"
        );
        // token -> token
        assert_eq!(
            said(Lang::En, typo, side(Some("text/label-m"), "Inter 14/20 Medium"), side(Some("text/label-l"), "Inter 16/24 Medium")),
            "Typography : text/label-m (Inter 14/20 Medium) → text/label-l (Inter 16/24 Medium)"
        );
        // value -> token, and the other labels
        assert_eq!(said(Lang::En, Label::Prop(Prop::Stroke), side(None, "none"), side(Some("Colors/color/border"), "#E3E7E8")), "Stroke : none → Colors/color/border (#E3E7E8)");
        assert_eq!(said(Lang::En, Label::Prop(Prop::Radius), side(Some("Radius/radius/default"), "10"), side(None, "8")), "Radius : Radius/radius/default (10) → 8");
        assert_eq!(said(Lang::En, Label::Prop(Prop::Fill), side(Some("Colors/color/primary"), "#C6005C"), side(None, "none")), "Fill : Colors/color/primary (#C6005C) → none");
        assert_eq!(said(Lang::En, Label::Prop(Prop::Gap), side(Some("Spacing/space/8"), "8"), side(None, "6")), "Gap : Spacing/space/8 (8) → 6");
        // Words stay translated.
        assert_eq!(
            said(Lang::Pt, typo, side(Some("text/h3"), "Inter 24/32 SemiBold, espaçamento entre letras -1%"), side(None, "Inter 24/32 SemiBold, espaçamento entre letras -1%")),
            "Tipografia : text/h3 (Inter 24/32 SemiBold, espaçamento entre letras -1%) → Inter 24/32 SemiBold, espaçamento entre letras -1%"
        );
        assert_eq!(said(Lang::Pt, Label::Prop(Prop::Fill), side(Some("Colors/color/primary"), "#C6005C"), side(None, "none")), "Preenchimento : Colors/color/primary (#C6005C) → nenhum");
    }

    #[test]
    fn a_line_with_no_token_is_what_it_was_and_effects_carry_their_description() {
        // value -> value
        assert_eq!(said(Lang::En, Label::Width, side(None, "100"), side(None, "120")), "Width : 100 → 120");
        assert_eq!(said(Lang::En, Label::Prop(Prop::Fill), side(None, "#3B5BDB"), side(None, "#2F4AB5")), "Fill : #3B5BDB → #2F4AB5");
        assert_eq!(said(Lang::En, Label::Content, side(None, "Hi"), side(None, "Bye")), "Text : \"Hi\" → \"Bye\"");
        // Effects: none on one side, a style on the other.
        assert_eq!(
            said(Lang::En, Label::Effects, side(None, "none"), side(Some("shadow/sm"), "Drop shadow 0 1 blur 2 #000000 at 5%")),
            "Effects : none → shadow/sm (Drop shadow 0 1 blur 2 #000000 at 5%)"
        );
        assert_eq!(
            said(Lang::Pt, Label::Effects, side(None, "none"), side(Some("shadow/sm"), "Sombra projetada 0 1 desfoque 2 #000000 a 5%")),
            "Efeitos : nenhum → shadow/sm (Sombra projetada 0 1 desfoque 2 #000000 a 5%)"
        );
    }

    #[test]
    fn a_token_with_an_empty_or_dash_value_or_its_own_text_is_shown_alone() {
        let fill = Label::Prop(Prop::Fill);
        assert_eq!(said(Lang::En, fill, side(Some("Colors/a"), ""), side(None, "#FFF")), "Fill : Colors/a → #FFF");
        assert_eq!(said(Lang::En, fill, side(Some("Colors/a"), "  "), side(Some("Colors/b"), "—")), "Fill : Colors/a → Colors/b");
        assert_eq!(said(Lang::En, fill, side(Some("Colors/a"), "Colors/a"), side(None, "#FFF")), "Fill : Colors/a → #FFF");
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
        assert_eq!(texts(&d), vec!["Fill : color/primary/500 (#3B5BDB) → color/primary/700 (#2F4AB5)"]);
    }

    #[test]
    fn a_layer_that_is_switched_off_or_on_is_one_line() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let mut b = a.clone();
        b[1].0["visible"] = json!(false);
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Icon : hidden"]);
        assert_eq!(texts(&diff_of(&b, &a)), vec!["↳ Icon : visible"]);
    }

    #[test]
    fn what_is_under_a_hidden_layer_is_not_compared() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let mut b = button(8.0, 16.0, "#3B5BDB", 18.0);
        b[2].0["visible"] = json!(false);
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Label : hidden"]);
    }

    #[test]
    fn under_half_a_pixel_is_no_difference() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let b = button(8.3, 16.4, "#3B5BDB", 14.0);
        assert!(diff_of(&a, &b).changes.is_empty());
        let c = button(8.6, 16.0, "#3B5BDB", 14.0);
        assert_eq!(texts(&diff_of(&a, &c)), vec!["Radius : 8 → 8.6"]);
        assert_eq!(texts(&diff_of(&a, &a)), vec!["No differences from default"]);
    }

    #[test]
    fn effects_are_read_and_an_unreadable_one_is_said() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        let mut b = a.clone();
        b[0].0["effects"] = json!([{"type": "dropShadow", "offset": [0.0, 8.0], "radius": 8.0, "spread": 0.0, "visible": true, "color": {"r": 0.231, "g": 0.357, "b": 0.859, "a": 0.3}}]);
        assert_eq!(texts(&diff_of(&a, &b)), vec!["Effects : none → Drop shadow 0 8 blur 8 #3B5BDB at 30%"]);
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
        assert_eq!(say(Lang::En), vec!["Effects : none → Drop shadow 0 8 blur 8 spread 2 #3B5BDB at 30%; Inner shadow 1 1 blur 4 #000000; Layer blur 6; Background blur 12"]);
        assert_eq!(say(Lang::Pt), vec!["Efeitos : nenhum → Sombra projetada 0 8 desfoque 8 expansão 2 #3B5BDB a 30%; Sombra interna 1 1 desfoque 4 #000000; Desfoque de camada 6; Desfoque de fundo 12"]);
        // Opacity of a fill: "… at 50%" / "… a 50%".
        let mut c = a.clone();
        c[0].0["fills"] = json!([{"type": "solid", "color": "#3B5BDB", "opacity": 0.5}]);
        let fill = |lang: Lang| list(lang, &compare(&snapshot(lang, &a, &Vars::default(), &Styles::default()), &snapshot(lang, &c, &Vars::default(), &Styles::default()))).texts(lang);
        assert_eq!(fill(Lang::En), vec!["Fill : #3B5BDB → #3B5BDB at 50%"]);
        assert_eq!(fill(Lang::Pt), vec!["Preenchimento : #3B5BDB → #3B5BDB a 50%"]);
    }

    #[test]
    fn an_effect_the_dictionary_does_not_know_is_shown_as_the_host_names_it() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        for kind in ["noise", "texture", "glass"] {
            let mut b = a.clone();
            b[0].0["effects"] = json!([{"type": kind, "visible": true}]);
            for lang in [Lang::En, Lang::Pt] {
                let said = list(lang, &compare(&snapshot(lang, &a, &Vars::default(), &Styles::default()), &snapshot(lang, &b, &Vars::default(), &Styles::default()))).texts(lang);
                assert_eq!(said, vec![format!("{} : {} → {kind}", t(lang, Key::DeltaEffects), t(lang, Key::ValueNone))], "{kind} {lang:?}");
            }
        }
    }

    #[test]
    fn a_list_stops_at_twelve_lines_and_counts_the_rest() {
        let a = button(8.0, 16.0, "#3B5BDB", 14.0);
        // Twenty layers whose text changes: a heading and a line each.
        let mut base = a.clone();
        let mut var = a.clone();
        for i in 0..20 {
            let n = json!({"name": format!("Row {i}"), "type": "TEXT", "characters": "a", "fontFamily": "W", "fontSize": 14.0, "fontStyle": "R", "lineHeight": "150%", "letterSpacing": "0%"});
            let mut m = n.clone();
            m["characters"] = json!("b");
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
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Cell[1]", "Typography : W 14/150% R → W 16/150% R"]);
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
        assert_eq!(texts(&diff_of(&a, &b)), vec!["↳ Box", "Width : 100 → 140"]);
    }
}
