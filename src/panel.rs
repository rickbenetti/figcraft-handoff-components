//! The "Generate documentation…" panel (plugin API 2): what the menu command shows.
//!
//! One panel does everything: the page's settings (language, line and fill colours, stage, what to do on a
//! rerun), the documentation's Version and Status (empty = "to be defined", and emptied again after a
//! documentation is made: nothing is remembered), the messages, and the buttons. The button "Generate
//! documentation" makes the documentation INSIDE its own click, for whatever is selected at that moment,
//! then takes the screen to its start and gives the selection back. The panel stays open.
//!
//! The first half of this file is pure logic (the form, the panel's spec, what a click would write); the
//! second half is the glue to the host. Texts come from the dictionary.

use crate::direct::write_data;
use crate::gerar::{generate, host, kind_word};
use crate::i18n::{Key, Lang, t, tf};
use crate::store::{DEFAULT_FILL_OPACITY, Rerun, SETTINGS_KEY, Settings, StageMode, is_hex_colour};
use crate::theme;
use figcraft_plugin_sdk::{Input, Result, Value, close_ui, json, show_ui};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// Longest label or option the host accepts, and longest note.
const MAX_LABEL: usize = 80;
const MAX_NOTE: usize = 1000;

// ---------------------------------------------------------------------------------------------
// Pure logic
// ---------------------------------------------------------------------------------------------

/// A page setting, as the panel names it: what "Restore defaults" marks, and what an edit unmarks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Field {
    Lang,
    Line,
    FillOpacity,
    Stage,
    Rerun,
}

impl Field {
    /// What "Restore defaults" restores: everything on the page but the language.
    pub const RESTORABLE: [Field; 4] = [Field::Line, Field::FillOpacity, Field::Stage, Field::Rerun];

    /// The page setting a control edits, by the control's name.
    pub fn of_control(name: &str) -> Option<Field> {
        match name {
            "lang" => Some(Field::Lang),
            "line" => Some(Field::Line),
            "fill_opacity" => Some(Field::FillOpacity),
            "stage" => Some(Field::Stage),
            "on_rerun" => Some(Field::Rerun),
            _ => None,
        }
    }
}

/// What the form holds, whatever its language.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub lang: Lang,
    pub line: String,
    /// The fill's opacity as the field holds it (percent). Only a whole number from 0 to 100 is valid; see [`Form::fill_valid`].
    pub fill_pct: f64,
    pub stage: StageMode,
    pub rerun: Rerun,
    pub version: String,
    pub status: String,
}

/// `#RGB`, `#RRGGBB` or `#RRGGBBAA` (the `#` optional) as upper-case `#RRGGBB(AA)`; none when it isn't one.
pub fn hex_norm(text: &str) -> Option<String> {
    let t = text.trim();
    let t = if t.starts_with('#') { t.to_string() } else { format!("#{t}") };
    if !is_hex_colour(&t) {
        return None;
    }
    let hex = t.trim_start_matches('#').to_uppercase();
    let hex = if hex.len() == 3 { hex.chars().flat_map(|c| [c, c]).collect() } else { hex };
    Some(format!("#{hex}"))
}

impl Form {
    /// The form of a page with nothing defined.
    pub fn defaults() -> Form {
        let line = hex_norm(theme::REDLINE).unwrap_or_default();
        Form {
            lang: Lang::En,
            line,
            fill_pct: f64::from(DEFAULT_FILL_OPACITY),
            stage: StageMode::Light,
            rerun: Rerun::Replace,
            version: String::new(),
            status: String::new(),
        }
    }

    /// The form that shows what is stored (Version and Status always start empty).
    pub fn from_stored(s: &Settings) -> Form {
        let mut f = Form::defaults();
        f.lang = s.lang.as_deref().and_then(Lang::strict).unwrap_or_default();
        if let Some(line) = s.line.as_deref().and_then(hex_norm) {
            f.line = line;
        }
        f.fill_pct = f64::from(s.fill_percent());
        f.stage = s.stage_mode();
        f.rerun = s.rerun();
        f
    }

    /// The form the control values say, on top of `base` (a control the panel didn't draw keeps its
    /// value in `base`). Every label is understood in either language, so a language switch loses nothing.
    pub fn from_values(values: &Value, base: &Form) -> Form {
        let mut f = base.clone();
        let text = |name: &str| values.get(name).and_then(Value::as_str);
        if let Some(l) = text("lang").and_then(parse_lang_label) {
            f.lang = l;
        }
        if let Some(c) = text("line").and_then(hex_norm) {
            f.line = c;
        }
        match values.get("fill_opacity") {
            Some(Value::Number(n)) => f.fill_pct = n.as_f64().unwrap_or(f64::NAN),
            Some(Value::String(t)) => f.fill_pct = t.trim().parse().unwrap_or(f64::NAN),
            Some(Value::Null) => f.fill_pct = f64::NAN,
            _ => {}
        }
        if let Some(s) = text("stage").and_then(parse_stage_label) {
            f.stage = s;
        }
        if let Some(r) = text("on_rerun").and_then(parse_rerun_label) {
            f.rerun = r;
        }
        if let Some(v) = text("version") {
            f.version = v.to_string();
        }
        if let Some(v) = text("status") {
            f.status = v.to_string();
        }
        f
    }

    /// The fill opacity as a whole number from 0 to 100, or none when the field holds anything else.
    pub fn fill_valid(&self) -> Option<u8> {
        let n = self.fill_pct;
        (n.is_finite() && n.fract() == 0.0 && (0.0..=100.0).contains(&n)).then_some(n as u8)
    }

    /// The page's part of the form back to "nothing defined", except the language: it stays, so the panel does
    /// not change language under the user's hands.
    pub fn restore_page_defaults(&self) -> Form {
        let d = Form::defaults();
        Form { line: d.line.clone(), fill_pct: d.fill_pct, stage: d.stage, rerun: d.rerun, ..self.clone() }
    }

    /// The form after a documentation was made: nothing is remembered, so Version and Status are empty again.
    pub fn after_generate(&self) -> Form {
        Form { version: String::new(), status: String::new(), ..self.clone() }
    }
}

// ---- labels, in either language ------------------------------------------------------------

fn lang_label(l: Lang) -> &'static str {
    t(l, if l == Lang::Pt { Key::PanelLangPt } else { Key::PanelLangEn })
}

fn parse_lang_label(label: &str) -> Option<Lang> {
    [Lang::En, Lang::Pt].into_iter().find(|l| lang_label(*l) == label)
}

fn stage_key(s: StageMode) -> Key {
    match s {
        StageMode::Light => Key::PanelStageLight,
        StageMode::Dark => Key::PanelStageDark,
        StageMode::Auto => Key::PanelStageAuto,
    }
}

fn parse_stage_label(label: &str) -> Option<StageMode> {
    [StageMode::Light, StageMode::Dark, StageMode::Auto].into_iter().find(|s| [Lang::En, Lang::Pt].iter().any(|l| t(*l, stage_key(*s)) == label))
}

