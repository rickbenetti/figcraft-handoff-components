//! The Tokens frame of 0.11 part C, two tables: "Tokens by layer" (what the default variant's layers are bound to: colours, measures
//! and styles) and "Resolved colours" (every colour token of every variant once, with its value in the Dark and Light modes).
//!
//! The rules of the modes and of the aliases are plain functions (no host calls), so they are tested on the host:
//! - a collection has its own modes; its Dark and Light columns are the modes NAMED dark/escuro/escura and light/claro/clara (case
//!   ignored), when it has both. With no such pair ("Base / Escuro / Contraste" has a Dark but no Light) both columns show the
//!   collection's default mode, marked "one mode only". The default mode is the first one (the list names none).
//! - an alias is followed in the DEFAULT mode of the collection of its target, at most [`MAX_HOPS`] hops; a target with more than
//!   one mode adds a note; a loop, a chain that is too long, a missing target or a value that is not a colour is "alias with no value".

#![warn(dead_code)] // the module above allows dead code (the 0.10.0 drawing); this one is checked

use super::section::{Ctx, Cv, USEFUL_W, bold, header, pal, table_nodes};
use super::tables::{base_walk, heads, muted_note, put_table};
use super::*;
use std::collections::HashMap;

/// The columns of the two tables (they add up to 1084).
const LAYER_COLS: [f64; 4] = [220.0, 320.0, 320.0, 224.0];
const COLOUR_COLS: [f64; 3] = [400.0, 342.0, 342.0];
/// The most nodes the two tables of one run may make together (shared between them like the axes of States and Variants).
pub const TOKENS_NODE_CAP: usize = 450;
/// The most layers read, in all the variants, to find their colours (each layer is a host call, about 0.2 M of fuel).
pub const TOKENS_READ_CAP: usize = 1_000;
/// Most alias hops followed from a token to its value.
pub const MAX_HOPS: usize = 8;

// ---------------------------------------------------------------------------------------------
// Variables with their modes
// ---------------------------------------------------------------------------------------------

/// A collection of variables: its name and its modes, (id, name), the first being the default.
#[derive(Clone, Debug, PartialEq)]
pub struct Collection {
    pub name: String,
    pub modes: Vec<(String, String)>,
}

/// A variable of `variable.list` with the value it has in each mode.
#[derive(Clone, Debug)]
pub struct ModeVar {
    pub name: String,
    pub collection: usize,
    /// `{type, value}` by mode id (`type` may be `alias`).
    pub values: HashMap<String, Value>,
}

/// Every variable of the file with all its modes, by id.
#[derive(Debug, Default)]
pub struct ModeVars {
    pub collections: Vec<Collection>,
    pub by_id: HashMap<String, ModeVar>,
}

impl ModeVars {
    /// Read the reply of `variable.list`.
    pub fn parse(list: &Value) -> ModeVars {
        let mut collections = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        for c in list.get("collections").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
            let id = c.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
            let modes = c
                .get("modes")
                .and_then(Value::as_array)
                .map(|m| m.iter().map(|m| (m.get("id").and_then(Value::as_str).unwrap_or_default().to_string(), m.get("name").and_then(Value::as_str).unwrap_or_default().to_string())).collect())
                .unwrap_or_default();
            index.insert(id, collections.len());
            collections.push(Collection { name: c.get("name").and_then(Value::as_str).unwrap_or_default().to_string(), modes });
        }
        let mut by_id = HashMap::new();
        for v in list.get("variables").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
            let Some(id) = v.get("id").and_then(Value::as_str) else { continue };
            let Some(&collection) = v.get("collection").and_then(Value::as_str).and_then(|c| index.get(c)) else { continue };
            let values = v.get("values").and_then(Value::as_object).map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
            by_id.insert(
                id.to_string(),
                ModeVar { name: v.get("name").and_then(Value::as_str).unwrap_or_default().to_string(), collection, values },
            );
        }
        ModeVars { collections, by_id }
    }

    /// `collection/name`.
    pub fn path(&self, var: &ModeVar) -> String {
        format!("{}/{}", self.collections.get(var.collection).map_or("?", |c| c.name.as_str()), var.name)
    }
}

/// The two columns of the resolved colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Dark,
    Light,
}

/// Which mode of a collection each column shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Columns {
    /// The collection has a Dark and a Light mode: their ids.
    Mapped { dark: String, light: String },
    /// It has not: both columns show the default mode (the first one).
    OneMode { mode: String },
}

/// The role a mode's name stands for: dark/escuro/escura or light/claro/clara, in any case.
pub fn role_of(mode_name: &str) -> Option<Role> {
    match mode_name.trim().to_lowercase().as_str() {
        "dark" | "escuro" | "escura" => Some(Role::Dark),
        "light" | "claro" | "clara" => Some(Role::Light),
        _ => None,
    }
}

/// The columns of a collection: Dark and Light by name when it has both (the first mode of each name), else its default mode for both.
pub fn columns(c: &Collection) -> Columns {
    let find = |role: Role| c.modes.iter().find(|(_, n)| role_of(n) == Some(role)).map(|(id, _)| id.clone());
    match (find(Role::Dark), find(Role::Light)) {
        (Some(dark), Some(light)) => Columns::Mapped { dark, light },
        _ => Columns::OneMode { mode: c.modes.first().map(|(id, _)| id.clone()).unwrap_or_default() },
    }
}

