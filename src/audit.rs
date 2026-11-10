//! The token audit: what a layer's properties are bound to, checked against `variable.list`.
//! Pure logic (no host calls), so it can be tested on the host.
//!
//! A property with a binding is in one of these states:
//! - `Style`: a style is applied (a colour style, a text style) and it exists in the file (for a colour: with the layer's value);
//! - `Token`: the variable exists, its type fits the property and its resolved value is the layer's;
//! - `Suspect`: it exists but its type doesn't fit or its value differs (a likely ID collision);
//! - `Missing`: no variable has that ID in the file;
//! - `Unbound`: no binding at all (the raw value).

use figcraft_plugin_sdk::Value;
use std::collections::HashMap;

/// Most alias hops followed before giving up (a guard against loops).
const MAX_HOPS: usize = 16;

/// A variable of `variable.list`, with its value in the collection's default mode.
#[derive(Clone, Debug)]
pub struct Var {
    pub name: String,
    pub collection: String,
    /// `number`, `color`, `string` or `boolean`.
    pub kind: String,
    /// `{type, value}` as the list gives it (`type` may be `alias`).
    pub value: Value,
}

impl Var {
    /// `collection/name`.
    pub fn path(&self) -> String {
        format!("{}/{}", self.collection, self.name)
    }
}

/// Every variable of the file, by id.
#[derive(Debug, Default)]
pub struct Vars {
    pub by_id: HashMap<String, Var>,
}

/// What a variable comes to once its aliases are followed.
#[derive(Clone, Debug, PartialEq)]
pub enum Resolved {
    Number(f64),
    Color(String),
    Other(String),
}

impl Resolved {
    pub fn show(&self) -> String {
        match self {
            Resolved::Number(n) => number(*n),
            Resolved::Color(c) => c.clone(),
            Resolved::Other(t) => t.clone(),
        }
    }
}

/// A property's value on the layer.
#[derive(Clone, Debug, PartialEq)]
pub enum Real {
    Num(f64),
    /// Several numbers that differ (a corner radius per corner, left and right padding).
    Nums(Vec<f64>),
    Color(String),
    /// Anything else (a gradient's kind, say).
    Text(String),
}