fn rerun_key(r: Rerun) -> Key {
    match r {
        Rerun::Replace => Key::PanelRerunReplace,
        Rerun::New => Key::PanelRerunNew,
    }
}

fn parse_rerun_label(label: &str) -> Option<Rerun> {
    [Rerun::Replace, Rerun::New].into_iter().find(|r| [Lang::En, Lang::Pt].iter().any(|l| t(*l, rerun_key(*r)) == label))
}

/// Text the host accepts in a label, an option or a note: no control characters, trimmed, cut to `max`
/// characters (with an ellipsis).
pub fn safe_text(s: &str, max: usize) -> String {
    let cleaned: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let trimmed = cleaned.trim();
    if trimmed.chars().count() <= max {
        return trimmed.to_string();
    }
    let mut cut: String = trimmed.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

// ---- what a click writes ----------------------------------------------------------------------

/// One stored text of a setting that the form says `want`. `eff` is what the stored text means (none
/// when absent or unreadable); `clear` is a "Restore defaults" not yet applied. A field the user didn't
/// change keeps its stored text exactly, and a key that is absent stays absent when the form shows the
/// default.
fn choose<T: PartialEq>(stored: &Option<String>, eff: Option<T>, clear: bool, want: T, default: T, text: impl Fn(&T) -> String) -> Option<String> {
    if clear {
        return None;
    }
    match eff {
        Some(e) if e == want => stored.clone(),
        Some(_) => Some(text(&want)),
        None if want == default => stored.clone(),
        None => Some(text(&want)),
    }
}

/// The page settings to write for this form, given what is stored; none when nothing differs. `cleared`
/// are the page settings marked by "Restore defaults".
pub fn plan(stored: &Settings, form: &Form, cleared: &BTreeSet<Field>) -> Option<Settings> {
    let s = stored;
    let has = |f: Field| cleared.contains(&f);
    let default_line = hex_norm(theme::REDLINE).unwrap_or_default();
    let line_eff = s.line.as_deref().and_then(hex_norm);
    let new = Settings {
        lang: choose(&s.lang, s.lang.as_deref().and_then(Lang::strict), has(Field::Lang), form.lang, Lang::En, |l| l.code().to_string()),
        line: choose(&s.line, line_eff, has(Field::Line), form.line.clone(), default_line, |c| c.clone()),
        // Only a valid opacity is ever planned; the click refuses the others before it gets here.
        fill_opacity: match form.fill_valid() {
            _ if has(Field::FillOpacity) => None,
            Some(want) if Some(want) == s.fill_opacity => s.fill_opacity,
            Some(want) if s.fill_opacity.is_none() && want == DEFAULT_FILL_OPACITY => None,
            Some(want) => Some(want),
            None => s.fill_opacity,
        },
        on_rerun: choose(&s.on_rerun, s.on_rerun.as_deref().and_then(Rerun::parse), has(Field::Rerun), form.rerun, Rerun::Replace, |r| r.name().to_string()),
        stage: choose(&s.stage, s.stage.as_deref().and_then(StageMode::parse), has(Field::Stage), form.stage, StageMode::Light, |m| m.name().to_string()),
        // A leftover key (a `tz` or a `fill` colour from 0.9.6 and before) is ignored, not deleted; "Restore
        // defaults" (every setting but the language cleared) drops it too.
        other: if Field::RESTORABLE.iter().all(|f| has(*f)) { serde_json::Map::new() } else { s.other.clone() },
    };
    (new != *s).then_some(new)
}

// ---- the selection --------------------------------------------------------------------------------

/// What is wrong with a selection whose first layer has the host type `kind` (none when it is a component,
/// a component set or an instance, the three things a documentation can be made for): the layer type in the
/// panel's language.
pub fn wrong_kind(lang: Lang, kind: &str) -> Option<String> {
    (!matches!(kind, "COMPONENT" | "COMPONENT_SET" | "INSTANCE")).then(|| kind_word(lang, kind))
}

// ---- where the screen goes --------------------------------------------------------------------------

/// The view that shows the start of a frame at (`x`, `y`) that is `w` wide, in a viewport of `vw` × `vh`: as
/// wide as the frame needs (90% of the viewport at most, zoom 100% at most), with the frame's top edge near
/// the top of the screen (3% of the viewport's height below it). Returns (zoom, cx, cy), the centre in page units.
pub fn view_for(vw: f64, vh: f64, x: f64, y: f64, w: f64) -> (f64, f64, f64) {
    let zoom = if w > 0.0 && vw > 0.0 { (0.9 * vw / w).min(1.0) } else { 1.0 };
    let shown = vh / zoom;
    (zoom, x + w / 2.0, y + shown / 2.0 - 0.03 * shown)
}

// ---- the panel's spec ---------------------------------------------------------------------------

/// The symbols that start a note (the host's font has them): done, refused or failed, and information.
const SYM_OK: &str = "✔";
const SYM_ERR: &str = "✖";
const SYM_INFO: &str = "ℹ";

/// A note's text after its symbol.
fn marked(symbol: &str, text: &str) -> String {
    format!("{symbol} {text}")
}

/// What the state note says.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Message {
    /// The panel was just opened (or nothing has happened since).
    #[default]
    Idle,
    /// A documentation was made, for a component with this name.
    Done(String),
    /// Nothing was selected.
    NothingSelected,
    /// The selection is a layer of this kind (in the panel's language), not a component.
    WrongKind(String),
    /// The fill opacity is not a whole number from 0 to 100.
    BadOpacity,
    /// "Restore defaults" was pressed.
    Restored,
}

/// The state note's text, with its symbol: none for what is only information.
pub fn message_text(lang: Lang, m: &Message) -> String {
    match m {
        Message::Idle => t(lang, Key::PanelIdle).to_string(),
        Message::Done(name) => marked(SYM_OK, &tf(lang, Key::PanelGenerated, &[("name", name)])),
        Message::NothingSelected => marked(SYM_ERR, t(lang, Key::PanelNothingSelected)),
        Message::WrongKind(kind) => marked(SYM_ERR, &tf(lang, Key::PanelWrongKind, &[("kind", kind)])),
        Message::BadOpacity => marked(SYM_ERR, t(lang, Key::PanelBadOpacity)),
        Message::Restored => t(lang, Key::PanelRestored).to_string(),
    }
}

fn note(label: &str) -> Value {
    json!({"type": "note", "label": safe_text(label, MAX_NOTE)})
}

fn button(name: &str, label: &str) -> Value {
    json!({"type": "button", "name": name, "label": safe_text(label, MAX_LABEL)})
}

/// The non-breaking spaces that end a label: `LabelPad` in the dictionary says how many (0 when it isn't a number).
fn label_pad(lang: Lang) -> String {
    "\u{a0}".repeat(t(lang, Key::LabelPad).trim().parse::<usize>().unwrap_or(0).min(MAX_LABEL / 2))
}