/// A colour: `#RRGGBB` and its opacity (0 to 1).
pub type Colour = (String, f64);

/// What a token's value comes to in one column.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolution {
    pub colour: Option<Colour>,
    /// The token's own collection has no Dark/Light pair.
    pub one_mode: bool,
    /// An alias went to a collection with more than one mode: how many, and the name of the one used.
    pub target_modes: Option<(usize, String)>,
    /// No value: a loop, a chain longer than [`MAX_HOPS`], a missing target or a value that is not a colour.
    pub broken: bool,
}

fn colour_of(value: &Value) -> Option<Colour> {
    let ch = |k: &str| value.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let alpha = value.get("a").and_then(Value::as_f64).unwrap_or(1.0);
    Some((audit::hex(ch("r"), ch("g"), ch("b"), 1.0), alpha.clamp(0.0, 1.0)))
}

/// The colour of the variable `id` in the column `role`: its own mode by the columns of its collection, then through its aliases in the
/// default mode of each target's collection.
pub fn resolve(vars: &ModeVars, id: &str, role: Role) -> Resolution {
    let mut out = Resolution::default();
    let Some(mut var) = vars.by_id.get(id) else {
        out.broken = true;
        return out;
    };
    let Some(coll) = vars.collections.get(var.collection) else {
        out.broken = true;
        return out;
    };
    let mut mode = match columns(coll) {
        Columns::Mapped { dark, light } => if role == Role::Dark { dark } else { light },
        Columns::OneMode { mode } => {
            out.one_mode = true;
            mode
        }
    };
    let mut seen = vec![id.to_string()];
    for hop in 0..=MAX_HOPS {
        // The mode's value, or the first one the variable has when it has none for this mode.
        let Some(value) = var.values.get(&mode).or_else(|| var.values.values().next()) else {
            out.broken = true;
            return out;
        };
        match (value.get("type").and_then(Value::as_str), value.get("value")) {
            (Some("color"), Some(v)) => {
                out.colour = colour_of(v);
                out.broken = out.colour.is_none();
                return out;
            }
            (Some("alias"), Some(Value::String(next))) => {
                if hop == MAX_HOPS || seen.contains(next) {
                    out.broken = true;
                    return out;
                }
                seen.push(next.clone());
                let Some(target) = vars.by_id.get(next) else {
                    out.broken = true;
                    return out;
                };
                let Some(target_coll) = vars.collections.get(target.collection) else {
                    out.broken = true;
                    return out;
                };
                mode = target_coll.modes.first().map(|(id, _)| id.clone()).unwrap_or_default();
                if target_coll.modes.len() > 1 && out.target_modes.is_none() {
                    out.target_modes = Some((target_coll.modes.len(), target_coll.modes[0].1.clone()));
                }
                var = target;
            }
            _ => {
                out.broken = true;
                return out;
            }
        }
    }
    out.broken = true;
    out
}

// ---------------------------------------------------------------------------------------------
// Table 1: the tokens of each layer of the default variant
// ---------------------------------------------------------------------------------------------

/// What one layer is bound to: colours, measures and styles, each as a value cell.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerTokens {
    pub name: String,
    pub colours: Vec<Cv>,
    pub measures: Vec<Cv>,
    pub styles: Vec<Cv>,
}

fn hex_only(value: &str) -> String {
    let rest = value.strip_prefix('#').unwrap_or_default();
    let n = rest.chars().take_while(char::is_ascii_hexdigit).count();
    format!("#{}", &rest[..n.min(8)])
}

/// The tokens of one layer from its audit rows (only a row bound to a variable or a style counts) and the name of its effect style.
/// `None` when the layer uses no token.
pub fn layer_tokens(lang: Lang, name: &str, rows: &[Row], effect_style: Option<&str>) -> Option<LayerTokens> {
    let mut out = LayerTokens { name: name.to_string(), colours: Vec::new(), measures: Vec::new(), styles: Vec::new() };
    for r in rows.iter().filter(|r| matches!(r.state, State::Token | State::Check | State::Style)) {
        let label = t(lang, delta::Label::Prop(r.property).key());
        match r.property {
            Prop::Fill | Prop::Stroke => out.colours.push(Cv::Val { token: Some(r.token.clone()), text: format!("{label} · {}", r.value), swatch: Some((hex_only(&r.value), false)) }),
            Prop::Typography => out.styles.push(Cv::Val { token: Some(r.token.clone()), text: r.value.clone(), swatch: None }),
            _ => out.measures.push(Cv::Val { token: Some(r.token.clone()), text: format!("{label} · {}px", r.value), swatch: None }),
        }
    }
    if let Some(e) = effect_style {
        out.styles.push(Cv::Val { token: Some(e.to_string()), text: t(lang, Key::DeltaEffects).to_string(), swatch: None });
    }
    (!out.colours.is_empty() || !out.measures.is_empty() || !out.styles.is_empty()).then_some(out)
}

