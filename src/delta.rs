//! What a variant changes from the default variant: the layers of the two are matched by their path
//! of names (with an index when siblings share a name) and compared property by property.
//! Pure logic (no host calls), so it can be tested on the host.
//!
//! A layer's entries are the audit's rows (radius, padding, gap, fill, stroke, typography, with their
//! token or style) plus what the audit leaves out: size, stroke weight, opacity, effects, resizing
//! and a text layer's content. Numbers that differ by less than half a pixel are the same.

use crate::audit::{self, Prop, State, Styles, Vars, number};
use crate::i18n::{Key, Lang, t, tf};
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
}

fn float(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|f| f.is_finite())
}

fn describe_effect(e: &Value) -> String {
    let kind = e.get("type").and_then(Value::as_str).unwrap_or("effect");
    let n = |k: &str| float(e, k).map(number);
    let name = match kind {
        "dropShadow" => "Drop shadow",
        "innerShadow" => "Inner shadow",
        "layerBlur" => "Layer blur",
        "backgroundBlur" => "Background blur",
        other => other,
    };
    let mut out = name.to_string();
    if let Some([x, y]) = e.get("offset").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect::<Vec<_>>()).as_deref() {
        out.push_str(&format!(" {} {}", number(*x), number(*y)));
    }
    if let Some(r) = n("radius") {
        out.push_str(&format!(" blur {r}"));
    }
    if let Some(s) = float(e, "spread").filter(|s| s.abs() > 0.005) {
        out.push_str(&format!(" spread {}", number(s)));
    }
    if let Some(c) = e.get("color").filter(|c| c.is_object()) {
        let ch = |k: &str| float(c, k).unwrap_or(0.0);
        let alpha = float(c, "a").unwrap_or(1.0);
        out.push_str(&format!(" {}", audit::hex(ch("r"), ch("g"), ch("b"), 1.0)));
        if alpha < 0.998 {
            out.push_str(&format!(" at {}%", number((alpha * 100.0).round())));
        }
    }
    out
}

fn percent(opacity: f64) -> f64 {
    (opacity * 100.0 * 10.0).round() / 10.0
}

/// The entries of one layer, as `node.get` gives it (with the audit's fields and `effects`, `strokeWeight`).
pub fn entries(node: &Value, root: bool, vars: &Vars, styles: &Styles) -> Vec<Entry> {
    let mut out = Vec::new();
    for r in audit::rows_for(node, vars, styles) {
        let token = matches!(r.state, State::Token | State::Check | State::Style).then(|| r.token.clone());
        let mut value = r.value.clone();
        if matches!(r.property, Prop::Fill | Prop::Stroke) {
            let opacity = audit::first_paint_opacity(node, if r.property == Prop::Fill { "fills" } else { "strokes" });
            if opacity < 0.995 {
                value = format!("{value} at {}%", number(percent(opacity)));
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
        let value = if shown.is_empty() { "none".to_string() } else { shown.iter().map(|e| describe_effect(e)).collect::<Vec<_>>().join("; ") };
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
pub fn snapshot(walked: &[(Value, usize)], vars: &Vars, styles: &Styles) -> Vec<Snap> {
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
            entries: entries(n, i == 0, vars, styles),
            effects_raw: n.get("effects").map(ToString::to_string).unwrap_or_default(),
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

/// What `variant` changes from `base`: the layers matched by path, the root by position.
pub fn compare(base: &[Snap], variant: &[Snap]) -> Diff {
    let mut diff = Diff::default();
    let by_path: HashMap<&str, usize> = base.iter().enumerate().skip(1).map(|(i, s)| (s.path.as_str(), i)).collect();
    let (base_drawn, var_drawn) = (drawn(base), drawn(variant));
    let mut matched = vec![false; base.len()];
    let mut mapped = vec![false; variant.len()];
    for (j, v) in variant.iter().enumerate() {
        let i = if j == 0 { (!base.is_empty()).then_some(0) } else { by_path.get(v.path.as_str()).copied() };
        let layer = (j > 0).then(|| v.path.clone());
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
        let gone = !matched.get(i).copied().unwrap_or(false);
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
            out.push(tf(lang, Key::DeltaMore, &[("n", &self.more.to_string())]));
        }
        out.extend(self.none.clone());
        out
    }
}

fn show_side(s: &Side, label: Label, lang: Lang) -> String {
    if let Some(token) = &s.token {
        return token.clone();
    }
    match label {
        Label::HSizing | Label::VSizing => sizing_word(lang, &s.value),
        _ => s.value.clone(),
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

/// "Label : from → to", with the values in parentheses after the names of a token.
fn change_text(lang: Lang, label: Label, from: &Side, to: &Side) -> String {
    let (a, b) = if label == Label::Content {
        let (a, b) = windows(&from.value, &to.value);
        (format!("\"{a}\""), format!("\"{b}\""))
    } else {
        (show_side(from, label, lang), show_side(to, label, lang))
    };
    let mut out = format!("{} : {a} → {b}", t(lang, label.key()));
    if from.token.is_some() || to.token.is_some() {
        let (va, vb) = (show_side(&Side { token: None, ..from.clone() }, label, lang), show_side(&Side { token: None, ..to.clone() }, label, lang));
        out.push_str(&format!(" ({va} → {vb})"));
    }
    out
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
    // A layer with a single switch of visibility is one line; any other gets a heading (the root none).
    let headed = |layer: Option<&String>| {
        let mut own = diff.changes.iter().filter(|c| c.layer.as_ref() == layer);
        layer.is_some() && !(own.next().is_some_and(|c| matches!(c.kind, Kind::Shown(_))) && own.next().is_none())
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
        compare(&snapshot(a, &vars, &styles), &snapshot(b, &vars, &styles))
    }

    fn texts(d: &Diff) -> Vec<String> {
        list(Lang::En, d).texts(Lang::En)
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
        let d = compare(&snapshot(&a, &vars, &styles), &snapshot(&b, &vars, &styles));
        assert_eq!(texts(&d), vec!["Fill : color/primary/500 → color/primary/700 (#3B5BDB → #2F4AB5)"]);
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
        let snaps = snapshot(&a, &Vars::default(), &Styles::default());
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
