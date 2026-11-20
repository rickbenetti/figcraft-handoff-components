//! The "Generate documentation…" panel (plugin API 2): what the menu command shows.
//!
//! The panel is short and its layout never changes: a title and two page settings (language, stage), a title and the choice of what
//! "Generate" updates when the component already has its Section, a short note, four buttons in one row and one message note. The
//! button "Generate" makes the documentation INSIDE its own click, for whatever is selected at that moment, then takes the screen to
//! the Section and selects the component set that now lives in the Component Master. "New Section" makes another Section beside the
//! first. The panel stays open.
//!
//! The first half of this file is pure logic (the form, the panel's spec, what a click would write); the second half is the glue to
//! the host. Texts come from the dictionary.

use crate::direct::write_data;
use crate::gerar::section::{self, Update, Warn};
use crate::gerar::{host, kind_word};
use crate::i18n::{Key, Lang, t, tf};
use crate::store::{SETTINGS_KEY, Settings, StageMode};
use figcraft_plugin_sdk::{Input, Result, Value, close_ui, json, show_ui};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// Longest label or option the host accepts, and longest note.
const MAX_LABEL: usize = 80;
const MAX_NOTE: usize = 1000;
/// The longest name of a component shown in the message note (longer ones are cut with an ellipsis).
const MAX_NAME: usize = 24;

// ---------------------------------------------------------------------------------------------
// Pure logic
// ---------------------------------------------------------------------------------------------

/// A page setting, as the panel names it: what "Restore defaults" marks, and what an edit unmarks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Field {
    Lang,
    Stage,
}

impl Field {
    /// What "Restore defaults" restores: both page settings.
    pub const RESTORABLE: [Field; 2] = [Field::Lang, Field::Stage];

    /// The page setting a control edits, by the control's name.
    pub fn of_control(name: &str) -> Option<Field> {
        match name {
            "lang" => Some(Field::Lang),
            "stage" => Some(Field::Stage),
            _ => None,
        }
    }
}

/// What the form holds, whatever its language.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub lang: Lang,
    pub stage: StageMode,
    /// What a rerun replaces (not remembered: it is "all" each time the panel opens).
    pub update: Update,
}

impl Form {
    /// The form of a page with nothing defined.
    pub fn defaults() -> Form {
        Form { lang: Lang::En, stage: StageMode::Auto, update: Update::All }
    }

    /// The form that shows what is stored.
    pub fn from_stored(s: &Settings) -> Form {
        let mut f = Form::defaults();
        f.lang = s.lang.as_deref().and_then(Lang::strict).unwrap_or_default();
        f.stage = s.stage_mode();
        f
    }

    /// The form the control values say, on top of `base` (a control the panel didn't draw keeps its value in `base`). Every
    /// label is understood in either language, so a language switch loses nothing.
    pub fn from_values(values: &Value, base: &Form) -> Form {
        let mut f = base.clone();
        let text = |name: &str| values.get(name).and_then(Value::as_str);
        if let Some(l) = text("lang").and_then(parse_lang_label) {
            f.lang = l;
        }
        if let Some(s) = text("stage").and_then(parse_stage_label) {
            f.stage = s;
        }
        if let Some(u) = text("update").and_then(parse_update_label) {
            f.update = u;
        }
        f
    }