impl Real {
    pub fn show(&self) -> String {
        match self {
            Real::Num(n) => number(*n),
            Real::Nums(v) => v.iter().map(|n| number(*n)).collect::<Vec<_>>().join(" / "),
            Real::Color(c) | Real::Text(c) => c.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Token,
    /// A style applied and found in `style.list` (for a colour: its value is the layer's).
    Style,
    /// A numeric token on a layer of a tree that has suspect bindings: it may be a collision.
    Check,
    Suspect,
    Missing,
    Unbound,
}

/// A property the audit looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Prop {
    Radius,
    Gap,
    PadH,
    PadV,
    Fill,
    Stroke,
    /// A text layer's font, size, line height and weight (with letter spacing, case and decoration when set).
    Typography,
}

impl Prop {
    /// Whether it takes a number (the others take a colour or a text).
    pub fn numeric(self) -> bool {
        !matches!(self, Prop::Fill | Prop::Stroke | Prop::Typography)
    }
}

/// One row of the audit: a property of a layer.
#[derive(Clone, Debug)]
pub struct Row {
    pub property: Prop,
    /// `collection/name` (the alias chain for a token), or what to say instead.
    pub token: String,
    pub value: String,
    pub state: State,
    /// Whether the value is zero (a row with no binding and a zero value is left out).
    pub zero: bool,
    /// Where the layer is in the walk, and how deep (0: the root).
    pub node: usize,
    pub depth: usize,
    /// Whether the state (and the token) come from a style rather than a variable.
    pub style: bool,
}

/// A number without a useless fraction.
pub fn number(n: f64) -> String {
    if (n - n.round()).abs() < 0.005 {
        format!("{}", n.round() as i64)
    } else {
        let s = format!("{n:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// `#RRGGBB` (or `#RRGGBBAA` when not opaque) from channels of 0 to 1, rounded.
pub fn hex(r: f64, g: f64, b: f64, a: f64) -> String {
    let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a < 0.998 {
        format!("#{:02X}{:02X}{:02X}{:02X}", c(r), c(g), c(b), c(a))
    } else {
        format!("#{:02X}{:02X}{:02X}", c(r), c(g), c(b))
    }
}

/// A layer's hex colour in a form to compare: upper case, an opaque alpha dropped.
fn normal_hex(h: &str) -> String {
    let up = h.trim().to_uppercase();
    match up.get(..7) {
        Some(head) if up.len() == 9 && up.ends_with("FF") => head.to_string(),
        _ => up,
    }
}

impl Vars {
    /// Read the reply of `variable.list`.
    pub fn parse(list: &Value) -> Vars {
        // Collection id -> (name, id of its first mode).
        let mut collections: HashMap<&str, (&str, &str)> = HashMap::new();
        if let Some(cs) = list.get("collections").and_then(Value::as_array) {
            for c in cs {
                let id = c.get("id").and_then(Value::as_str).unwrap_or_default();
                let name = c.get("name").and_then(Value::as_str).unwrap_or_default();
                let mode = c.get("modes").and_then(Value::as_array).and_then(|m| m.first()).and_then(|m| m.get("id")).and_then(Value::as_str).unwrap_or_default();
                collections.insert(id, (name, mode));
            }
        }
        let mut by_id = HashMap::with_capacity(list.get("variables").and_then(Value::as_array).map_or(0, Vec::len));
        if let Some(vs) = list.get("variables").and_then(Value::as_array) {
            for v in vs {
                let Some(id) = v.get("id").and_then(Value::as_str) else { continue };
                let coll = v.get("collection").and_then(Value::as_str).unwrap_or_default();
                let (cname, mode) = collections.get(coll).copied().unwrap_or(("?", ""));
                let values = v.get("values").and_then(Value::as_object);
                let value = values.and_then(|m| m.get(mode).or_else(|| m.values().next())).cloned().unwrap_or(Value::Null);
                by_id.insert(
                    id.to_string(),
                    Var {
                        name: v.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
                        collection: cname.to_string(),
                        kind: v.get("resolvedType").and_then(Value::as_str).unwrap_or_default().to_string(),
                        value,
                    },
                );
            }
        }
        Vars { by_id }
    }

    /// Follow `id` through its aliases: the paths of the variables met (the first is `id`'s own)
    /// and the value it comes to.
    pub fn resolve(&self, id: &str) -> Option<(Vec<String>, Resolved)> {
        let mut chain = Vec::new();
        let mut at = id.to_string();
        for _ in 0..MAX_HOPS {
            let var = self.by_id.get(&at)?;
            chain.push(var.path());
            let kind = var.value.get("type").and_then(Value::as_str).unwrap_or_default();
            let value = var.value.get("value");
            match (kind, value) {
                ("alias", Some(Value::String(next))) => at = next.clone(),
                ("number", Some(v)) => return Some((chain, Resolved::Number(v.as_f64()?))),
                ("color", Some(v)) => {
                    let ch = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                    return Some((chain, Resolved::Color(hex(ch("r"), ch("g"), ch("b"), v.get("a").and_then(Value::as_f64).unwrap_or(1.0)))));
                }
                (_, v) => return Some((chain, Resolved::Other(v.map(ToString::to_string).unwrap_or_default()))),
            }
        }
        None
    }
}

/// A style of `style.list`, as far as the audit needs it.
#[derive(Clone, Debug)]
pub struct StyleDef {
    pub name: String,
    /// A paint style's first paint that is drawn: its solid colour and its opacity (the colour's alpha included).
    pub paint: Option<(String, f64)>,
}

/// Every style of the file, by id (one `style.list`).
#[derive(Debug, Default)]
pub struct Styles {
    pub by_id: HashMap<String, StyleDef>,
}

impl Styles {
    /// Read the reply of `style.list`.
    pub fn parse(list: &Value) -> Styles {
        let mut by_id = HashMap::new();
        for s in list.as_array().map(Vec::as_slice).unwrap_or_default() {
            let Some(id) = s.get("id").and_then(Value::as_str) else { continue };
            let paint = s.get("paints").and_then(Value::as_array).and_then(|a| a.iter().find(|p| paint_shown(p))).and_then(|p| {
                if p.get("type").and_then(Value::as_str) != Some("solid") {
                    return None;
                }
                let c = p.get("color")?;
                let ch = |k: &str| c.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                let alpha = c.get("a").and_then(Value::as_f64).unwrap_or(1.0);
                Some((hex(ch("r"), ch("g"), ch("b"), 1.0), alpha * p.get("opacity").and_then(Value::as_f64).unwrap_or(1.0)))
            });
            by_id.insert(
                id.to_string(),
                StyleDef { name: s.get("name").and_then(Value::as_str).unwrap_or_default().to_string(), paint },
            );
        }
        Styles { by_id }
    }
}

/// The kind of variable a property takes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Want {
    Number,
    Color,
}

impl Want {
    fn name(self) -> &'static str {
        match self {
            Want::Number => "number",
            Want::Color => "color",
        }
    }
}

/// Whether the layer's value is what the variable resolves to.
fn same(real: &Real, resolved: &Resolved) -> bool {
    match (real, resolved) {
        (Real::Num(a), Resolved::Number(b)) => (a - b).abs() < 0.01,
        (Real::Nums(v), Resolved::Number(b)) => v.iter().all(|a| (a - b).abs() < 0.01),
        (Real::Color(a), Resolved::Color(b)) => normal_hex(a) == normal_hex(b),
        _ => false,
    }
}

/// Classify one property: its state and what to show in the token column.
fn classify(real: &Real, want: Want, bound: Option<&str>, vars: &Vars) -> (State, String) {
    let Some(id) = bound else { return (State::Unbound, "—".into()) };
    let Some(var) = vars.by_id.get(id) else { return (State::Missing, id.to_string()) };
    let Some((chain, resolved)) = vars.resolve(id) else {
        return (State::Suspect, format!("{} (alias loop or broken)", var.path()));
    };
    if var.kind == want.name() && same(real, &resolved) {
        (State::Token, chain.join(" → "))
    } else {
        (State::Suspect, format!("{} ({}: {})", var.path(), var.kind, resolved.show()))
    }
}

/// A node's numbers: `a` if the two agree, else both.
fn pair(a: f64, b: f64) -> Real {
    if (a - b).abs() < 0.005 { Real::Num(a) } else { Real::Nums(vec![a, b]) }
}

/// Whether a paint is drawn: not switched off (`visible: false`) and not fully transparent.
fn paint_shown(paint: &Value) -> bool {
    paint.get("visible").and_then(Value::as_bool).unwrap_or(true) && paint.get("opacity").and_then(Value::as_f64).is_none_or(|o| o > 0.0)
}

/// How many of the node's fills and strokes lead with a paint that isn't drawn (and so get no row).
pub fn hidden_paints(node: &Value) -> usize {
    ["fills", "strokes"].iter().filter(|k| node.get(**k).and_then(Value::as_array).and_then(|a| a.first()).is_some_and(|p| !paint_shown(p))).count()
}

/// The first paint of `node[key]` that is drawn, as a layer value, if there is one.
fn first_paint(node: &Value, key: &str) -> Option<Real> {
    let paint = node.get(key).and_then(Value::as_array).and_then(|a| a.iter().find(|p| paint_shown(p)))?;
    match (paint.get("type").and_then(Value::as_str), paint.get("color").and_then(Value::as_str)) {
        (Some("solid"), Some(c)) => Some(Real::Color(normal_hex(c))),
        (Some(t), _) => Some(Real::Text(t.to_string())),
        _ => None,
    }
}

/// The opacity of the first paint of `node[key]` that is drawn (1 when it says nothing).
pub fn first_paint_opacity(node: &Value, key: &str) -> f64 {
    node.get(key).and_then(Value::as_array).and_then(|a| a.iter().find(|p| paint_shown(p))).and_then(|p| p.get("opacity").and_then(Value::as_f64)).unwrap_or(1.0)
}

/// A colour style on a property: found with the layer's colour (`Style`), found with another one
/// (`Suspect`), or not in the file (`Missing`, the token is the style's id).
fn classify_style(real: &Real, opacity: f64, id: &str, styles: &Styles) -> (State, String) {
    let Some(def) = styles.by_id.get(id) else { return (State::Missing, id.to_string()) };
    match (&def.paint, real) {
        (Some((color, alpha)), Real::Color(c)) if normal_hex(c) != *color || (opacity - alpha).abs() > 0.01 => {
            let shown = if (alpha - 1.0).abs() > 0.01 { format!("{color} at {}%", number((alpha * 100.0).round())) } else { color.clone() };
            (State::Suspect, format!("{} (style: {shown})", def.name))
        }
        _ => (State::Style, def.name.clone()),
    }
}

/// The text style of a text layer: the one the whole layer has, or the only one its ranges use.
fn text_style_id(node: &Value) -> Option<String> {
    let whole = node.get("styles").and_then(|s| s.get("text")).and_then(Value::as_str);
    if let Some(id) = whole {
        return Some(id.to_string());
    }
    let ranges = node.get("textStyleRanges").and_then(Value::as_array)?;
    let mut ids = ranges.iter().filter_map(|r| r.get(2).and_then(Value::as_str));
    let first = ids.next()?;
    (ranges.len() == 1 || ids.all(|i| i == first)).then(|| first.to_string())
}

/// A spacing such as `"2%"`, `"1.5"` or `0`, as a number (the percentage or the pixels) and its text.
fn spacing_value(v: &Value) -> Option<(f64, String)> {
    match v {
        Value::Number(n) => n.as_f64().map(|n| (n, format!("{}px", number(n)))),
        Value::String(t) => {
            let n: f64 = t.trim().trim_end_matches('%').trim_end_matches("px").trim().parse().ok()?;
            Some((n, if t.contains('%') { format!("{}%", number(n)) } else { format!("{}px", number(n)) }))
        }
        _ => None,
    }
}

/// "Work Sans 14/150% Medium", then ", letter spacing 2%" and ", underline" when they are set (the host's `node.get` does not report a layer's text case).
fn typography_value(node: &Value) -> Option<String> {
    let family = node.get("fontFamily").and_then(Value::as_str)?;
    let size = node.get("fontSize").and_then(Value::as_f64).map(number)?;
    let line = match node.get("lineHeight") {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Number(n)) => n.as_f64().map(number).unwrap_or_default(),
        _ => "auto".to_string(),
    };
    let weight = node.get("fontStyle").and_then(Value::as_str).unwrap_or_default();
    let mut out = format!("{family} {size}/{line} {weight}").trim_end().to_string();
    if let Some((n, text)) = node.get("letterSpacing").and_then(spacing_value)
        && n.abs() > 0.005
    {
        out.push_str(&format!(", letter spacing {text}"));
    }
    if let Some(d) = node.get("textDecoration").and_then(Value::as_str).filter(|d| !matches!(*d, "none" | "")) {
        out.push_str(&format!(", {}", d.to_lowercase()));
    }
    Some(out)
}

/// The audit rows of one layer (`node` as `node.get` gives it with `boundVariables`,
/// `cornerRadius`, `layout`, `fills` and `strokes`).
pub fn rows_for(node: &Value, vars: &Vars, styles: &Styles) -> Vec<Row> {
    let bound = node.get("boundVariables");
    let bound_id = |key: &str| bound.and_then(|b| b.get(key)).and_then(Value::as_str);
    let mut rows = Vec::new();
    let style_id = |key: &str| node.get("styles").and_then(|s| s.get(key)).and_then(Value::as_str);
    let mut add = |property: Prop, key: &str, real: Real, want: Want| {
        let (mut state, mut token) = classify(&real, want, bound_id(key), vars);
        let mut from_style = false;
        // A colour style on a fill or a stroke: with no variable it is the token; with a variable, a note.
        if let (Some(sid), Want::Color) = (style_id(key), want) {
            let opacity = first_paint_opacity(node, if key == "fill" { "fills" } else { "strokes" });
            let (st, name) = classify_style(&real, opacity, sid, styles);
            if state == State::Unbound {
                (state, token, from_style) = (st, name, true);
            } else if st != State::Missing {
                // A variable and a style at once: the variable is the token, the style a note.
                let style_name = styles.by_id.get(sid).map_or(name, |d| d.name.clone());
                token = format!("{token} · style {style_name}");
            }
        }
        let zero = match &real {
            Real::Num(n) => n.abs() < 0.005,
            Real::Nums(v) => v.iter().all(|n| n.abs() < 0.005),
            _ => false,
        };
        rows.push(Row { property, token, value: real.show(), state, zero, node: 0, depth: 0, style: from_style });
    };

    // Corner radius: a row if bound or not zero.
    let radius = match node.get("cornerRadius") {
        Some(Value::Number(n)) => n.as_f64().map(Real::Num),
        Some(Value::Array(a)) => {
            let v: Vec<f64> = a.iter().filter_map(Value::as_f64).collect();
            match v.first() {
                Some(first) if v.iter().all(|x| (x - first).abs() < 0.005) => Some(Real::Num(*first)),
                Some(_) => Some(Real::Nums(v)),
                None => None,
            }
        }
        _ => None,
    };
    if let Some(r) = radius {
        let zero = match &r {
            Real::Num(n) => n.abs() < 0.005,
            Real::Nums(v) => v.iter().all(|n| n.abs() < 0.005),
            _ => false,
        };
        if !zero || bound_id("cornerRadius").is_some() {
            add(Prop::Radius, "cornerRadius", r, Want::Number);
        }
    }

    // Auto layout: gap and the two paddings.
    if let Some(layout) = node.get("layout").filter(|l| matches!(l.get("mode").and_then(Value::as_str), Some("horizontal" | "vertical" | "grid"))) {
        if let Some(g) = layout.get("gap").and_then(Value::as_f64) {
            add(Prop::Gap, "gap", Real::Num(g), Want::Number);
        }
        let pad: Vec<f64> = layout.get("padding").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
        if let [top, right, bottom, left] = pad[..] {
            add(Prop::PadH, "paddingHorizontal", pair(left, right), Want::Number);
            add(Prop::PadV, "paddingVertical", pair(top, bottom), Want::Number);
        }
    }

    // First fill and first stroke.
    if let Some(r) = first_paint(node, "fills") {
        add(Prop::Fill, "fill", r, Want::Color);
    }
    if let Some(r) = first_paint(node, "strokes") {
        add(Prop::Stroke, "stroke", r, Want::Color);
    }

    // A text layer: its typography, and the text style when there is one.
    if node.get("type").and_then(Value::as_str) == Some("TEXT")
        && let Some(value) = typography_value(node)
    {
        let (state, token, style) = match text_style_id(node) {
            Some(id) => match styles.by_id.get(&id) {
                Some(def) => (State::Style, def.name.clone(), true),
                None => (State::Missing, id, true),
            },
            None => (State::Unbound, "—".to_string(), false),
        };
        rows.push(Row { property: Prop::Typography, token, value, state, zero: false, node: 0, depth: 0, style });
    }
    rows
}

/// Leave out the rows with no binding and a zero value (a radius, gap or padding that isn't set).
pub fn drop_zero_unbound(rows: Vec<Row>) -> Vec<Row> {
    rows.into_iter().filter(|r| !(r.state == State::Unbound && r.zero)).collect()
}

/// Only the rows of layers that are drawn (`shown[node]`): the summary counts what the legend shows.
pub fn keep_shown(rows: Vec<Row>, shown: &[bool]) -> Vec<Row> {
    rows.into_iter().filter(|r| shown.get(r.node).copied().unwrap_or(false)).collect()
}

/// How many variable bindings have a type that doesn't fit or a value that differs.
pub fn suspects(rows: &[Row]) -> usize {
    rows.iter().filter(|r| r.state == State::Suspect && !r.style).count()
}

/// With suspect bindings in the tree, a numeric token may be a collision that the file's own
/// values hide: it turns into `Check`.
pub fn mark_checks(rows: &mut [Row]) {
    if suspects(rows) == 0 {
        return;
    }
    for r in rows.iter_mut().filter(|r| r.property.numeric() && r.state == State::Token) {
        r.state = State::Check;
    }
}

/// Counts for the summary line: (bound, unbound, missing, suspect). `Check` and `Style` count as bound.
pub fn counts(rows: &[Row]) -> (usize, usize, usize, usize) {
    let n = |s: State| rows.iter().filter(|r| r.state == s).count();
    (n(State::Token) + n(State::Check) + n(State::Style), n(State::Unbound), n(State::Missing), n(State::Suspect))
}

/// How many of the bound properties are bound to a variable and how many to a style.
pub fn bound_split(rows: &[Row]) -> (usize, usize) {
    let styles = rows.iter().filter(|r| r.state == State::Style).count();
    let vars = rows.iter().filter(|r| matches!(r.state, State::Token | State::Check)).count();
    (vars, styles)
}

/// One property of a layer, as the legend shows it. A line stands for the layers of its group.
#[derive(Clone, Debug)]
pub struct Line {
    pub property: Prop,
    pub token: String,
    pub value: String,
    pub state: State,
    /// The place in the walk of every layer it stands for (those of its group).
    pub nodes: Vec<usize>,
}

/// What the legend needs to know about a layer of the walk besides its rows.
#[derive(Clone, Debug, Default)]
pub struct Info {
    pub name: String,
    /// The node's type (`FRAME`, `TEXT`, `VECTOR`…).
    pub kind: String,
    /// The line under the title of a part ("Instance of Icon", "Frame"); none for a deeper layer.
    pub sub: Option<String>,
    /// A part (the root and its visible direct children) gets a block even with no row.
    pub part: bool,
}

/// A block of the legend: layers that are alike all the way down.
#[derive(Clone, Debug)]
pub struct Group {
    pub name: String,
    pub kind: String,
    pub sub: Option<String>,
    /// Whether the layer is a part (it gets a block even with no row).
    pub part: bool,
    /// The first layer's place in the walk.
    pub node: usize,
    /// Every layer of the group, in walk order.
    pub nodes: Vec<usize>,
    /// The lines of the first layer (they are the same on all).
    pub lines: Vec<Line>,
}

type Signature = Vec<(Prop, String, String, State)>;

/// Group the layers by the whole block: a layer joins another only when name, type, "Instance of"
/// line and *every* row (property, token, value, state) are equal. Layers with no row appear only
/// when they are parts. Groups come in order of their first layer.
pub fn group_layers(rows: &[Row], infos: &[Info]) -> Vec<Group> {
    let mut by_node: Vec<Signature> = vec![Vec::new(); infos.len()];
    let mut first_rows: Vec<Vec<&Row>> = vec![Vec::new(); infos.len()];
    for r in rows {
        if let (Some(sig), Some(fr)) = (by_node.get_mut(r.node), first_rows.get_mut(r.node)) {
            sig.push((r.property, r.token.clone(), r.value.clone(), r.state));
            fr.push(r);
        }
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut index: HashMap<(String, String, Option<String>, Signature), usize> = HashMap::new();
    for (i, info) in infos.iter().enumerate() {
        let sig = by_node.get(i).cloned().unwrap_or_default();
        if sig.is_empty() && !info.part {
            continue;
        }
        let key = (info.name.clone(), info.kind.clone(), info.sub.clone(), sig);
        match index.get(&key) {
            Some(&g) => {
                if let Some(group) = groups.get_mut(g) {
                    group.nodes.push(i);
                    for l in &mut group.lines {
                        l.nodes.push(i);
                    }
                }
            }
            None => {
                index.insert(key, groups.len());
                let lines = first_rows.get(i).map(|rs| rs.iter().map(|r| Line { property: r.property, token: r.token.clone(), value: r.value.clone(), state: r.state, nodes: vec![i] }).collect()).unwrap_or_default();
                groups.push(Group { name: info.name.clone(), kind: info.kind.clone(), sub: info.sub.clone(), part: info.part, node: i, nodes: vec![i], lines });
            }
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use figcraft_plugin_sdk::json;

    fn vars() -> Vars {
        Vars::parse(&json!({
            "collections": [
                {"id": "C1", "name": "Tokens", "modes": [{"id": "C1/m1", "name": "A"}]},
                {"id": "C2", "name": "Semantico", "modes": [{"id": "C2/m1", "name": "Claro"}, {"id": "C2/m2", "name": "Escuro"}]}
            ],
            "variables": [
                {"id": "V1", "collection": "C1", "name": "spacing/8", "resolvedType": "number", "values": {"C1/m1": {"type": "number", "value": 8.0}}},
                {"id": "V2", "collection": "C1", "name": "color/fill/primary", "resolvedType": "color",
                 "values": {"C1/m1": {"type": "color", "value": {"a": 1.0, "b": 0.8588235378265381, "g": 0.35686275362968445, "r": 0.23137255012989044}}}},
                {"id": "V3", "collection": "C2", "name": "color/action/bg", "resolvedType": "color",
                 "values": {"C2/m1": {"type": "alias", "value": "V2"}, "C2/m2": {"type": "color", "value": {"a": 1.0, "b": 0.0, "g": 0.0, "r": 0.0}}}}
            ]
        }))
    }

    #[test]
    fn token_alias_chain_and_rounded_hex() {
        let (state, text) = classify(&Real::Color("#3B5BDB".into()), Want::Color, Some("V3"), &vars());
        assert_eq!(state, State::Token);
        assert_eq!(text, "Semantico/color/action/bg → Tokens/color/fill/primary");
    }

    #[test]
    fn suspect_on_type_or_value() {
        let v = vars();
        assert_eq!(classify(&Real::Color("#FFFFFF".into()), Want::Color, Some("V1"), &v).0, State::Suspect);
        assert_eq!(classify(&Real::Num(12.0), Want::Number, Some("V1"), &v).0, State::Suspect);
        assert_eq!(classify(&Real::Num(8.0), Want::Number, Some("V1"), &v).0, State::Token);
    }

    #[test]
    fn missing_and_unbound() {
        let v = vars();
        assert_eq!(classify(&Real::Num(8.0), Want::Number, Some("V99"), &v).0, State::Missing);
        assert_eq!(classify(&Real::Num(8.0), Want::Number, None, &v).0, State::Unbound);
    }

    #[test]
    fn rows_follow_what_the_layer_has() {
        let node = json!({"name": "Botão", "cornerRadius": 0.0, "fills": [], "strokes": [{"type": "solid", "color": "#111111"}],
            "layout": {"mode": "horizontal", "gap": 4.0, "padding": [8.0, 16.0, 8.0, 16.0]},
            "boundVariables": {"gap": "V1"}});
        let rows = rows_for(&node, &vars(), &Styles::default());
        let props: Vec<Prop> = rows.iter().map(|r| r.property).collect();
        assert_eq!(props, [Prop::Gap, Prop::PadH, Prop::PadV, Prop::Stroke]);
        assert_eq!(rows[0].state, State::Suspect);
        assert_eq!(counts(&rows), (0, 3, 0, 1));
    }

    #[test]
    fn zero_unbound_rows_are_left_out() {
        let node = json!({"name": "Ícone", "cornerRadius": 0.0, "layout": {"mode": "horizontal", "gap": 0.0, "padding": [0.0, 0.0, 0.0, 0.0]},
            "boundVariables": {"paddingVertical": "V1"}});
        let rows = drop_zero_unbound(rows_for(&node, &vars(), &Styles::default()));
        let props: Vec<Prop> = rows.iter().map(|r| r.property).collect();
        assert_eq!(props, [Prop::PadV]);
    }

    #[test]
    fn identical_rows_group_and_checks_follow_suspects() {
        let node = |name: &str| json!({"name": name, "type": "FRAME", "layout": {"mode": "vertical", "gap": 8.0, "padding": [0.0, 0.0, 0.0, 0.0]},
            "fills": [{"type": "solid", "color": "#FFFFFF"}], "boundVariables": {"gap": "V1", "fill": "V1"}});
        let mut rows = Vec::new();
        for (i, n) in ["A", "A", "A"].into_iter().enumerate() {
            rows.extend(rows_for(&node(n), &vars(), &Styles::default()).into_iter().map(|r| Row { node: i, ..r }));
        }
        mark_checks(&mut rows);
        // The fill is bound to a number variable: suspect; the gap's number matches: a check.
        assert_eq!(suspects(&rows), 3);
        let infos: Vec<Info> = (0..3).map(|_| Info { name: "A".into(), kind: "FRAME".into(), sub: None, part: false }).collect();
        let groups = group_layers(&drop_zero_unbound(rows.clone()), &infos);
        assert_eq!(groups.len(), 1);
        assert_eq!((groups[0].nodes.len(), groups[0].lines.len()), (3, 2));
        assert!(rows.iter().any(|r| r.state == State::Check));
    }

    #[test]
    fn numbers_show_without_noise() {
        assert_eq!(number(12.0), "12");
        assert_eq!(number(12.504), "12.5");
        assert_eq!(hex(0.0, 0.0, 0.0, 0.5), "#00000080");
    }

    fn fill_row(name: &str, kind: &str, node: usize, value: &str) -> Row {
        let n = json!({"name": name, "type": kind, "fills": [{"type": "solid", "color": value}]});
        let mut r = rows_for(&n, &vars(), &Styles::default()).remove(0);
        r.node = node;
        r
    }

    fn info(name: &str, kind: &str) -> Info {
        Info { name: name.into(), kind: kind.into(), sub: None, part: false }
    }

    #[test]
    fn same_name_type_and_rows_group() {
        let infos = [info("Root", "FRAME"), info("Cube", "FRAME"), info("Label", "INSTANCE"), info("Cube", "FRAME")];
        let rows = [fill_row("Cube", "FRAME", 1, "#FFFFFF"), fill_row("Label", "INSTANCE", 2, "#FFFFFF"), fill_row("Cube", "FRAME", 3, "#FFFFFF")];
        let groups = group_layers(&rows, &infos);
        assert_eq!(groups.len(), 2);
        assert_eq!((groups[0].name.as_str(), groups[0].nodes.clone(), groups[0].lines[0].nodes.clone()), ("Cube", vec![1, 3], vec![1, 3]));
    }

    #[test]
    fn another_name_does_not_group() {
        let infos = [info("Cube", "FRAME"), info("Square", "FRAME")];
        let rows = [fill_row("Cube", "FRAME", 0, "#FFFFFF"), fill_row("Square", "FRAME", 1, "#FFFFFF")];
        assert_eq!(group_layers(&rows, &infos).len(), 2);
    }

    #[test]
    fn another_type_does_not_group() {
        let infos = [info("Icon", "FRAME"), info("Icon", "VECTOR")];
        let rows = [fill_row("Icon", "FRAME", 0, "#FFFFFF"), fill_row("Icon", "VECTOR", 1, "#FFFFFF")];
        assert_eq!(group_layers(&rows, &infos).len(), 2);
    }

    #[test]
    fn the_whole_block_must_match() {
        // Same name and type, same fill, but one has a radius too: two blocks.
        let infos = [info("Card", "FRAME"), info("Card", "FRAME")];
        let mut rows = vec![fill_row("Card", "FRAME", 0, "#FFFFFF"), fill_row("Card", "FRAME", 1, "#FFFFFF")];
        rows.insert(1, Row { property: Prop::Radius, value: "8".into(), ..rows[0].clone() });
        assert_eq!(group_layers(&rows, &infos).len(), 2);
        // A different "Instance of" line also keeps them apart.
        let rows = [fill_row("Icon", "INSTANCE", 0, "#FFFFFF"), fill_row("Icon", "INSTANCE", 1, "#FFFFFF")];
        let infos = [Info { sub: Some("Instance of A".into()), ..info("Icon", "INSTANCE") }, Info { sub: Some("Instance of B".into()), ..info("Icon", "INSTANCE") }];
        assert_eq!(group_layers(&rows, &infos).len(), 2);
    }

    #[test]
    fn parts_get_a_block_without_rows() {
        let infos = [Info { part: true, ..info("Root", "FRAME") }, info("Deep", "FRAME"), Info { part: true, ..info("Label", "INSTANCE") }];
        let groups = group_layers(&[], &infos);
        assert_eq!(groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Root", "Label"]);
        assert!(groups.iter().all(|g| g.lines.is_empty()));
    }

    #[test]
    fn the_summary_counts_only_drawn_layers() {
        let rows = vec![fill_row("A", "FRAME", 0, "#FFFFFF"), fill_row("B", "FRAME", 1, "#FFFFFF"), fill_row("C", "FRAME", 2, "#FFFFFF")];
        let kept = keep_shown(rows, &[true, false, true]);
        assert_eq!(kept.iter().map(|r| r.node).collect::<Vec<_>>(), [0, 2]);
        assert_eq!(counts(&kept).1, 2);
    }

    fn styles() -> Styles {
        Styles::parse(&json!([
            {"id": "1:1", "name": "brand/primary", "type": "paint", "paints": [{"type": "solid", "color": {"r": 0.23137255, "g": 0.35686275, "b": 0.85882354, "a": 1.0}, "opacity": 1.0, "visible": true}]},
            {"id": "1:2", "name": "body/medium", "type": "text", "style": {"font_family": "Work Sans"}}
        ]))
    }

    fn fill_node(color: &str, style: Option<&str>) -> Value {
        json!({"name": "Box", "type": "RECTANGLE", "fills": [{"type": "solid", "color": color}], "styles": {"fill": style, "stroke": null, "text": null}})
    }

    #[test]
    fn a_colour_style_with_the_layers_value_is_a_token() {
        let rows = rows_for(&fill_node("#3B5BDB", Some("1:1")), &vars(), &styles());
        assert_eq!((rows[0].state, rows[0].token.as_str(), rows[0].style), (State::Style, "brand/primary", true));
        assert_eq!(counts(&rows).0, 1);
        assert_eq!(bound_split(&rows), (0, 1));
    }

    #[test]
    fn a_colour_style_with_another_value_is_suspect_and_a_gone_style_is_missing() {
        let rows = rows_for(&fill_node("#FF0000", Some("1:1")), &vars(), &styles());
        assert_eq!(rows[0].state, State::Suspect);
        assert!(rows[0].token.contains("#3B5BDB"));
        // A style's suspicion is not a variable collision: no warning, no "check".
        assert_eq!(suspects(&rows), 0);
        assert_eq!(counts(&rows).3, 1);
        let rows = rows_for(&fill_node("#3B5BDB", Some("9:9")), &vars(), &styles());
        assert_eq!((rows[0].state, rows[0].token.as_str()), (State::Missing, "9:9"));
    }

    #[test]
    fn a_variable_wins_over_a_style_and_the_style_is_a_note() {
        let mut node = fill_node("#3B5BDB", Some("1:1"));
        node["boundVariables"] = json!({"fill": "v:1"});
        let mut vs = vars();
        vs.by_id.insert("v:1".into(), Var { name: "accent".into(), collection: "C".into(), kind: "color".into(), value: json!({"type": "color", "value": {"r": 0.23137255, "g": 0.35686275, "b": 0.85882354, "a": 1.0}}) });
        let rows = rows_for(&node, &vs, &styles());
        assert_eq!((rows[0].state, rows[0].style), (State::Token, false));
        assert_eq!(rows[0].token, "C/accent · style brand/primary");
        assert_eq!(bound_split(&rows), (1, 0));
    }

    fn text_node(style: Option<&str>) -> Value {
        json!({"name": "T", "type": "TEXT", "fontFamily": "Work Sans", "fontSize": 14.0, "fontStyle": "Medium", "lineHeight": "150%", "letterSpacing": "0%", "textDecoration": "underline",
            "styles": {"fill": null, "stroke": null, "text": style}})
    }

    fn typography(node: &Value) -> Row {
        let mut rows = rows_for(node, &vars(), &styles());
        rows.retain(|r| r.property == Prop::Typography);
        assert_eq!(rows.len(), 1, "one typography row");
        rows.remove(0)
    }

    #[test]
    fn typography_is_a_line_with_or_without_a_text_style() {
        let r = typography(&text_node(None));
        assert_eq!((r.value.as_str(), r.state), ("Work Sans 14/150% Medium, underline", State::Unbound));
        let r = typography(&text_node(Some("1:2")));
        assert_eq!((r.state, r.token.as_str()), (State::Style, "body/medium"));
        assert_eq!(typography(&text_node(Some("8:8"))).state, State::Missing);
        // Letter spacing shows only when it is not zero.
        let mut n = text_node(None);
        n["letterSpacing"] = json!("2%");
        assert!(typography(&n).value.contains("letter spacing 2%"));
        // A text style held by the ranges alone counts too.
        let mut n = text_node(None);
        n["textStyleRanges"] = json!([[0, 6, "1:2"]]);
        assert_eq!(typography(&n).state, State::Style);
    }

    #[test]
    fn texts_with_another_typography_do_not_group() {
        let a = text_node(None);
        let mut b = text_node(None);
        b["fontSize"] = json!(12.0);
        let rows: Vec<Row> = [&a, &b].iter().enumerate().flat_map(|(i, n)| rows_for(n, &vars(), &styles()).into_iter().map(move |r| Row { node: i, ..r })).collect();
        let infos = [info("Lorem", "TEXT"), info("Lorem", "TEXT")];
        assert_eq!(group_layers(&rows, &infos).len(), 2);
        let rows: Vec<Row> = [&a, &a].iter().enumerate().flat_map(|(i, n)| rows_for(n, &vars(), &styles()).into_iter().map(move |r| Row { node: i, ..r })).collect();
        assert_eq!(group_layers(&rows, &infos).len(), 1);
    }

    #[test]
    fn hidden_paints_get_no_row() {
        let node = json!({"name": "Cube", "type": "FRAME",
            "fills": [{"type": "solid", "color": "#FFFFFF", "visible": false}],
            "strokes": [{"type": "solid", "color": "#111111", "opacity": 0.0}]});
        assert!(rows_for(&node, &vars(), &Styles::default()).is_empty());
        assert_eq!(hidden_paints(&node), 2);
        // The first paint that is drawn stands for the layer.
        let two = json!({"name": "Cube", "type": "FRAME", "fills": [{"type": "solid", "color": "#FFFFFF", "visible": false}, {"type": "solid", "color": "#000000"}]});
        assert_eq!(rows_for(&two, &vars(), &Styles::default())[0].value, "#000000");
    }
}