/// Layers that follow each other and are alike (same name, same tokens) are one, with how many they are.
pub fn merge_consecutive(list: Vec<LayerTokens>) -> Vec<(LayerTokens, usize)> {
    let mut out: Vec<(LayerTokens, usize)> = Vec::new();
    for l in list {
        match out.last_mut() {
            Some((last, n)) if *last == l => *n += 1,
            _ => out.push((l, 1)),
        }
    }
    out
}

fn items_cell(mut items: Vec<Cv>) -> Cv {
    match items.len() {
        0 => Cv::Guide("—".to_string()),
        1 => items.remove(0),
        _ => Cv::Many(items),
    }
}

/// The rows of the table: layer, colours, measures, styles.
pub fn layer_rows(layers: &[(LayerTokens, usize)]) -> Vec<Vec<Cv>> {
    layers
        .iter()
        .map(|(l, n)| {
            let name = if *n > 1 { format!("{} (x{n})", l.name) } else { l.name.clone() };
            vec![Cv::Bold(name), items_cell(l.colours.clone()), items_cell(l.measures.clone()), items_cell(l.styles.clone())]
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Table 2: every colour of every variant, once
// ---------------------------------------------------------------------------------------------

/// A colour a variant's layer shows: bound to a variable, to a style, or loose.
#[derive(Clone, Debug, PartialEq)]
pub enum Use {
    Variable(String),
    Style(String, Colour),
    Loose(Colour),
}

/// The distinct colours of the variants, in the order they are met (the default variant first).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Collected {
    pub uses: Vec<Use>,
}

impl Collected {
    pub fn add(&mut self, u: Use) {
        let same = |a: &Use| match (a, &u) {
            (Use::Variable(x), Use::Variable(y)) | (Use::Style(x, _), Use::Style(y, _)) => x == y,
            (Use::Loose(x), Use::Loose(y)) => x.0 == y.0 && (x.1 - y.1).abs() < 0.005,
            _ => false,
        };
        if !self.uses.iter().any(same) {
            self.uses.push(u);
        }
    }
}

/// How the colour tokens came out, for the answer and the report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Tokens whose collection has a Dark and a Light mode (the columns differ by mode).
    pub mapped: usize,
    /// Tokens of a collection with no such pair ("one mode only").
    pub one_mode: usize,
    /// Colours bound to a style (a style has no modes).
    pub style: usize,
    /// Loose values (no variable, no style).
    pub loose: usize,
    /// Tokens whose alias ends in a collection with more than one mode.
    pub alias_multi: usize,
    /// Tokens with an alias that has no value.
    pub broken: usize,
}

fn percent(alpha: f64) -> String {
    format!("{}%", audit::number((alpha * 100.0).round()))
}

/// The cell of a colour: its square, `#RRGGBB`, its opacity when it is not full and a note.
fn colour_cell(c: &Colour, notes: &[String]) -> Cv {
    let mut text = c.0.clone();
    if c.1 < 0.995 {
        text.push_str(&format!(" · {}", percent(c.1)));
    }
    for n in notes {
        text.push_str(&format!(" · {n}"));
    }
    let swatch = if c.1 < 0.995 { format!("{}{:02X}", c.0, (c.1 * 255.0).round() as u8) } else { c.0.clone() };
    Cv::Val { token: None, text, swatch: Some((swatch, c.1 < 0.995)) }
}

fn column(lang: Lang, r: &Resolution) -> Cv {
    let Some(colour) = &r.colour else { return Cv::Guide(t(lang, Key::TokNoValue).to_string()) };
    let mut notes = Vec::new();
    if r.one_mode {
        notes.push(t(lang, Key::TokOneMode).to_string());
    }
    if let Some((n, mode)) = &r.target_modes {
        notes.push(tf(lang, Key::TokTargetModes, &[("n", &n.to_string()), ("mode", mode)]));
    }
    colour_cell(colour, &notes)
}

/// The rows of "Resolved colours" (token, Dark, Light) and how they came out: tokens and styles first, loose values last.
pub fn colour_rows(lang: Lang, vars: &ModeVars, styles: &Styles, found: &Collected) -> (Vec<Vec<Cv>>, Stats) {
    let mut stats = Stats::default();
    let mut rows = Vec::new();
    for u in &found.uses {
        if let Use::Variable(id) = u {
            let (dark, light) = (resolve(vars, id, Role::Dark), resolve(vars, id, Role::Light));
            let name = vars.by_id.get(id).map_or_else(|| id.clone(), |v| vars.path(v));
            if dark.one_mode { stats.one_mode += 1 } else { stats.mapped += 1 }
            if dark.target_modes.is_some() || light.target_modes.is_some() {
                stats.alias_multi += 1;
            }
            if dark.broken || light.broken {
                stats.broken += 1;
            }
            rows.push(vec![Cv::Bold(name), column(lang, &dark), column(lang, &light)]);
        }
    }
    for u in &found.uses {
        if let Use::Style(id, colour) = u {
            stats.style += 1;
            let name = styles.by_id.get(id).map_or_else(|| id.clone(), |s| s.name.clone());
            let one = t(lang, Key::TokOneMode).to_string();
            rows.push(vec![Cv::Bold(name), colour_cell(colour, std::slice::from_ref(&one)), colour_cell(colour, &[one])]);
        }
    }
    for u in &found.uses {
        if let Use::Loose(colour) = u {
            stats.loose += 1;
            rows.push(vec![Cv::Guide(t(lang, Key::TokLoose).to_string()), colour_cell(colour, &[]), colour_cell(colour, &[])]);
        }
    }
    (rows, stats)
}

// ---------------------------------------------------------------------------------------------
// The nodes shared between the two tables
// ---------------------------------------------------------------------------------------------

/// What a table asks for when the nodes are shared: the nodes of its headers and the cost of each of its rows.
pub struct Ask {
    pub head: usize,
    pub rows: Vec<usize>,
}

/// How many rows each table gets out of `cap` nodes: first every table with rows gets its header, then the tables take a row in turn
/// while the next row of each fits (a table whose next row does not fit stops; the other goes on), so neither takes what the other needs.
pub fn share_rows(asks: &[Ask], cap: usize) -> Vec<usize> {
    let mut taken = vec![0usize; asks.len()];
    let mut used = 0usize;
    let mut active: Vec<bool> = asks
        .iter()
        .map(|a| {
            let ok = !a.rows.is_empty() && used + a.head <= cap;
            if ok {
                used += a.head;
            }
            ok
        })
        .collect();
    loop {
        let mut progressed = false;
        for (i, a) in asks.iter().enumerate() {
            if !active[i] || taken[i] >= a.rows.len() {
                continue;
            }
            let cost = a.rows[taken[i]];
            if used + cost > cap {
                active[i] = false;
            } else {
                used += cost;
                taken[i] += 1;
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }
    taken
}

/// The nodes of a row (its frame and its cells).
fn row_cost(r: &[Cv]) -> usize {
    1 + r.iter().map(Cv::nodes).sum::<usize>()
}

// ---------------------------------------------------------------------------------------------
// Reading the layers
// ---------------------------------------------------------------------------------------------

const COLOUR_FIELDS: [&str; 6] = ["boundVariables", "fills", "strokes", "visible", "children", "styles"];

/// A paint that is drawn: not switched off and not fully transparent.
fn shown(p: &Value) -> bool {
    p.get("visible").and_then(Value::as_bool).unwrap_or(true) && p.get("opacity").and_then(Value::as_f64).is_none_or(|o| o > 0.0)
}

/// The colour use of a node's first drawn paint of `key` (`fills` or `strokes`); `bind` is the key of its variable and its style.
pub fn paint_use(node: &Value, key: &str, bind: &str) -> Option<Use> {
    let paint = node.get(key).and_then(Value::as_array)?.iter().find(|p| shown(p))?;
    if paint.get("type").and_then(Value::as_str) != Some("solid") {
        return None;
    }
    if let Some(id) = node.get("boundVariables").and_then(|b| b.get(bind)).and_then(Value::as_str) {
        return Some(Use::Variable(id.to_string()));
    }
    let raw = paint.get("color").and_then(Value::as_str)?.to_uppercase();
    let hex = raw.get(..7)?.to_string();
    let alpha = raw.get(7..9).and_then(|a| u8::from_str_radix(a, 16).ok()).map_or(1.0, |a| f64::from(a) / 255.0) * paint.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
    match node.get("styles").and_then(|s| s.get(bind)).and_then(Value::as_str) {
        Some(style) => Some(Use::Style(style.to_string(), (hex, alpha))),
        None => Some(Use::Loose((hex, alpha))),
    }
}

/// What reading the variants found.
struct Read {
    found: Collected,
    /// A layer with a variable bound (so that `variable.list` is worth asking), and one with a style bound (likewise `style.list`).
    probe: Vec<Value>,
    style_probe: Vec<Value>,
    read: usize,
    variants_read: usize,
}

/// The colours of the layers (and the nested ones) of each of `ids`, in order, until [`TOKENS_READ_CAP`] layers have been read.
fn read_colours(lang: Lang, ids: &[u64]) -> Result<Read> {
    let mut out = Read { found: Collected::default(), probe: Vec::new(), style_probe: Vec::new(), read: 0, variants_read: 0 };
    for &id in ids {
        if out.read >= TOKENS_READ_CAP {
            break;
        }
        let mut stack = vec![id];
        while let Some(at) = stack.pop() {
            let node = host(lang, "node.get", &json!({"id": at, "fields": COLOUR_FIELDS}))?;
            out.read += 1;
            if node.get("visible").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            if let Some(children) = node.get("children").and_then(Value::as_array) {
                stack.extend(children.iter().rev().filter_map(|c| c.get("id").and_then(Value::as_u64)));
            }
            for (key, bind) in [("fills", "fill"), ("strokes", "stroke")] {
                if let Some(u) = paint_use(&node, key, bind) {
                    if matches!(u, Use::Variable(_)) && out.probe.is_empty() {
                        out.probe.push(node.clone());
                    }
                    if matches!(u, Use::Style(..)) && out.style_probe.is_empty() {
                        out.style_probe.push(node.clone());
                    }
                    out.found.add(u);
                }
            }
        }
        out.variants_read += 1;
    }
    Ok(out)
}

/// The ids of the variants to read, the default one first (a component that is not in a set: itself).
fn variant_ids(c: &Ctx) -> Vec<u64> {
    match &c.target.set {
        Some(s) => {
            let first = s.info.find(&s.default).map(|v| v.id);
            first.into_iter().chain(s.info.variants.iter().map(|v| v.id).filter(|id| Some(*id) != first)).collect()
        }
        None => vec![c.target.component],
    }
}

/// The Tokens frame: the two tables.
pub(super) fn build_tokens(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    header(c, frame, t(lang, Key::SecTokens))?;
    // 1. The default variant's layers: the tokens of each.
    let walked = base_walk(c)?;
    let nodes: Vec<Value> = walked.iter().map(|(n, _)| n.clone()).collect();
    let vars = c.vars_for(&nodes)?;
    let styles = c.styles_for(&nodes)?;
    let drawn = rendered(&walked);
    let mut layers: Vec<LayerTokens> = Vec::new();
    for (i, (n, _)) in walked.iter().enumerate() {
        if !drawn.get(i).copied().unwrap_or(false) {
            continue;
        }
        let rows = audit::rows_for(lang, n, &vars, &styles);
        let effect = n.get("styles").and_then(|s| s.get("effect")).and_then(Value::as_str).and_then(|id| styles.by_id.get(id)).map(|d| d.name.clone());
        let name = if i == 0 { c.target.title.clone() } else { n.get("name").and_then(Value::as_str).unwrap_or_default().to_string() };
        layers.extend(layer_tokens(lang, &name, &rows, effect.as_deref()));
    }
    let layer_list = merge_consecutive(layers);
    let layer_body = layer_rows(&layer_list);
    // 2. Every variant's colours, each once.
    let ids = variant_ids(c);
    let read = read_colours(lang, &ids)?;
    let modes = c.modes_for(&read.probe)?;
    // The styles of the other variants: read now when the default variant had none (the names of the styles that colour a layer).
    let colour_styles = c.styles_for(&read.style_probe)?;
    let (colour_body, stats) = colour_rows(lang, &modes, &colour_styles, &read.found);
    // 3. The nodes are shared between the tables.
    let asks = [Ask { head: table_nodes(4, &[]), rows: layer_body.iter().map(|r| row_cost(r)).collect() }, Ask { head: table_nodes(3, &[]), rows: colour_body.iter().map(|r| row_cost(r)).collect() }];
    let taken = share_rows(&asks, TOKENS_NODE_CAP);
    // 4. Each table in a frame of its own with its title.
    let mut left = [0usize; 2];
    for (k, (title, cols, key_heads, body)) in [(Key::TokTableLayers, &LAYER_COLS[..], &[Key::TblLayer, Key::TblColours, Key::TblMeasures, Key::TblStyles][..], &layer_body), (Key::TokTableColours, &COLOUR_COLS[..], &[Key::TblToken, Key::TblDark, Key::TblLight][..], &colour_body)].into_iter().enumerate() {
        let mut box_ = FlowSpec::new(t(lang, title), USEFUL_W, "vertical");
        box_.gap = 16.0;
        let box_ = flow_frame(lang, frame, box_)?;
        text_with(lang, box_, t(lang, title), theme::SECTION_SIZE, pal::TEXT, "Title", bold())?;
        if body.is_empty() {
            muted_note(lang, box_, t(lang, if k == 0 { Key::TokNone } else { Key::TokNoColours }))?;
            continue;
        }
        let n = taken[k];
        if n > 0 {
            put_table(c, box_, t(lang, title), cols, &heads(lang, key_heads), &body[..n])?;
        }
        left[k] = body.len() - n;
        if left[k] > 0 {
            muted_note(lang, box_, &tf(lang, plural(left[k], Key::MatrixLeftOutOne, Key::MatrixLeftOut), &[("n", &left[k].to_string())]))?;
        }
    }
    // 5. What the second table could not show.
    if stats.broken > 0 {
        muted_note(lang, frame, &tf(lang, Key::TokBroken, &[("n", &stats.broken.to_string())]))?;
    }
    let not_read = ids.len() - read.variants_read;
    if not_read > 0 {
        muted_note(lang, frame, &tf(lang, plural(not_read, Key::TokNotReadOne, Key::TokNotRead), &[("n", &not_read.to_string())]))?;
    }
    c.tokens.borrow_mut().replace(json!({
        "layers": layer_list.len(), "layer_rows": layer_body.len(), "colour_rows": colour_body.len(), "left_out": left, "read_layers": read.read, "variants": ids.len(), "variants_read": read.variants_read,
        "mapped": stats.mapped, "one_mode": stats.one_mode, "style": stats.style, "loose": stats.loose, "alias_multi": stats.alias_multi, "broken": stats.broken,
        "collections": modes.collections.len(),
    }));
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn list(collections: Value, variables: Value) -> ModeVars {
        ModeVars::parse(&json!({"collections": collections, "variables": variables}))
    }

    fn col(r: f64, g: f64, b: f64, a: f64) -> Value {
        json!({"type": "color", "value": {"r": r, "g": g, "b": b, "a": a}})
    }

    fn alias(id: &str) -> Value {
        json!({"type": "alias", "value": id})
    }

    fn var(id: &str, coll: &str, name: &str, values: Value) -> Value {
        json!({"id": id, "collection": coll, "name": name, "resolvedType": "color", "values": values})
    }

    fn modes(names: &[&str]) -> Collection {
        Collection { name: "C".into(), modes: names.iter().enumerate().map(|(i, n)| (format!("m{i}"), n.to_string())).collect() }
    }

    #[test]
    fn dark_and_light_are_found_by_name_in_both_languages_and_any_case() {
        for (a, b) in [("Dark", "Light"), ("dark", "light"), ("DARK", "LIGHT"), ("Escuro", "Claro"), ("escura", "clara"), ("Light", "Dark"), (" Claro ", " Escuro "), ("Dark", "Clara")] {
            let c = modes(&[a, b]);
            let Columns::Mapped { dark, light } = columns(&c) else { panic!("{a}/{b} should be mapped") };
            let name = |id: &str| c.modes.iter().find(|(i, _)| i == id).map(|(_, n)| n.trim().to_lowercase()).unwrap_or_default();
            assert_eq!(role_of(&name(&dark)), Some(Role::Dark), "{a}/{b}");
            assert_eq!(role_of(&name(&light)), Some(Role::Light), "{a}/{b}");
        }
    }

    #[test]
    fn a_collection_with_one_mode_or_without_the_pair_shows_its_default_mode_in_both_columns() {
        assert_eq!(columns(&modes(&["Mode 1"])), Columns::OneMode { mode: "m0".into() });
        // Base / Escuro / Contraste has a Dark (Escuro) but no recognisable Light: nothing is invented, the default (Base) is used.
        assert_eq!(columns(&modes(&["Base", "Escuro", "Contraste"])), Columns::OneMode { mode: "m0".into() });
        assert_eq!(columns(&modes(&["Dark"])), Columns::OneMode { mode: "m0".into() });
        assert_eq!(columns(&modes(&["Light", "High contrast"])), Columns::OneMode { mode: "m0".into() });
        // With three modes and both names present, Dark and Light are those two and the third is ignored.
        assert_eq!(columns(&modes(&["Base", "Escuro", "Claro"])), Columns::Mapped { dark: "m1".into(), light: "m2".into() });
    }

    fn colours() -> ModeVars {
        list(
            json!([
                {"id": "P", "name": "Primitives", "modes": [{"id": "P/1", "name": "Value"}]},
                {"id": "P2", "name": "Two", "modes": [{"id": "P2/1", "name": "Base"}, {"id": "P2/2", "name": "Alt"}]},
                {"id": "T", "name": "Tokens", "modes": [{"id": "T/l", "name": "Light"}, {"id": "T/d", "name": "Dark"}]},
                {"id": "S", "name": "Single", "modes": [{"id": "S/1", "name": "Mode 1"}]}
            ]),
            json!([
                var("blue", "P", "blue/600", json!({"P/1": col(0.0, 0.0, 1.0, 1.0)})),
                var("white", "P", "white", json!({"P/1": col(1.0, 1.0, 1.0, 0.8)})),
                var("two", "P2", "two", json!({"P2/1": col(1.0, 0.0, 0.0, 1.0), "P2/2": col(0.0, 1.0, 0.0, 1.0)})),
                var("bg", "T", "color/bg", json!({"T/l": alias("white"), "T/d": alias("blue")})),
                var("fg", "T", "color/fg", json!({"T/l": col(0.0, 0.0, 0.0, 1.0), "T/d": col(1.0, 1.0, 1.0, 1.0)})),
                var("alt", "T", "color/alt", json!({"T/l": alias("two"), "T/d": alias("two")})),
                var("chain", "T", "color/chain", json!({"T/l": alias("bg2"), "T/d": alias("bg2")})),
                var("bg2", "S", "bg2", json!({"S/1": alias("blue")})),
                var("solo", "S", "solo", json!({"S/1": col(1.0, 1.0, 0.0, 1.0)}))
            ]),
        )
    }

    #[test]
    fn a_dark_light_collection_gives_each_column_its_own_mode() {
        let v = colours();
        let (d, l) = (resolve(&v, "fg", Role::Dark), resolve(&v, "fg", Role::Light));
        assert_eq!((d.colour.clone().map(|c| c.0), l.colour.clone().map(|c| c.0)), (Some("#FFFFFF".into()), Some("#000000".into())));
        assert!(!d.one_mode && !d.broken && d.target_modes.is_none());
    }

    #[test]
    fn a_single_mode_collection_gives_the_same_value_and_the_note() {
        let v = colours();
        let (d, l) = (resolve(&v, "solo", Role::Dark), resolve(&v, "solo", Role::Light));
        assert_eq!(d, l);
        assert!(d.one_mode);
        assert_eq!(d.colour, Some(("#FFFF00".into(), 1.0)));
    }

    #[test]
    fn an_alias_to_a_single_mode_target_resolves_in_each_column_without_a_note() {
        let v = colours();
        let (d, l) = (resolve(&v, "bg", Role::Dark), resolve(&v, "bg", Role::Light));
        assert_eq!(d.colour, Some(("#0000FF".into(), 1.0)));
        assert_eq!(l.colour, Some(("#FFFFFF".into(), 0.8)), "the alpha is the target's");
        assert!(d.target_modes.is_none() && l.target_modes.is_none() && !d.broken);
    }

    #[test]
    fn an_alias_to_a_target_with_several_modes_uses_its_default_and_says_so() {
        let v = colours();
        let (d, l) = (resolve(&v, "alt", Role::Dark), resolve(&v, "alt", Role::Light));
        assert_eq!(d.colour, Some(("#FF0000".into(), 1.0)));
        assert_eq!(d, Resolution { target_modes: Some((2, "Base".into())), ..d.clone() });
        assert_eq!(l.target_modes, Some((2, "Base".into())));
    }

    #[test]
    fn a_chain_of_two_levels_is_followed_to_its_value() {
        let v = colours();
        // chain (Tokens, Light/Dark) -> bg2 (Single) -> blue (Primitives).
        let d = resolve(&v, "chain", Role::Dark);
        assert_eq!(d.colour, Some(("#0000FF".into(), 1.0)));
        assert!(!d.broken);
    }

    fn loop_of(n: usize) -> ModeVars {
        // n variables in one single-mode collection, each an alias to the next, the last one a colour.
        let vars: Vec<Value> = (0..=n).map(|i| if i == n { var(&format!("v{i}"), "S", "x", json!({"S/1": col(1.0, 0.0, 0.0, 1.0)})) } else { var(&format!("v{i}"), "S", "x", json!({"S/1": alias(&format!("v{}", i + 1))})) }).collect();
        list(json!([{"id": "S", "name": "S", "modes": [{"id": "S/1", "name": "Mode 1"}]}]), json!(vars))
    }

    #[test]
    fn the_depth_is_eight_hops_and_a_longer_chain_has_no_value() {
        assert!(!resolve(&loop_of(MAX_HOPS), "v0", Role::Dark).broken, "8 hops resolve");
        assert!(resolve(&loop_of(MAX_HOPS + 1), "v0", Role::Dark).broken, "9 hops do not");
        assert_eq!(resolve(&loop_of(MAX_HOPS + 1), "v0", Role::Dark).colour, None);
    }

    #[test]
    fn a_loop_and_a_missing_target_are_alias_with_no_value_and_do_not_hang() {
        let cyc = list(
            json!([{"id": "S", "name": "S", "modes": [{"id": "S/1", "name": "Mode 1"}]}]),
            json!([var("a", "S", "a", json!({"S/1": alias("b")})), var("b", "S", "b", json!({"S/1": alias("a")})), var("self", "S", "s", json!({"S/1": alias("self")})), var("gone", "S", "g", json!({"S/1": alias("nowhere")}))]),
        );
        for id in ["a", "self", "gone", "nowhere"] {
            let r = resolve(&cyc, id, Role::Light);
            assert!(r.broken && r.colour.is_none(), "{id}");
        }
    }

    #[test]
    fn the_resolved_rows_count_how_each_colour_came_out_and_mark_loose_values() {
        let v = colours();
        let mut found = Collected::default();
        for u in [Use::Loose(("#112233".into(), 1.0)), Use::Variable("bg".into()), Use::Variable("solo".into()), Use::Variable("alt".into()), Use::Variable("bg".into()), Use::Style("S1".into(), ("#445566".into(), 0.5))] {
            found.add(u);
        }
        assert_eq!(found.uses.len(), 5, "a token is listed once");
        let (rows, stats) = colour_rows(Lang::En, &v, &Styles::default(), &found);
        assert_eq!(stats, Stats { mapped: 2, one_mode: 1, style: 1, loose: 1, alias_multi: 1, broken: 0 });
        assert_eq!(rows.len(), 5);
        // Tokens first, then styles, loose values last.
        assert_eq!(rows[0][0], Cv::Bold("Tokens/color/bg".into()));
        assert_eq!(rows[4][0], Cv::Guide("loose value".into()));
        // "#FFFFFF · 80%" for the 80% white of the light column.
        let Cv::Val { text, swatch, .. } = &rows[0][2] else { panic!() };
        assert_eq!(text, "#FFFFFF · 80%");
        assert_eq!(swatch, &Some(("#FFFFFFCC".into(), true)));
        let Cv::Val { text, .. } = &rows[1][1] else { panic!() };
        assert_eq!(text, "#FFFF00 · one mode only");
        let Cv::Val { text, .. } = &rows[2][1] else { panic!() };
        assert_eq!(text, "#FF0000 · target has 2 modes: used Base");
    }

    #[test]
    fn a_broken_alias_is_a_cell_that_says_so_and_a_count() {
        let v = loop_of(MAX_HOPS + 1);
        let mut found = Collected::default();
        found.add(Use::Variable("v0".into()));
        let (rows, stats) = colour_rows(Lang::En, &v, &Styles::default(), &found);
        assert_eq!(stats.broken, 1);
        assert_eq!(rows[0][1], Cv::Guide("alias with no value".into()));
    }

    fn row(property: Prop, token: &str, value: &str, state: State) -> Row {
        Row { property, token: token.into(), value: value.into(), state, zero: false, node: 0, depth: 0, style: false }
    }

    #[test]
    fn a_layer_lists_its_colour_measure_and_style_tokens_and_skips_what_is_not_bound() {
        let rows = [
            row(Prop::Fill, "Colors/primary", "#C6005C", State::Token),
            row(Prop::Stroke, "—", "#000000", State::Unbound),
            row(Prop::PadH, "Spacing/16", "16", State::Token),
            row(Prop::Radius, "Radius/default", "10", State::Token),
            row(Prop::Gap, "—", "8", State::Unbound),
            row(Prop::Typography, "text/label-m", "Inter 14/20 Medium", State::Style),
        ];
        let l = layer_tokens(Lang::En, "Button", &rows, Some("shadow/sm")).unwrap();
        assert_eq!(l.colours.len(), 1);
        assert_eq!(l.measures.len(), 2);
        assert_eq!(l.styles.len(), 2, "the text style and the effect style");
        assert_eq!(l.measures[0], Cv::Val { token: Some("Spacing/16".into()), text: "Horizontal padding · 16px".into(), swatch: None });
        assert!(layer_tokens(Lang::En, "Plain", &[row(Prop::Fill, "—", "#FFFFFF", State::Unbound)], None).is_none(), "a layer with only loose values has no row");
    }

    #[test]
    fn layers_that_follow_each_other_and_are_alike_are_one_with_a_count() {
        let l = |name: &str, token: &str| LayerTokens { name: name.into(), colours: vec![Cv::Text(token.into())], measures: vec![], styles: vec![] };
        let merged = merge_consecutive(vec![l("Icon", "a"), l("Icon", "a"), l("Icon", "a"), l("Label", "b"), l("Icon", "a")]);
        assert_eq!(merged.iter().map(|(l, n)| (l.name.as_str(), *n)).collect::<Vec<_>>(), [("Icon", 3), ("Label", 1), ("Icon", 1)], "only consecutive layers merge");
        let rows = layer_rows(&merged);
        assert_eq!(rows[0][0], Cv::Bold("Icon (x3)".into()));
        assert_eq!(rows[1][2], Cv::Guide("—".into()));
    }

    #[test]
    fn the_cap_gives_each_table_its_header_and_its_first_rows_and_leaves_the_rest_out() {
        let big = Ask { head: 10, rows: vec![30; 40] };
        let small = Ask { head: 8, rows: vec![12; 6] };
        let taken = share_rows(&[big, small], 400);
        // The small table is shown whole (6 x 12 + 8 = 80), the big one takes the rest: (400 - 80 - 10) / 30 = 10 rows.
        assert_eq!(taken, [10, 6]);
        // Neither is left with nothing when there is room for a header and a row.
        let taken = share_rows(&[Ask { head: 10, rows: vec![1000; 3] }, Ask { head: 8, rows: vec![12; 3] }], 100);
        assert_eq!(taken, [0, 3], "a row that does not fit is left out, the other table goes on");
        assert_eq!(share_rows(&[Ask { head: 10, rows: vec![] }, Ask { head: 8, rows: vec![5; 2] }], 100), [0, 2], "a table without rows takes nothing");
    }

    #[test]
    fn a_paint_is_a_variable_a_style_or_a_loose_value() {
        let fill = |extra: Value| {
            let mut n = json!({"fills": [{"type": "solid", "color": "#c6005c"}]});
            if let (Some(o), Some(e)) = (n.as_object_mut(), extra.as_object()) {
                o.extend(e.clone());
            }
            n
        };
        assert_eq!(paint_use(&fill(json!({"boundVariables": {"fill": "V1"}})), "fills", "fill"), Some(Use::Variable("V1".into())));
        assert_eq!(paint_use(&fill(json!({"styles": {"fill": "S1"}})), "fills", "fill"), Some(Use::Style("S1".into(), ("#C6005C".into(), 1.0))));
        assert_eq!(paint_use(&fill(json!({})), "fills", "fill"), Some(Use::Loose(("#C6005C".into(), 1.0))));
        let half = json!({"fills": [{"type": "solid", "color": "#112233", "opacity": 0.5}]});
        assert_eq!(paint_use(&half, "fills", "fill"), Some(Use::Loose(("#112233".into(), 0.5))));
        let off = json!({"fills": [{"type": "solid", "color": "#112233", "visible": false}, {"type": "solid", "color": "#445566"}]});
        assert_eq!(paint_use(&off, "fills", "fill"), Some(Use::Loose(("#445566".into(), 1.0))), "the first paint that is drawn");
        assert_eq!(paint_use(&json!({"fills": [{"type": "gradient"}]}), "fills", "fill"), None);
    }

    #[test]
    fn the_columns_add_up_to_the_useful_width() {
        assert_eq!(LAYER_COLS.iter().sum::<f64>(), USEFUL_W);
        assert_eq!(COLOUR_COLS.iter().sum::<f64>(), USEFUL_W);
    }
}