/// The panel's spec in the language `lang` (the form's own), with every field's value from `form`.
///
/// The order is: the fields, then the buttons, then the messages. The host hands its events to the plugin
/// in the order of these controls, so a button before a field would run before the field's new value
/// arrived. The first button is the primary one: it makes the documentation.
pub fn spec(lang: Lang, form: &Form, message: &Message) -> Value {
    let label = |k: Key| safe_text(t(lang, k), MAX_LABEL);
    let mut c: Vec<Value> = Vec::new();
    // A section's title is a note whose text is in capitals in the dictionary, with an empty note before it as
    // a spacer (except at the top): the host has no section, divider or spacing of its own.
    c.push(note(t(lang, Key::PanelTitlePage)));
    c.push(json!({"type": "select", "name": "lang", "label": label(Key::PanelLang), "options": [lang_label(Lang::En), lang_label(Lang::Pt)], "value": lang_label(form.lang)}));
    // The English "Line colour" gets non-breaking spaces at its end (the dictionary's `LabelPad`) so that the column
    // of labels is as wide in English as in Portuguese. Added after `safe_text`, which would trim them.
    let line_label = format!("{}{}", label(Key::PanelLineColour), label_pad(lang));
    c.push(json!({"type": "color", "name": "line", "label": line_label, "value": form.line}));
    // The field holds a whole number from 0 to 100 (the host clamps what is typed; the plugin checks the rest).
    let shown = if form.fill_pct.is_finite() { form.fill_pct } else { f64::from(DEFAULT_FILL_OPACITY) };
    c.push(json!({"type": "number", "name": "fill_opacity", "label": label(Key::PanelFillOpacity), "min": 0, "max": 100, "value": shown}));
    c.push(note(t(lang, Key::PanelFillHint)));
    let stage: Vec<&str> = [StageMode::Light, StageMode::Dark, StageMode::Auto].iter().map(|s| t(lang, stage_key(*s))).collect();
    c.push(json!({"type": "select", "name": "stage", "label": label(Key::PanelStage), "options": stage, "value": t(lang, stage_key(form.stage))}));
    let rerun: Vec<&str> = [Rerun::Replace, Rerun::New].iter().map(|r| t(lang, rerun_key(*r))).collect();
    c.push(json!({"type": "select", "name": "on_rerun", "label": label(Key::PanelRerun), "options": rerun, "value": t(lang, rerun_key(form.rerun))}));
    c.push(note(""));
    c.push(note(t(lang, Key::PanelTitleDocumentation)));
    c.push(json!({"type": "text", "name": "version", "label": label(Key::PanelVersion), "value": form.version}));
    c.push(json!({"type": "text", "name": "status", "label": label(Key::PanelStatus), "value": form.status}));
    c.push(note(t(lang, Key::PanelBlankHint)));
    c.push(note(""));
    c.push(button("generate", t(lang, Key::PanelGenerate)));
    c.push(button("defaults", t(lang, Key::PanelDefaults)));
    c.push(button("close", t(lang, Key::PanelClose)));
    // The messages are BELOW the buttons: their height changes from one message to the next, and anything under
    // them would move; the buttons stay where they are. The list of controls is the same in every state.
    c.push(note(""));
    c.push(note(t(lang, Key::PanelTitleMessages)));
    c.push(note(&message_text(lang, message)));
    c.push(note(&marked(SYM_INFO, t(lang, Key::PanelFreezeNote))));
    json!({"title": safe_text(t(lang, Key::PanelTitle), 64), "controls": c})
}

// ---------------------------------------------------------------------------------------------
// The glue to the host
// ---------------------------------------------------------------------------------------------

/// What the panel remembers between callbacks (the module stays loaded while the panel is open).
#[derive(Default)]
struct State {
    /// The page settings "Restore defaults" marked and no edit has taken back.
    cleared: BTreeSet<Field>,
    /// What the state note says.
    message: Message,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The current page's id.
fn page_id(lang: Lang) -> Result<u64> {
    let pages = host(lang, "page.list", &json!({}))?;
    let list = pages.as_array().ok_or_else(|| t(lang, Key::ErrNotList).to_string())?;
    list.iter()
        .find(|p| p.get("current").and_then(Value::as_bool).unwrap_or(false))
        .or_else(|| list.first())
        .and_then(|p| p.get("id").and_then(Value::as_u64))
        .ok_or_else(|| t(lang, Key::ErrNoPage).to_string())
}

/// The current page and its settings.
fn read_page(lang: Lang) -> Result<(u64, Settings)> {
    let page = page_id(lang)?;
    let data = host(lang, "node.get", &json!({"id": page, "fields": ["pluginData"]}))?;
    Ok((page, Settings::from_plugin_data(data.get("pluginData").unwrap_or(&Value::Null))))
}

/// Draw the panel.
fn draw(form: &Form, message: &Message) -> Result<()> {
    show_ui(&spec(form.lang, form, message))
}

/// Run from the menu: show the panel with what the page has stored.
pub fn open(_input: Input) -> Result<Value> {
    let (_, settings) = read_page(Lang::En)?;
    let form = Form::from_stored(&settings);
    STATE.with(|s| *s.borrow_mut() = State::default());
    draw(&form, &Message::Idle)?;
    Ok(json!({"phase": "open", "lang": form.lang.code(), "message": message_text(form.lang, &Message::Idle)}))
}

/// A button was pressed or a field changed.
pub fn event(input: Input) -> Result<Value> {
    let Some(ev) = &input.event else { return Ok(Value::Null) };
    let control = ev.control.as_str();
    if control == "close" {
        close_ui();
        return Ok(json!({"phase": "close"}));
    }
    let mut state = STATE.with(|s| std::mem::take(&mut *s.borrow_mut()));
    let out = run_event(&input, control, &mut state);
    STATE.with(|s| *s.borrow_mut() = state);
    out
}

fn run_event(input: &Input, control: &str, state: &mut State) -> Result<Value> {
    let ev_values = input.event.as_ref().map_or(&Value::Null, |e| &e.values);
    // The language of the form is the language of the messages (a switch redraws in the new one).
    let (page, stored) = read_page(Lang::En)?;
    let base = Form::from_stored(&stored);
    let mut form = Form::from_values(ev_values, &base);

    match control {
        "defaults" => {
            state.cleared = Field::RESTORABLE.into_iter().collect();
            form = form.restore_page_defaults();
            state.message = Message::Restored;
        }
        "generate" => return generate_click(input, state, &stored, page, form),
        other => {
            if let Some(f) = Field::of_control(other) {
                state.cleared.remove(&f);
            }
        }
    }
    draw(&form, &state.message)?;
    Ok(json!({"phase": control, "lang": form.lang.code(), "message": message_text(form.lang, &state.message), "selection_now": input.selection}))
}

/// The click on "Generate documentation", all in this callback and in this order: look at the selection and
/// refuse (a note, nothing changed) when it can't be documented or the fill opacity is not a whole number from 0 to 100; write the page's settings; make the
/// documentation; take the screen to its start; give the selection back; redraw with Version and Status
/// empty. An error from any step after the first look is returned as it is: the host puts everything back.
fn generate_click(input: &Input, state: &mut State, stored: &Settings, page: u64, form: Form) -> Result<Value> {
    let lang = form.lang;
    // a) Look before writing anything: the selection, then the fill opacity.
    let refusal = match input.selection.first() {
        None => Some(Message::NothingSelected),
        Some(first) => {
            let node = host(lang, "node.get", &json!({"id": first, "fields": ["type"]}))?;
            let kind = node.get("type").and_then(Value::as_str).unwrap_or_default();
            wrong_kind(lang, kind).map(Message::WrongKind)
        }
    };
    let refusal = refusal.or_else(|| form.fill_valid().is_none().then_some(Message::BadOpacity));
    if let Some(message) = refusal {
        draw(&form, &message)?;
        state.message = message.clone();
        return Ok(json!({"phase": "generate", "refused": true, "message": message_text(lang, &message)}));
    }
    // b) The page's settings, as the form says.
    if let Some(new) = plan(stored, &form, &state.cleared) {
        write_data(lang, page, SETTINGS_KEY, new.to_text())?;
    }
    // c) The documentation, with the two fields.
    let gen_input = Input { params: json!({"version": form.version, "status": form.status}), event: None, ..input.clone() };
    let out = generate(gen_input)?;
    let name = out.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    // d) The start of the documentation on the screen. It only moves the view, so a failure here is told, not fatal.
    let view = show_start(lang, &out);
    // e) The selection as it was before the click.
    host(lang, "select.set", &json!({"ids": input.selection}))?;
    // f) The panel again, with Version and Status empty.
    state.cleared.clear();
    state.message = Message::Done(name);
    let form = form.after_generate();
    draw(&form, &state.message)?;
    Ok(json!({
        "phase": "generate", "refused": false, "message": message_text(lang, &state.message), "frame": out.get("frame"),
        "view": view, "selection": input.selection, "meta": out.get("meta"),
    }))
}

/// Move the view to the start of the frame `out` describes; the view as it is afterwards, or why it didn't move.
fn show_start(lang: Lang, out: &Value) -> Value {
    let go = || -> Result<Value> {
        let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64);
        let view = host(lang, "view.get", &json!({}))?;
        let (vw, vh) = (num(&view, "vw").unwrap_or(0.0), num(&view, "vh").unwrap_or(0.0));
        let (x, y) = (num(out, "x").unwrap_or(0.0), num(out, "y").unwrap_or(0.0));
        let w = out.get("frame_size").and_then(|f| num(f, "w")).unwrap_or(0.0);
        let (zoom, cx, cy) = view_for(vw, vh, x, y, w);
        host(lang, "view.set", &json!({"zoom": zoom, "cx": cx, "cy": cy}))?;
        let after = host(lang, "view.get", &json!({}))?;
        Ok(json!({"zoom": after.get("zoom"), "cx": after.get("cx"), "cy": after.get("cy"), "vw": after.get("vw"), "vh": after.get("vh"), "asked": {"zoom": zoom, "cx": cx, "cy": cy}}))
    };
    go().unwrap_or_else(|e| json!({"error": e}))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn none() -> BTreeSet<Field> {
        BTreeSet::new()
    }