    /// Language, stage and update back to the defaults.
    pub fn restore_defaults(&self) -> Form {
        Form::defaults()
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

fn parse_update_label(label: &str) -> Option<Update> {
    Update::ALL.into_iter().find(|u| [Lang::En, Lang::Pt].iter().any(|l| t(*l, u.label_key()) == label))
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
/// are the page settings marked by "Restore defaults". The setting "on rerun" of 0.10.0 has no control any more: what is stored
/// stays as it is.
pub fn plan(stored: &Settings, form: &Form, cleared: &BTreeSet<Field>) -> Option<Settings> {
    let s = stored;
    let has = |f: Field| cleared.contains(&f);
    let new = Settings {
        lang: choose(&s.lang, s.lang.as_deref().and_then(Lang::strict), has(Field::Lang), form.lang, Lang::En, |l| l.code().to_string()),
        on_rerun: s.on_rerun.clone(),
        stage: choose(&s.stage, s.stage.as_deref().and_then(StageMode::parse), has(Field::Stage), form.stage, StageMode::Auto, |m| m.name().to_string()),
        // A leftover key (a `tz`, a `fill` colour, a `line` colour or a `fill_opacity` from earlier versions) is
        // ignored, not deleted; "Restore defaults" (both settings cleared) drops it too.
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

/// What the message note says.
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
    /// "Restore defaults" was pressed.
    Restored,
    /// A documentation was made, and the person should know something (the first of the warnings is shown).
    Warned(String, Vec<Warn>),
}

/// The message note's text, with its symbol: none for what is only information. It is meant to fit in three lines.
pub fn message_text(lang: Lang, m: &Message) -> String {
    let done = |name: &str| marked(SYM_OK, &tf(lang, Key::PanelGenerated, &[("name", &safe_text(name, MAX_NAME))]));
    match m {
        Message::Idle => t(lang, Key::PanelIdle).to_string(),
        Message::Done(name) => done(name),
        Message::NothingSelected => marked(SYM_ERR, t(lang, Key::PanelNothingSelected)),
        Message::WrongKind(kind) => marked(SYM_ERR, &tf(lang, Key::PanelWrongKind, &[("kind", kind)])),
        Message::Restored => t(lang, Key::PanelRestored).to_string(),
        Message::Warned(name, warns) => match warns.first() {
            Some(w) => format!("{} {}", done(name), marked(SYM_INFO, &w.line(lang))),
            None => done(name),
        },
    }
}

fn note(label: &str) -> Value {
    json!({"type": "note", "label": safe_text(label, MAX_NOTE)})
}

fn button(name: &str, label: &str) -> Value {
    json!({"type": "button", "name": name, "label": safe_text(label, MAX_LABEL)})
}

/// The panel's spec in the language `lang` (the form's own), with every field's value from `form`.
///
/// The order is: the fields, then the buttons, then the message. The host hands its events to the plugin in the order of these
/// controls, so a button before a field would run before the field's new value arrived. The first button is the primary one: it
/// makes the documentation. There are eleven controls in every state, and only the text of the last one changes.
pub fn spec(lang: Lang, form: &Form, message: &Message) -> Value {
    let label = |k: Key| safe_text(t(lang, k), MAX_LABEL);
    let mut c: Vec<Value> = Vec::new();
    c.push(note(t(lang, Key::PanelTitlePage)));
    c.push(json!({"type": "select", "name": "lang", "label": label(Key::PanelLang), "options": [lang_label(Lang::En), lang_label(Lang::Pt)], "value": lang_label(form.lang)}));
    let stage: Vec<&str> = [StageMode::Auto, StageMode::Light, StageMode::Dark].iter().map(|s| t(lang, stage_key(*s))).collect();
    c.push(json!({"type": "select", "name": "stage", "label": label(Key::PanelStage), "options": stage, "value": t(lang, stage_key(form.stage))}));
    c.push(note(t(lang, Key::PanelTitleUpdate)));
    let update: Vec<&str> = Update::ALL.iter().map(|u| t(lang, u.label_key())).collect();
    c.push(json!({"type": "select", "name": "update", "label": label(Key::PanelUpdate), "options": update, "value": t(lang, form.update.label_key())}));
    c.push(note(t(lang, Key::PanelUpdateNote)));
    c.push(button("generate", t(lang, Key::PanelGenerate)));
    c.push(button("new_section", t(lang, Key::PanelNewSection)));
    c.push(button("defaults", t(lang, Key::PanelDefaults)));
    c.push(button("close", t(lang, Key::PanelClose)));
    // The message is BELOW the buttons: its height changes from one message to the next, and anything under it would move.
    c.push(note(&message_text(lang, message)));
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
    /// What the message note says.
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
            form = form.restore_defaults();
            state.message = Message::Restored;
        }
        "generate" => return generate_click(input, state, &stored, page, form, false),
        "new_section" => return generate_click(input, state, &stored, page, form, true),
        other => {
            if let Some(f) = Field::of_control(other) {
                state.cleared.remove(&f);
            }
        }
    }
    draw(&form, &state.message)?;
    Ok(json!({"phase": control, "lang": form.lang.code(), "message": message_text(form.lang, &state.message), "selection_now": input.selection}))
}

/// The click on "Generate" or "New Section", all in this callback and in this order: look at the selection and refuse (a note,
/// nothing changed) when it can't be documented; write the page's settings; make the documentation; take the screen to the Section;
/// select the component set (it is in the Component Master now). An error from any step after the first look is returned as it
/// is: the host puts everything back.
fn generate_click(input: &Input, state: &mut State, stored: &Settings, page: u64, form: Form, new_section: bool) -> Result<Value> {
    let lang = form.lang;
    // a) Look before writing anything: the selection.
    let refusal = match input.selection.first() {
        None => Some(Message::NothingSelected),
        Some(first) => {
            let node = host(lang, "node.get", &json!({"id": first, "fields": ["type"]}))?;
            let kind = node.get("type").and_then(Value::as_str).unwrap_or_default();
            wrong_kind(lang, kind).map(Message::WrongKind)
        }
    };
    if let Some(message) = refusal {
        draw(&form, &message)?;
        state.message = message.clone();
        return Ok(json!({"phase": "generate", "refused": true, "message": message_text(lang, &message)}));
    }
    // b) The page's settings, as the form says.
    if let Some(new) = plan(stored, &form, &state.cleared) {
        write_data(lang, page, SETTINGS_KEY, new.to_text())?;
    }
    // c) The documentation.
    #[allow(unused_mut)]
    let mut params = json!({"update": form.update.key(), "new_section": new_section});
    // A test build passes the hook of the run that opened the panel on to the documentation.
    #[cfg(feature = "test-hooks")]
    if let (Some(p), Some(f)) = (params.as_object_mut(), input.params.get("fail_table")) {
        p.insert("fail_table".into(), f.clone());
    }
    let gen_input = Input { params, event: None, ..input.clone() };
    let out = section::generate(gen_input)?;
    let name = out.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    // d) The start of the Section on the screen. It only moves the view, so a failure here is told, not fatal.
    let view = show_start(lang, &out);
    // e) The component set (or the component) that lives in the Component Master now, not the frame.
    let holder = out.get("holder").and_then(Value::as_u64);
    match holder {
        Some(h) => host(lang, "select.set", &json!({"ids": [h]}))?,
        None => host(lang, "select.set", &json!({"ids": input.selection}))?,
    };
    // f) The panel again.
    state.cleared.clear();
    let dropped = out.get("anatomy").and_then(|a| a.get("dropped")).and_then(Value::as_array).map_or(0, Vec::len);
    let warns: Vec<Warn> = out
        .get("warnings")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).filter_map(|c| Warn::ALL.into_iter().find(|w| w.code() == c)).map(|w| if matches!(w, Warn::MarkersDropped(_)) { Warn::MarkersDropped(dropped) } else { w }).collect())
        .unwrap_or_default();
    state.message = if warns.is_empty() { Message::Done(name) } else { Message::Warned(name, warns) };
    draw(&form, &state.message)?;
    Ok(json!({
        "phase": "generate", "refused": false, "message": message_text(lang, &state.message), "frame": out.get("frame"),
        "view": view, "selected": holder, "new_section": out.get("new_section"), "update": out.get("update"), "frames": out.get("frames"), "warnings": out.get("warnings"), "stage": out.get("stage"),
    }))
}