    /// Every state the panel can be drawn in: the messages, both languages, a fill opacity of 0, 30 and 100.
    fn every_state() -> Vec<(Form, Message)> {
        let mut out = Vec::new();
        for lang in [Lang::En, Lang::Pt] {
            for pct in [0.0, 30.0, 100.0] {
                let mut form = Form::defaults();
                form.lang = lang;
                form.fill_pct = pct;
                for m in [
                    Message::Idle,
                    Message::Done("Botão".into()),
                    Message::Done("x".repeat(2000)),
                    Message::NothingSelected,
                    Message::WrongKind(kind_word(lang, "FRAME")),
                    Message::BadOpacity,
                    Message::Restored,
                ] {
                    out.push((form.clone(), m));
                }
            }
        }
        out
    }

    // ---- the spec fits the host ---------------------------------------------------------------

    /// The values the controls of a spec hold, by name (what the host sends back in `values`).
    fn values_of(spec: &Value) -> Value {
        let mut m = serde_json::Map::new();
        for c in spec["controls"].as_array().unwrap() {
            if !matches!(c["type"].as_str(), Some("note" | "button")) {
                m.insert(c["name"].as_str().unwrap().to_string(), c["value"].clone());
            }
        }
        Value::Object(m)
    }

    fn check_spec(spec: &Value) {
        let controls = spec["controls"].as_array().unwrap();
        assert!(controls.len() <= 64, "{} controls", controls.len());
        assert!(spec.to_string().len() < 1024 * 1024);
        assert!(spec["title"].as_str().unwrap().chars().count() <= 64);
        let mut names = BTreeSet::new();
        let first_button = controls.iter().find(|c| c["type"] == "button").unwrap();
        assert_eq!(first_button["name"], "generate", "Generate documentation is the primary button");
        for c in controls {
            let kind = c["type"].as_str().unwrap();
            let label = c["label"].as_str().unwrap_or_default();
            let max = if kind == "note" { MAX_NOTE } else { MAX_LABEL };
            assert!(label.chars().count() <= max, "label too long: {label}");
            assert!(!label.chars().any(char::is_control), "control character in {label:?}");
            assert!(kind == "note" || !label.is_empty(), "a control without a label");
            if kind != "note" {
                let name = c["name"].as_str().unwrap();
                assert!(!name.is_empty() && name.len() <= 64 && name.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_')), "bad name {name}");
                assert!(names.insert(name.to_string()), "two controls called {name}");
            }
            match kind {
                "select" => {
                    let options: Vec<&str> = c["options"].as_array().unwrap().iter().map(|o| o.as_str().unwrap()).collect();
                    assert!(options.len() <= 64 && !options.is_empty());
                    assert!(options.iter().all(|o| o.chars().count() <= MAX_LABEL && !o.chars().any(char::is_control)));
                    assert!(options.contains(&c["value"].as_str().unwrap()), "{} is not one of {options:?}", c["value"]);
                }
                "color" => assert!(is_hex_colour(c["value"].as_str().unwrap())),
                _ => {}
            }
        }
    }

    #[test]
    fn the_spec_fits_what_the_host_accepts_in_every_state_and_both_languages() {
        for (form, message) in every_state() {
            check_spec(&spec(form.lang, &form, &message));
        }
    }

    /// A spec as a list: a note by its text, any other control by its name.
    fn signature(sp: &Value) -> Vec<String> {
        sp["controls"].as_array().unwrap().iter().map(|c| if c["type"] == "note" { format!("note:{}", c["label"].as_str().unwrap()) } else { c["name"].as_str().unwrap().to_string() }).collect()
    }

    #[test]
    fn the_controls_come_as_page_fields_then_documentation_fields_then_buttons_then_messages() {
        let en = signature(&spec(Lang::En, &Form::defaults(), &Message::Idle));
        let nb = "\u{a0}";
        assert_eq!(
            en,
            [
                "note:THIS PAGE", "lang", "line", "fill_opacity", "note:Fill opacity, using the line colour. 0 = no fill.", "stage", "on_rerun", "note:", "note:DOCUMENTATION", "version", "status",
                &format!("note:Left blank, the documentation shows “to{nb}be{nb}defined”."), "note:", "generate", "defaults", "close",
                "note:", "note:MESSAGES", "note:Select a component, a component set or a variant and press Generate documentation.", "note:ℹ The screen freezes while the documentation is generated."
            ]
        );
        let pt = Form { lang: Lang::Pt, ..Form::defaults() };
        let pt = signature(&spec(Lang::Pt, &pt, &Message::Done("Botão de teste".into())));
        assert_eq!(
            pt,
            [
                "note:ESTA PÁGINA", "lang", "line", "fill_opacity", "note:Opacidade do preenchimento, na cor da linha. 0 = sem preenchimento.", "stage", "on_rerun", "note:", "note:DOCUMENTAÇÃO", "version", "status",
                &format!("note:Em branco, a documentação mostra “a{nb}definir”."), "note:", "generate", "defaults", "close",
                "note:", "note:MENSAGENS", "note:✔ Documentação gerada: Botão de teste.", "note:ℹ A tela fica travada enquanto a documentação é gerada."
            ]
        );
    }

    #[test]
    fn the_panel_has_the_same_number_of_controls_in_every_state() {
        for (form, message) in every_state() {
            assert_eq!(spec(form.lang, &form, &message)["controls"].as_array().unwrap().len(), 20);
        }
    }

    #[test]
    fn no_button_comes_before_any_field_in_any_state_of_the_panel() {
        // The host delivers its events in the order of the controls: a button above a field would run
        // before the field's new value arrives, and the first click would be lost.
        const FIELDS: [&str; 7] = ["text", "number", "boolean", "select", "color", "list", "slider"];
        for (form, message) in every_state() {
            let sp = spec(form.lang, &form, &message);
            let kinds: Vec<&str> = sp["controls"].as_array().unwrap().iter().map(|c| c["type"].as_str().unwrap()).collect();
            let first_button = kinds.iter().position(|k| *k == "button").unwrap();
            let last_field = kinds.iter().rposition(|k| FIELDS.contains(k));
            assert!(last_field.is_some_and(|f| f < first_button), "a button comes before a field: {kinds:?}");
            assert_eq!(sp["controls"][first_button]["name"], "generate", "the first button is the host's primary one");
            assert_eq!(kinds.iter().filter(|k| **k == "button").count(), 3);
        }
    }

    #[test]
    fn the_first_control_is_a_title_without_a_spacer_and_every_other_spacer_has_a_reason() {
        for (form, message) in every_state() {
            let sp = spec(form.lang, &form, &message);
            let controls = sp["controls"].as_array().unwrap();
            assert_eq!(controls[0]["type"], "note");
            assert_eq!(controls[0]["label"], t(form.lang, Key::PanelTitlePage));
            let titles = [t(form.lang, Key::PanelTitleDocumentation), t(form.lang, Key::PanelTitleMessages)];
            let empty: Vec<usize> = controls.iter().enumerate().filter(|(_, c)| c["type"] == "note" && c["label"] == "").map(|(i, _)| i).collect();
            assert_eq!(empty.len(), 3, "before DOCUMENTATION, before the buttons and before MESSAGES");
            for i in empty {
                let next = &controls[i + 1];
                let label = next["label"].as_str().unwrap();
                assert!(next["type"] == "button" || titles.contains(&label), "a spacer must come before a title or a button, not before {label:?}");
            }
            for title in [t(form.lang, Key::PanelTitlePage), titles[0], titles[1]] {
                assert_eq!(title, title.to_uppercase());
                assert!(!title.contains('─'));
            }
        }
    }

    #[test]
    fn the_freeze_warning_is_always_there_and_the_last_control() {
        for (form, message) in every_state() {
            let sp = spec(form.lang, &form, &message);
            let controls = sp["controls"].as_array().unwrap();
            let last = controls.last().unwrap();
            assert_eq!(last["type"], "note");
            assert_eq!(last["label"].as_str().unwrap(), marked(SYM_INFO, t(form.lang, Key::PanelFreezeNote)));
            assert!(last["label"].as_str().unwrap().starts_with("ℹ "));
            // The state note is right above it, under the MESSAGES title.
            assert_eq!(controls[controls.len() - 3]["label"], t(form.lang, Key::PanelTitleMessages));
        }
        assert_eq!(marked(SYM_INFO, t(Lang::Pt, Key::PanelFreezeNote)), "ℹ A tela fica travada enquanto a documentação é gerada.");
        assert_eq!(marked(SYM_INFO, t(Lang::En, Key::PanelFreezeNote)), "ℹ The screen freezes while the documentation is generated.");
    }

    #[test]
    fn the_list_of_controls_and_their_labels_is_the_same_in_every_state_and_both_languages() {
        // Only the text of the state note may differ from one state to the next (it is the one note under MESSAGES).
        // The layout then cannot jump: a field, a note or a button never appears, goes away or changes its label.
        let shape = |sp: &Value| -> Vec<(String, String, String)> {
            let controls = sp["controls"].as_array().unwrap();
            let state_note = controls.len() - 2;
            controls
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let label = if i == state_note { "<state note>".to_string() } else { c["label"].as_str().unwrap_or_default().to_string() };
                    (c["type"].as_str().unwrap().to_string(), c["name"].as_str().unwrap_or_default().to_string(), label)
                })
                .collect()
        };
        type Shape = Vec<(String, String, String)>;
        let mut per_lang: Vec<(Lang, Shape)> = Vec::new();
        for lang in [Lang::En, Lang::Pt] {
            let reference = shape(&spec(lang, &Form { lang, ..Form::defaults() }, &Message::Idle));
            // Every message, a custom line colour, other opacities, stages and reruns, long names.
            let mut states: Vec<(Form, Message)> = every_state().into_iter().filter(|(f, _)| f.lang == lang).collect();
            for form in [
                Form { lang, line: "#112233".into(), fill_pct: 0.0, stage: StageMode::Dark, rerun: Rerun::New, version: "1.2".into(), status: "Aprovado".into() },
                Form { lang, fill_pct: f64::NAN, ..Form::defaults() },
                Form { lang, fill_pct: 12.5, ..Form::defaults() },
            ] {
                states.push((form.clone(), Message::Idle));
                states.push((form, Message::BadOpacity));
            }
            assert!(states.len() >= 20);
            for (form, message) in states {
                let sp = spec(lang, &form, &message);
                assert_eq!(shape(&sp), reference, "{lang:?} {message:?} {form:?}");
                assert_eq!(sp["controls"].as_array().unwrap().len(), 20);
            }
            per_lang.push((lang, reference));
        }
        // Across languages: the same controls, in the same order, with the same names.
        let skeleton = |v: &[(String, String, String)]| v.iter().map(|(t, n, _)| (t.clone(), n.clone())).collect::<Vec<_>>();
        assert_eq!(skeleton(&per_lang[0].1), skeleton(&per_lang[1].1));
    }

    #[test]
    fn the_english_line_colour_label_is_padded_to_the_width_of_the_portuguese_column() {
        let line = |lang: Lang| spec(lang, &Form::defaults(), &Message::Idle)["controls"].as_array().unwrap().iter().find(|c| c["name"] == "line").unwrap()["label"].as_str().unwrap().to_string();
        let (en, pt) = (line(Lang::En), line(Lang::Pt));
        assert_eq!(en, format!("Line colour{}", "\u{a0}".repeat(8)));
        assert_eq!(pt, "Cor da linha", "Portuguese needs no padding");
        assert!(en.chars().count() <= MAX_LABEL);
        // The padding is non-breaking spaces and nothing else is padded.
        assert_eq!((t(Lang::En, Key::LabelPad), t(Lang::Pt, Key::LabelPad)), ("8", "0"), "the number of non-breaking spaces, one constant per language");
        assert_eq!(label_pad(Lang::En), "\u{a0}".repeat(8));
        assert_eq!(label_pad(Lang::Pt), "");
        let others: Vec<String> = spec(Lang::En, &Form::defaults(), &Message::Idle)["controls"].as_array().unwrap().iter().filter(|c| c["name"] != "line" && c["type"] != "note").map(|c| c["label"].as_str().unwrap().to_string()).collect();
        assert!(others.iter().all(|l| !l.contains('\u{a0}')));
    }

    #[test]
    fn the_blank_hint_never_breaks_inside_the_expression() {
        assert_eq!(t(Lang::Pt, Key::PanelBlankHint), "Em branco, a documentação mostra “a\u{a0}definir”.");
        assert_eq!(t(Lang::En, Key::PanelBlankHint), "Left blank, the documentation shows “to\u{a0}be\u{a0}defined”.");
        // The note keeps those spaces when it is put in the panel.
        let sp = spec(Lang::Pt, &Form::defaults(), &Message::Idle);
        assert!(sp["controls"].as_array().unwrap().iter().any(|c| c["label"] == "Em branco, a documentação mostra “a\u{a0}definir”."));
    }

    #[test]
    fn the_symbols_mark_only_success_and_refusal_and_the_warning() {
        assert_eq!(message_text(Lang::Pt, &Message::Idle), "Selecione um componente, um conjunto ou uma variante e aperte Gerar documentação.");
        assert_eq!(message_text(Lang::Pt, &Message::Done("Botão".into())), "✔ Documentação gerada: Botão.");
        assert_eq!(message_text(Lang::Pt, &Message::NothingSelected), "✖ Nada selecionado. Selecione um componente, um conjunto ou uma variante e aperte Gerar documentação.");
        assert_eq!(
            message_text(Lang::Pt, &Message::WrongKind("Quadro".into())),
            "✖ A seleção é do tipo Quadro, não um componente, conjunto ou variante. Selecione um deles e aperte Gerar documentação."
        );
        assert_eq!(message_text(Lang::En, &Message::Done("Button".into())), "✔ Documentation generated: Button.");
        assert_eq!(message_text(Lang::En, &Message::NothingSelected), "✖ Nothing is selected. Select a component, a component set or a variant and press Generate documentation.");
        assert_eq!(
            message_text(Lang::En, &Message::WrongKind("Frame".into())),
            "✖ The selection is of type Frame, not a component, a component set or a variant. Select one of them and press Generate documentation."
        );
        for l in [Lang::En, Lang::Pt] {
            assert!(!message_text(l, &Message::Idle).starts_with(['✔', '●', '✖', 'ℹ']), "the opening note has no symbol");
            assert!(!message_text(l, &Message::Restored).starts_with(['✔', '●', '✖', 'ℹ']), "nor has the restore note");
        }
    }

    #[test]
    fn the_title_of_the_panel_and_the_labels_of_the_buttons() {
        let sp = spec(Lang::Pt, &Form::defaults(), &Message::Idle);
        assert_eq!(sp["title"], "Handoff Components: Gerar documentação");
        let buttons: Vec<&str> = sp["controls"].as_array().unwrap().iter().filter(|c| c["type"] == "button").map(|c| c["label"].as_str().unwrap()).collect();
        assert_eq!(buttons, ["Gerar documentação", "Restaurar padrões", "Fechar"]);
        let sp = spec(Lang::En, &Form::defaults(), &Message::Idle);
        let buttons: Vec<&str> = sp["controls"].as_array().unwrap().iter().filter(|c| c["type"] == "button").map(|c| c["label"].as_str().unwrap()).collect();
        assert_eq!(buttons, ["Generate documentation", "Restore defaults", "Close"]);
    }

    #[test]
    fn the_fill_is_a_number_field_from_0_to_100_and_there_is_no_fill_colour() {
        let sp = spec(Lang::Pt, &Form::defaults(), &Message::Idle);
        let c = sp["controls"].as_array().unwrap().iter().find(|c| c["name"] == "fill_opacity").unwrap();
        assert_eq!((c["type"].as_str(), c["label"].as_str(), c["min"].as_i64(), c["max"].as_i64(), c["value"].as_f64()), (Some("number"), Some("Preenchimento (%)"), Some(0), Some(100), Some(30.0)));
        let en = spec(Lang::En, &Form::defaults(), &Message::Idle);
        assert!(en["controls"].as_array().unwrap().iter().any(|c| c["label"] == "Fill (%)"));
        for sp in [sp, en] {
            let names: Vec<&str> = sp["controls"].as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
            assert!(!names.contains(&"fill") && !names.contains(&"fill_auto"));
        }
    }

    #[test]
    fn only_a_whole_number_from_0_to_100_is_a_valid_fill() {
        let with = |n: f64| Form { fill_pct: n, ..Form::defaults() }.fill_valid();
        assert_eq!((with(0.0), with(30.0), with(100.0)), (Some(0), Some(30), Some(100)));
        for bad in [-1.0, 100.5, 12.5, 101.0, f64::NAN, f64::INFINITY] {
            assert_eq!(with(bad), None, "{bad}");
        }
        // What the host sends: a number, or text it coerced; null or a word is not a valid fill.
        let base = Form::defaults();
        for (v, want) in [(json!(45), Some(45)), (json!(45.0), Some(45)), (json!("45"), Some(45)), (json!(12.5), None), (json!(null), None), (json!("abc"), None)] {
            assert_eq!(Form::from_values(&json!({"fill_opacity": v}), &base).fill_valid(), want, "{v}");
        }
        // Not sent at all: the form keeps its value.
        assert_eq!(Form::from_values(&json!({}), &Form { fill_pct: 70.0, ..base }).fill_valid(), Some(70));
    }

    #[test]
    fn an_invalid_opacity_is_shown_as_a_note_and_the_message_is_the_same_length_class() {
        assert_eq!(message_text(Lang::Pt, &Message::BadOpacity), "✖ O preenchimento deve ser um número inteiro de 0 a 100. Corrija e aperte Gerar documentação.");
        assert_eq!(message_text(Lang::En, &Message::BadOpacity), "✖ The fill must be a whole number from 0 to 100. Correct it and press Generate documentation.");
    }

    // ---- Version and Status start empty and come back empty ----------------------------------------

    #[test]
    fn version_and_status_start_empty_and_are_empty_again_after_a_documentation() {
        let stored = Settings::from_plugin_data(&json!({SETTINGS_KEY: r##"{"lang":"pt-BR","line":"#112233"}"##}));
        let opened = Form::from_stored(&stored);
        assert_eq!((opened.version.as_str(), opened.status.as_str()), ("", ""));
        // What the user typed reaches the form...
        let typed = Form::from_values(&json!({"version": "1.2", "status": "Aprovado"}), &opened);
        assert_eq!((typed.version.as_str(), typed.status.as_str()), ("1.2", "Aprovado"));
        // ...and after a documentation both are empty, everything else is kept.
        let after = typed.after_generate();
        assert_eq!((after.version.as_str(), after.status.as_str()), ("", ""));
        assert_eq!(Form { version: String::new(), status: String::new(), ..typed.clone() }, after);
        let values = values_of(&spec(Lang::Pt, &after, &Message::Done("x".into())));
        assert_eq!(values["version"], "");
        assert_eq!(values["status"], "");
        assert_eq!(values["line"], "#112233");
    }

    // ---- the selection and the view ----------------------------------------------------------------

    #[test]
    fn only_a_component_a_set_or_an_instance_can_be_documented() {
        for ok in ["COMPONENT", "COMPONENT_SET", "INSTANCE"] {
            assert_eq!(wrong_kind(Lang::Pt, ok), None, "{ok}");
        }
        assert_eq!(wrong_kind(Lang::Pt, "FRAME").as_deref(), Some("Quadro"));
        assert_eq!(wrong_kind(Lang::En, "FRAME").as_deref(), Some("Frame"));
        assert_eq!(wrong_kind(Lang::Pt, "TEXT").as_deref(), Some("Texto"));
        assert!(wrong_kind(Lang::Pt, "WEIRD_THING").is_some(), "a type the dictionary lacks still refuses, in title case");
    }

    #[test]
    fn the_view_starts_at_the_top_of_the_frame_at_most_at_one_hundred_percent() {
        // A frame wider than the screen: fit it to 90% of the width; the frame's top is 3% of the height below the top.
        let (zoom, cx, cy) = view_for(1200.0, 800.0, 2128.0, 0.0, 1280.0);
        assert!((zoom - 0.84375).abs() < 1e-9, "{zoom}");
        assert!((cx - (2128.0 + 640.0)).abs() < 1e-9);
        let shown = 800.0 / zoom;
        let top_of_screen = cy - shown / 2.0;
        assert!((top_of_screen - (0.0 - 0.03 * shown)).abs() < 1e-9, "the frame's top is 3% of the screen below its top edge");
        // A frame narrower than the screen is not enlarged.
        let (zoom, _, _) = view_for(1200.0, 800.0, 0.0, 0.0, 300.0);
        assert_eq!(zoom, 1.0);
        // Nothing sensible known: zoom 100%.
        assert_eq!(view_for(0.0, 0.0, 0.0, 0.0, 0.0).0, 1.0);
    }

    // ---- the language switch loses nothing --------------------------------------------------

    fn busy_form() -> Form {
        Form {
            lang: Lang::Pt,
            line: "#3B82F6".into(),
            fill_pct: 45.0,
            stage: StageMode::Dark,
            rerun: Rerun::New,
            version: "1.2.0 beta".into(),
            status: "Em revisão".into(),
        }
    }

    #[test]
    fn switching_the_language_redraws_everything_the_user_edited() {
        let base = Form::defaults();
        let pt = busy_form();
        // The values the host would send after the switch to English: the controls as drawn in Portuguese.
        let drawn_pt = spec(Lang::Pt, &pt, &Message::Idle);
        let mut values = values_of(&drawn_pt);
        values["lang"] = json!("English");
        let parsed = Form::from_values(&values, &base);
        assert_eq!(parsed, Form { lang: Lang::En, ..pt.clone() });
        let drawn_en = spec(Lang::En, &parsed, &Message::Idle);
        check_spec(&drawn_en);
        let again = Form::from_values(&values_of(&drawn_en), &base);
        assert_eq!(again, parsed);
        let mut back = values_of(&drawn_en);
        back["lang"] = json!("Português (Brasil)");
        assert_eq!(Form::from_values(&back, &base), pt);
        let ctl = |sp: &Value, n: &str| sp["controls"].as_array().unwrap().iter().find(|c| c["name"] == n).unwrap().clone();
        assert_eq!(ctl(&drawn_pt, "stage")["value"], "Escuro");
        assert_eq!(ctl(&drawn_en, "stage")["value"], "Dark");
        assert_eq!(ctl(&drawn_en, "on_rerun")["value"], "New");
        assert_eq!(ctl(&drawn_en, "version")["value"], "1.2.0 beta");
    }

    #[test]
    fn every_stage_and_rerun_label_is_understood_in_either_language() {
        for l in [Lang::En, Lang::Pt] {
            for s in [StageMode::Light, StageMode::Dark, StageMode::Auto] {
                assert_eq!(parse_stage_label(t(l, stage_key(s))), Some(s));
            }
            for r in [Rerun::Replace, Rerun::New] {
                assert_eq!(parse_rerun_label(t(l, rerun_key(r))), Some(r));
            }
            assert_eq!(parse_lang_label(lang_label(l)), Some(l));
        }
        assert_eq!(parse_stage_label("nope"), None);
        assert_eq!(StageMode::Dark.name(), "dark");
        assert_eq!(Rerun::New.name(), "new");
    }

    // ---- what a click writes -------------------------------------------------------------------

    fn st(text: &str) -> Settings {
        Settings::from_plugin_data(&json!({SETTINGS_KEY: text}))
    }

    #[test]
    fn nothing_changed_writes_nothing_and_a_key_that_is_absent_stays_absent() {
        let s = Settings::default();
        let form = Form::from_stored(&s);
        assert_eq!(plan(&s, &form, &none()), None);
        // The form shows the defaults: choosing them by hand changes nothing either.
        let mut by_hand = Form::defaults();
        by_hand.line = "#FCA311".into();
        by_hand.lang = Lang::En;
        assert_eq!(plan(&s, &by_hand, &none()), None);
        // Version and Status are never stored.
        let typed = Form { version: "1".into(), status: "x".into(), ..form };
        assert_eq!(plan(&s, &typed, &none()), None);
    }

    #[test]
    fn a_field_the_user_did_not_change_keeps_its_stored_text_exactly() {
        let s = st(r##"{"lang":"pt","line":"#3b82f6","fill_opacity":45,"on_rerun":"new","stage":"dark","tz":"+9","fill":"#3B82F633"}"##);
        let form = Form::from_stored(&s);
        assert_eq!(form.lang, Lang::Pt);
        assert_eq!(plan(&s, &form, &none()), None, "pt, lower-case hex and +9 are left as they are");
        let mut changed = form.clone();
        changed.stage = StageMode::Auto;
        let p = plan(&s, &changed, &none()).unwrap();
        assert_eq!(p.to_text().unwrap(), r##"{"fill":"#3B82F633","fill_opacity":45,"lang":"pt","line":"#3b82f6","on_rerun":"new","stage":"auto","tz":"+9"}"##, "a leftover tz and an old fill colour are ignored but kept");
    }

    #[test]
    fn a_changed_field_is_written_in_its_canonical_form() {
        let s = Settings::default();
        let mut f = Form::from_stored(&s);
        f.lang = Lang::Pt;
        f.line = "#112233".into();
        f.stage = StageMode::Dark;
        f.rerun = Rerun::New;
        assert_eq!(plan(&s, &f, &none()).unwrap().to_text().unwrap(), r##"{"lang":"pt-BR","line":"#112233","on_rerun":"new","stage":"dark"}"##);
    }

    #[test]
    fn restore_defaults_keeps_the_language_and_removes_the_other_keys_that_are_stored() {
        let full = st(r##"{"lang":"pt-BR","line":"#112233","fill_opacity":45,"fill":"#11223344","on_rerun":"new","stage":"dark","tz":"5.5"}"##);
        let form = Form::from_stored(&full).restore_page_defaults();
        assert_eq!(form.lang, Lang::Pt, "the language stays, or the panel would turn English under the user");
        assert_eq!((form.fill_valid(), form.stage, form.rerun, form.line.as_str()), (Some(30), StageMode::Light, Rerun::Replace, "#FCA311"));
        let cleared: BTreeSet<Field> = Field::RESTORABLE.into_iter().collect();
        let p = plan(&full, &form, &cleared).unwrap();
        assert_eq!(p.to_text().as_deref(), Some(r##"{"lang":"pt-BR"}"##), "only the language is left: old keys (tz, the old fill colour) are dropped");
        // The language typed in the panel before the restore is the one kept.
        let en = Form { lang: Lang::En, ..form.clone() };
        assert_eq!(plan(&full, &en, &cleared).unwrap().to_text().as_deref(), Some(r##"{"lang":"en"}"##));
        // Nothing stored but a language: nothing to do.
        let only_lang = st(r##"{"lang":"pt-BR"}"##);
        assert_eq!(plan(&only_lang, &Form::from_stored(&only_lang), &cleared), None);
        let explicit = st(r##"{"line":"#FCA311","tz":"-3"}"##);
        let f2 = Form::from_stored(&explicit);
        assert_eq!(plan(&explicit, &f2, &none()), None, "unmarked: untouched");
        assert_eq!(plan(&explicit, &f2.restore_page_defaults(), &cleared).unwrap().to_text(), None);
        let empty = Settings::default();
        assert_eq!(plan(&empty, &Form::defaults(), &cleared), None);
    }

    #[test]
    fn the_fill_opacity_is_written_only_when_it_changes_and_the_default_stays_absent() {
        let none_stored = st(r##"{"line":"#112233"}"##);
        let mut f = Form::from_stored(&none_stored);
        assert_eq!(f.fill_valid(), Some(30));
        assert_eq!(plan(&none_stored, &f, &none()), None, "the default 30 on a page with nothing stored writes nothing");
        f.fill_pct = 50.0;
        assert_eq!(plan(&none_stored, &f, &none()).unwrap().to_text().unwrap(), r##"{"fill_opacity":50,"line":"#112233"}"##);
        f.fill_pct = 0.0;
        assert_eq!(plan(&none_stored, &f, &none()).unwrap().to_text().unwrap(), r##"{"fill_opacity":0,"line":"#112233"}"##, "0 is a value, not 'nothing'");
        // 30 typed by hand where 45 is stored is a change; the same value as stored is not.
        let stored = st(r##"{"fill_opacity":45}"##);
        let mut g = Form::from_stored(&stored);
        assert_eq!(plan(&stored, &g, &none()), None);
        g.fill_pct = 30.0;
        assert_eq!(plan(&stored, &g, &none()).unwrap().to_text().unwrap(), r##"{"fill_opacity":30}"##);
        // An old fill colour is ignored: the form shows 30 and the colour stays in the file.
        let old = st(r##"{"fill":"#3B82F633","line":"#112233"}"##);
        let h = Form::from_stored(&old);
        assert_eq!(h.fill_valid(), Some(30));
        assert_eq!(plan(&old, &h, &none()), None);
        let mut h2 = h.clone();
        h2.stage = StageMode::Dark;
        assert!(plan(&old, &h2, &none()).unwrap().to_text().unwrap().contains(r##""fill":"#3B82F633""##));
        // An invalid field plans no change to the opacity (the click refuses it before).
        let bad = Form { fill_pct: 12.5, ..Form::from_stored(&stored) };
        assert_eq!(plan(&stored, &bad, &none()), None);
    }

    #[test]
    fn a_stored_value_that_cannot_be_read_is_left_alone_unless_restored() {
        let junk = st(r##"{"line":"red","fill_opacity":150,"stage":"loud","tz":"99"}"##);
        let form = Form::from_stored(&junk);
        assert_eq!(form.line, "#FCA311");
        assert_eq!(form.fill_valid(), Some(30), "an opacity over 100 reads as the default");
        assert_eq!(plan(&junk, &form, &none()), None);
        let cleared: BTreeSet<Field> = Field::RESTORABLE.into_iter().collect();
        assert_eq!(plan(&junk, &form, &cleared).unwrap().to_text(), None);
    }

    #[test]
    fn an_edit_takes_back_the_restore_mark_of_its_own_field_only() {
        assert_eq!(Field::of_control("fill_opacity"), Some(Field::FillOpacity));
        assert_eq!(Field::of_control("fill"), None);
        assert_eq!(Field::of_control("on_rerun"), Some(Field::Rerun));
        assert_eq!(Field::of_control("version"), None);
        assert_eq!(Field::of_control("generate"), None);
    }

    // ---- small helpers --------------------------------------------------------------------

    #[test]
    fn hex_and_text_helpers() {
        for (typed, want) in [("#abc", Some("#AABBCC")), ("fca311", Some("#FCA311")), (" #fca31180 ", Some("#FCA31180")), ("red", None), ("#12", None), ("", None)] {
            assert_eq!(hex_norm(typed).as_deref(), want, "{typed:?}");
        }
        assert_eq!(safe_text("  a\nb\tc  ", 80), "a b c");
        assert_eq!(safe_text(&"x".repeat(100), 10), format!("{}…", "x".repeat(9)));
        assert_eq!(safe_text("short", 10), "short");
    }
}