/// Move the view to the start of the frame `out` describes; the view as it is afterwards, or why it didn't move.
fn show_start(lang: Lang, out: &Value) -> Value {
    let go = || -> Result<Value> {
        let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64);
        let view = host(lang, "view.get", &json!({}))?;
        let (vw, vh) = (num(&view, "vw").unwrap_or(0.0), num(&view, "vh").unwrap_or(0.0));
        let (x, y) = (num(out, "x").unwrap_or(0.0), num(out, "y").unwrap_or(0.0));
        let w = num(out, "view_w").or_else(|| out.get("frame_size").and_then(|f| num(f, "w"))).unwrap_or(0.0);
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

    /// Every state the panel can be drawn in: the messages, both languages, the three stages, every choice of update.
    fn every_state() -> Vec<(Form, Message)> {
        let mut out = Vec::new();
        for lang in [Lang::En, Lang::Pt] {
            for stage in [StageMode::Light, StageMode::Dark, StageMode::Auto] {
                for update in Update::ALL {
                    let form = Form { lang, stage, update };
                    for m in [
                        Message::Idle,
                        Message::Done("Botão".into()),
                        Message::Done("x".repeat(2000)),
                        Message::NothingSelected,
                        Message::WrongKind(kind_word(lang, "FRAME")),
                        Message::WrongKind(kind_word(lang, "BOOLEAN_OPERATION")),
                        Message::Restored,
                        Message::Warned("Botão".into(), vec![Warn::NoStage, Warn::Failed, Warn::LegacyDoc]),
                        Message::Warned("y".repeat(2000), vec![Warn::Failed]),
                        Message::Warned("Botão".into(), vec![Warn::LegacyDoc]),
                        Message::Warned("Botão".into(), vec![Warn::MarkersDropped(3)]),
                        Message::Warned("y".repeat(2000), vec![Warn::Failed, Warn::NoStage]),
                    ] {
                        out.push((form.clone(), m));
                    }
                }
            }
        }
        out
    }

    /// The number of controls of the panel: 2 titles, 3 selects, 1 note, 4 buttons and the message.
    const CONTROLS: usize = 11;

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
        assert_eq!(first_button["name"], "generate", "Generate is the primary button");
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
            if kind == "select" {
                let options: Vec<&str> = c["options"].as_array().unwrap().iter().map(|o| o.as_str().unwrap()).collect();
                assert!(options.len() <= 64 && !options.is_empty());
                assert!(options.iter().all(|o| o.chars().count() <= MAX_LABEL && !o.chars().any(char::is_control)));
                assert!(options.contains(&c["value"].as_str().unwrap()), "{} is not one of {options:?}", c["value"]);
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
    fn the_controls_are_the_page_fields_the_update_choice_the_buttons_and_one_message() {
        assert_eq!(
            signature(&spec(Lang::En, &Form::defaults(), &Message::Idle)),
            [
                "note:THIS PAGE", "lang", "stage", "note:UPDATE", "update", "note:Applies to a Section that already exists; the first time, every frame is made. The Master is always remade.",
                "generate", "new_section", "defaults", "close", "note:Select a component, set or variant and press Generate."
            ]
        );
        let pt = Form { lang: Lang::Pt, ..Form::defaults() };
        assert_eq!(
            signature(&spec(Lang::Pt, &pt, &Message::Done("Botão de teste".into()))),
            [
                "note:ESTA PÁGINA", "lang", "stage", "note:ATUALIZAR", "update", "note:Vale para uma Section que já existe; na primeira vez todos os quadros são criados. O Master é sempre refeito.",
                "generate", "new_section", "defaults", "close", "note:✔ Documentação gerada: Botão de teste."
            ]
        );
    }

    #[test]
    fn the_panel_has_at_most_twelve_controls_and_the_same_number_in_every_state() {
        for (form, message) in every_state() {
            let n = spec(form.lang, &form, &message)["controls"].as_array().unwrap().len();
            assert_eq!(n, CONTROLS);
            assert!(n <= 12, "{n} controls");
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
            assert_eq!(kinds.iter().filter(|k| **k == "button").count(), 4);
            // The four buttons are consecutive: the host draws them in one row.
            assert!(kinds[first_button..first_button + 4].iter().all(|k| *k == "button"));
        }
    }

    #[test]
    fn the_titles_are_in_capitals_and_the_panel_has_no_spacer() {
        for (form, message) in every_state() {
            let sp = spec(form.lang, &form, &message);
            let controls = sp["controls"].as_array().unwrap();
            assert_eq!(controls[0]["label"], t(form.lang, Key::PanelTitlePage));
            assert_eq!(controls[3]["label"], t(form.lang, Key::PanelTitleUpdate));
            for title in [t(form.lang, Key::PanelTitlePage), t(form.lang, Key::PanelTitleUpdate)] {
                assert_eq!(title, title.to_uppercase());
            }
            assert!(controls.iter().all(|c| c["type"] != "note" || c["label"] != ""), "no empty note used as a spacer");
        }
    }

    #[test]
    fn the_message_is_the_last_control_and_the_only_one_that_changes() {
        // The layout cannot jump: a field, a note or a button never appears, goes away or changes its label; only the text of
        // the last note (the message) differs from one state to the next.
        let shape = |sp: &Value| -> Vec<(String, String, String)> {
            let controls = sp["controls"].as_array().unwrap();
            let last = controls.len() - 1;
            controls
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let label = if i == last { "<message>".to_string() } else { c["label"].as_str().unwrap_or_default().to_string() };
                    (c["type"].as_str().unwrap().to_string(), c["name"].as_str().unwrap_or_default().to_string(), label)
                })
                .collect()
        };
        type Shape = Vec<(String, String, String)>;
        let mut per_lang: Vec<Shape> = Vec::new();
        for lang in [Lang::En, Lang::Pt] {
            let reference = shape(&spec(lang, &Form { lang, ..Form::defaults() }, &Message::Idle));
            let mut n = 0;
            for (form, message) in every_state().into_iter().filter(|(f, _)| f.lang == lang) {
                let sp = spec(lang, &form, &message);
                assert_eq!(shape(&sp), reference, "{lang:?} {message:?} {form:?}");
                assert_eq!(sp["controls"].as_array().unwrap().len(), CONTROLS);
                assert_eq!(sp["controls"].as_array().unwrap().last().unwrap()["type"], "note");
                n += 1;
            }
            assert!(n >= 100, "{n} states");
            per_lang.push(reference);
        }
        // Across languages: the same controls, in the same order, with the same names.
        let skeleton = |v: &[(String, String, String)]| v.iter().map(|(t, n, _)| (t.clone(), n.clone())).collect::<Vec<_>>();
        assert_eq!(skeleton(&per_lang[0]), skeleton(&per_lang[1]));
    }

    // ---- the height of the panel ------------------------------------------------------------------

    /// The estimated height of a panel, in points, by this formula (the host's egui window: 320 px wide, no scroll):
    /// - a select, text or number field is a row of 24;
    /// - a note is a row of 13 per line, with about 39 characters to a line (11 px font in a 220 px column), at least one line;
    /// - consecutive buttons share ONE row of 24;
    /// - 8 between rows; and about 68 of window chrome (title bar and margins).
    fn estimated_height(sp: &Value) -> f64 {
        const FIELD: f64 = 24.0;
        const LINE: f64 = 13.0;
        const CHARS: usize = 39;
        const ROW_GAP: f64 = 8.0;
        const CHROME: f64 = 68.0;
        let mut rows: Vec<f64> = Vec::new();
        let mut previous_button = false;
        for c in sp["controls"].as_array().unwrap() {
            match c["type"].as_str().unwrap() {
                "button" => {
                    if !previous_button {
                        rows.push(FIELD);
                    }
                    previous_button = true;
                }
                "note" => {
                    let lines = c["label"].as_str().unwrap().chars().count().div_ceil(CHARS).max(1);
                    rows.push(LINE * lines as f64);
                    previous_button = false;
                }
                _ => {
                    rows.push(FIELD);
                    previous_button = false;
                }
            }
        }
        CHROME + rows.iter().sum::<f64>() + ROW_GAP * (rows.len() as f64 - 1.0)
    }

    #[test]
    fn the_panel_is_never_taller_than_450_points_by_the_formula() {
        let mut worst: f64 = 0.0;
        for (form, message) in every_state() {
            let h = estimated_height(&spec(form.lang, &form, &message));
            worst = worst.max(h);
            assert!(h <= 450.0, "{h} pt for {message:?}");
        }
        // Usable heights, minus about 52 for the menu bar and the title: 1366×768 at 125% is 614 → 562, 1920×1080 at 125% is 864 → 812
        // and at 150% is 720 → 668. The panel fits all three with room.
        assert!(worst <= 562.0);
    }

    #[test]
    fn the_note_under_update_takes_at_most_three_lines() {
        for lang in [Lang::En, Lang::Pt] {
            let n = t(lang, Key::PanelUpdateNote).chars().count();
            assert!(n <= 3 * 39, "{n} characters: {}", t(lang, Key::PanelUpdateNote));
        }
    }

    #[test]
    fn the_message_never_takes_more_than_three_lines() {
        for (form, message) in every_state() {
            let text = message_text(form.lang, &message);
            assert!(text.chars().count() <= 120, "{} characters: {text}", text.chars().count());
        }
    }

    #[test]
    fn the_labels_are_plain_text() {
        for lang in [Lang::En, Lang::Pt] {
            for c in spec(lang, &Form::defaults(), &Message::Idle)["controls"].as_array().unwrap() {
                if c["type"] != "note" {
                    let l = c["label"].as_str().unwrap();
                    assert_eq!(l, l.trim(), "no label ends or starts with a space: {l:?}");
                    assert!(!l.contains('\u{a0}'), "{l:?}");
                }
            }
        }
    }

    #[test]
    fn the_symbols_mark_only_success_and_refusal_and_the_warning() {
        assert_eq!(message_text(Lang::Pt, &Message::Idle), "Selecione um componente, conjunto ou variante e clique em Gerar.");
        assert_eq!(message_text(Lang::Pt, &Message::Done("Botão".into())), "✔ Documentação gerada: Botão.");
        assert_eq!(message_text(Lang::Pt, &Message::NothingSelected), "✖ Nada selecionado. Selecione um componente, conjunto ou variante e clique em Gerar.");
        assert_eq!(message_text(Lang::Pt, &Message::WrongKind("Quadro".into())), "✖ A seleção é do tipo Quadro. Selecione um componente, conjunto ou variante.");
        assert_eq!(message_text(Lang::En, &Message::Done("Button".into())), "✔ Documentation generated: Button.");
        assert_eq!(message_text(Lang::En, &Message::NothingSelected), "✖ Nothing is selected. Select a component, set or variant and press Generate.");
        assert_eq!(message_text(Lang::En, &Message::WrongKind("Frame".into())), "✖ The selection is a Frame. Select a component, set or variant.");
        assert_eq!(message_text(Lang::Pt, &Message::Warned("Botão".into(), vec![Warn::NoStage, Warn::LegacyDoc])), "✔ Documentação gerada: Botão. ℹ Nenhum palco serve a todas as variantes: troque em Palco.");
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
        assert_eq!(buttons, ["Gerar", "Nova Section", "Restaurar padrões", "Fechar"]);
        let sp = spec(Lang::En, &Form::defaults(), &Message::Idle);
        let buttons: Vec<&str> = sp["controls"].as_array().unwrap().iter().filter(|c| c["type"] == "button").map(|c| c["label"].as_str().unwrap()).collect();
        assert_eq!(buttons, ["Generate", "New Section", "Restore defaults", "Close"]);
    }

    #[test]
    fn the_fields_are_language_stage_and_update_with_seven_options_for_the_update() {
        for lang in [Lang::En, Lang::Pt] {
            let sp = spec(lang, &Form::defaults(), &Message::Idle);
            let names: Vec<&str> = sp["controls"].as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
            assert_eq!(names, ["lang", "stage", "update", "generate", "new_section", "defaults", "close"]);
            let update = sp["controls"].as_array().unwrap().iter().find(|c| c["name"] == "update").unwrap();
            assert_eq!(update["options"].as_array().unwrap().len(), 7);
            assert_eq!(update["value"], t(lang, Key::UpdAll), "all sections by default");
            let stage = sp["controls"].as_array().unwrap().iter().find(|c| c["name"] == "stage").unwrap();
            let options: Vec<&str> = stage["options"].as_array().unwrap().iter().map(|o| o.as_str().unwrap()).collect();
            assert_eq!(options, [t(lang, Key::PanelStageAuto), t(lang, Key::PanelStageLight), t(lang, Key::PanelStageDark)]);
        }
        let pt = spec(Lang::Pt, &Form::defaults(), &Message::Idle);
        let update = pt["controls"].as_array().unwrap().iter().find(|c| c["name"] == "update").unwrap();
        let options: Vec<&str> = update["options"].as_array().unwrap().iter().map(|o| o.as_str().unwrap()).collect();
        assert_eq!(options, ["Todas as seções", "Só tabelas", "Só Tokens", "Só Propriedades", "Só Anatomia", "Só Estados e variações", "Só Layouts"]);
        assert_eq!(Form::defaults().stage, StageMode::Auto, "the stage is automatic unless the page says otherwise");
        assert_eq!(Form::defaults().update, Update::All);
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

    #[test]
    fn switching_the_language_redraws_everything_the_user_edited() {
        let base = Form::defaults();
        let pt = Form { lang: Lang::Pt, stage: StageMode::Dark, update: Update::Tokens };
        // The values the host would send after the switch to English: the controls as drawn in Portuguese.
        let drawn_pt = spec(Lang::Pt, &pt, &Message::Idle);
        let mut values = values_of(&drawn_pt);
        values["lang"] = json!("English");
        let parsed = Form::from_values(&values, &base);
        assert_eq!(parsed, Form { lang: Lang::En, ..pt.clone() });
        let drawn_en = spec(Lang::En, &parsed, &Message::Idle);
        check_spec(&drawn_en);
        assert_eq!(Form::from_values(&values_of(&drawn_en), &base), parsed);
        let mut back = values_of(&drawn_en);
        back["lang"] = json!("Português (Brasil)");
        assert_eq!(Form::from_values(&back, &base), pt);
        let ctl = |sp: &Value, n: &str| sp["controls"].as_array().unwrap().iter().find(|c| c["name"] == n).unwrap().clone();
        assert_eq!(ctl(&drawn_pt, "stage")["value"], "Escuro");
        assert_eq!(ctl(&drawn_en, "stage")["value"], "Dark");
        assert_eq!(ctl(&drawn_pt, "update")["value"], "Só Tokens");
        assert_eq!(ctl(&drawn_en, "update")["value"], "Tokens only");
    }

    #[test]
    fn every_stage_and_update_label_is_understood_in_either_language() {
        for l in [Lang::En, Lang::Pt] {
            for s in [StageMode::Light, StageMode::Dark, StageMode::Auto] {
                assert_eq!(parse_stage_label(t(l, stage_key(s))), Some(s));
            }
            for u in Update::ALL {
                assert_eq!(parse_update_label(t(l, u.label_key())), Some(u));
            }
            assert_eq!(parse_lang_label(lang_label(l)), Some(l));
        }
        assert_eq!(parse_stage_label("nope"), None);
        assert_eq!(parse_update_label("nope"), None);
        assert_eq!(StageMode::Dark.name(), "dark");
    }

    #[test]
    fn restore_defaults_resets_language_stage_and_update() {
        let busy = Form { lang: Lang::Pt, stage: StageMode::Dark, update: Update::Layouts };
        assert_eq!(busy.restore_defaults(), Form::defaults());
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
        let by_hand = Form { lang: Lang::En, stage: StageMode::Auto, update: Update::Tokens };
        assert_eq!(plan(&s, &by_hand, &none()), None, "the update choice is never stored");
    }

    #[test]
    fn a_field_the_user_did_not_change_keeps_its_stored_text_exactly() {
        let s = st(r##"{"lang":"pt","line":"#3b82f6","fill_opacity":45,"on_rerun":"new","stage":"dark","tz":"+9","fill":"#3B82F633"}"##);
        let form = Form::from_stored(&s);
        assert_eq!(form.lang, Lang::Pt);
        assert_eq!(plan(&s, &form, &none()), None, "pt and +9 are left as they are, and so are the line colour, the opacity and the old on_rerun");
        let changed = Form { stage: StageMode::Auto, ..form.clone() };
        let p = plan(&s, &changed, &none()).unwrap();
        assert_eq!(p.to_text().unwrap(), r##"{"fill":"#3B82F633","fill_opacity":45,"lang":"pt","line":"#3b82f6","on_rerun":"new","stage":"auto","tz":"+9"}"##, "a leftover tz, an old fill colour and the old on_rerun are ignored but kept");
    }

    #[test]
    fn a_changed_field_is_written_in_its_canonical_form() {
        let s = Settings::default();
        let f = Form { lang: Lang::Pt, stage: StageMode::Dark, update: Update::All };
        assert_eq!(plan(&s, &f, &none()).unwrap().to_text().unwrap(), r##"{"lang":"pt-BR","stage":"dark"}"##);
    }

    #[test]
    fn restore_defaults_removes_the_language_and_the_other_keys_that_are_stored() {
        let full = st(r##"{"lang":"pt-BR","line":"#112233","fill_opacity":45,"fill":"#11223344","on_rerun":"new","stage":"dark","tz":"5.5"}"##);
        let form = Form::from_stored(&full).restore_defaults();
        assert_eq!((form.lang, form.stage), (Lang::En, StageMode::Auto));
        let cleared: BTreeSet<Field> = Field::RESTORABLE.into_iter().collect();
        let p = plan(&full, &form, &cleared).unwrap();
        // The language and the stage are cleared with the old keys (tz, the old colours); the old on_rerun is not a setting any more and stays.
        assert_eq!(p.to_text().as_deref(), Some(r##"{"on_rerun":"new"}"##));
        // Nothing stored but a language: restoring removes it.
        let only_lang = st(r##"{"lang":"pt-BR"}"##);
        assert_eq!(plan(&only_lang, &form, &cleared).unwrap().to_text(), None);
        let empty = Settings::default();
        assert_eq!(plan(&empty, &Form::defaults(), &cleared), None);
    }

    #[test]
    fn a_line_colour_and_an_opacity_stored_by_earlier_versions_are_ignored_and_kept() {
        let old = st(r##"{"fill":"#3B82F633","line":"#112233","fill_opacity":45}"##);
        let form = Form::from_stored(&old);
        assert_eq!((form.stage, form.update), (StageMode::Auto, Update::All));
        assert_eq!(plan(&old, &form, &none()), None);
        let dark = Form { stage: StageMode::Dark, ..form };
        let text = plan(&old, &dark, &none()).unwrap().to_text().unwrap();
        assert_eq!(text, r##"{"fill":"#3B82F633","fill_opacity":45,"line":"#112233","stage":"dark"}"##);
    }

    #[test]
    fn a_stored_value_that_cannot_be_read_is_left_alone_unless_restored() {
        let junk = st(r##"{"line":"red","fill_opacity":150,"stage":"loud","tz":"99"}"##);
        let form = Form::from_stored(&junk);
        assert_eq!(form.stage, StageMode::Auto, "a stage that cannot be read is the default");
        assert_eq!(plan(&junk, &form, &none()), None);
        let cleared: BTreeSet<Field> = Field::RESTORABLE.into_iter().collect();
        assert_eq!(plan(&junk, &form, &cleared).unwrap().to_text(), None);
    }

    #[test]
    fn an_edit_takes_back_the_restore_mark_of_its_own_field_only() {
        assert_eq!(Field::of_control("stage"), Some(Field::Stage));
        assert_eq!(Field::of_control("lang"), Some(Field::Lang));
        assert_eq!(Field::of_control("update"), None);
        assert_eq!(Field::of_control("on_rerun"), None);
        assert_eq!(Field::of_control("generate"), None);
    }

    // ---- small helpers --------------------------------------------------------------------

    #[test]
    fn text_helpers() {
        assert_eq!(safe_text("  a\nb\tc  ", 80), "a b c");
        assert_eq!(safe_text(&"x".repeat(100), 10), format!("{}…", "x".repeat(9)));
        assert_eq!(safe_text("short", 10), "short");
    }

    #[test]
    fn the_panel_says_how_many_markers_did_not_fit_and_nothing_when_none_did() {
        let one = message_text(Lang::En, &Message::Warned("Button".into(), vec![Warn::MarkersDropped(1)]));
        assert_eq!(one, "✔ Documentation generated: Button. ℹ 1 marker did not fit in the drawing; see the Note column.");
        let three = message_text(Lang::Pt, &Message::Warned("Botão".into(), vec![Warn::MarkersDropped(3)]));
        assert_eq!(three, "✔ Documentação gerada: Botão. ℹ 3 marcadores não couberam no desenho; veja a coluna Nota.");
        assert_eq!(message_text(Lang::Pt, &Message::Done("Botão".into())), "✔ Documentação gerada: Botão.");
    }
}
