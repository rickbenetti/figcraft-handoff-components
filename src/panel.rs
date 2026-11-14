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
use crate::store::{Rerun, SETTINGS_KEY, Settings, StageMode};
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
    Stage,
    Rerun,
}

impl Field {
    /// What "Restore defaults" restores: everything on the page but the language.
    pub const RESTORABLE: [Field; 2] = [Field::Stage, Field::Rerun];

    /// The page setting a control edits, by the control's name.
    pub fn of_control(name: &str) -> Option<Field> {
        match name {
            "lang" => Some(Field::Lang),
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
    pub stage: StageMode,
    pub rerun: Rerun,
    pub version: String,
    pub status: String,
}

impl Form {
    /// The form of a page with nothing defined.
    pub fn defaults() -> Form {
        Form {
            lang: Lang::En,
            stage: StageMode::Auto,
            rerun: Rerun::Replace,
            version: String::new(),
            status: String::new(),
        }
    }

    /// The form that shows what is stored (Version and Status always start empty).
    pub fn from_stored(s: &Settings) -> Form {
        let mut f = Form::defaults();
        f.lang = s.lang.as_deref().and_then(Lang::strict).unwrap_or_default();
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

    /// The page's part of the form back to "nothing defined", except the language: it stays, so the panel does
    /// not change language under the user's hands.
    pub fn restore_page_defaults(&self) -> Form {
        let d = Form::defaults();
        Form { stage: d.stage, rerun: d.rerun, ..self.clone() }
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
    let new = Settings {
        lang: choose(&s.lang, s.lang.as_deref().and_then(Lang::strict), has(Field::Lang), form.lang, Lang::En, |l| l.code().to_string()),
        on_rerun: choose(&s.on_rerun, s.on_rerun.as_deref().and_then(Rerun::parse), has(Field::Rerun), form.rerun, Rerun::Replace, |r| r.name().to_string()),
        stage: choose(&s.stage, s.stage.as_deref().and_then(StageMode::parse), has(Field::Stage), form.stage, StageMode::Auto, |m| m.name().to_string()),
        // A leftover key (a `tz`, a `fill` colour, a `line` colour or a `fill_opacity` from earlier versions) is
        // ignored, not deleted; "Restore defaults" (every setting but the language cleared) drops it too.
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
        Message::Restored => t(lang, Key::PanelRestored).to_string(),
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
/// refuse (a note, nothing changed) when it can't be documented; write the page's settings; make the
/// documentation; take the screen to its start; give the selection back; redraw with Version and Status
/// empty. An error from any step after the first look is returned as it is: the host puts everything back.
fn generate_click(input: &Input, state: &mut State, stored: &Settings, page: u64, form: Form) -> Result<Value> {
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

    /// Every state the panel can be drawn in: the messages, both languages, the three stages.
    fn every_state() -> Vec<(Form, Message)> {
        let mut out = Vec::new();
        for lang in [Lang::En, Lang::Pt] {
            for stage in [StageMode::Light, StageMode::Dark, StageMode::Auto] {
                let mut form = Form::defaults();
                form.lang = lang;
                form.stage = stage;
                for m in [
                    Message::Idle,
                    Message::Done("Botão".into()),
                    Message::Done("x".repeat(2000)),
                    Message::NothingSelected,
                    Message::WrongKind(kind_word(lang, "FRAME")),
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
    fn the_controls_come_as_page_fields_then_documentation_fields_then_buttons_then_messages() {
        let en = signature(&spec(Lang::En, &Form::defaults(), &Message::Idle));
        let nb = "\u{a0}";
        assert_eq!(
            en,
            [
                "note:THIS PAGE", "lang", "stage", "on_rerun", "note:", "note:DOCUMENTATION", "version", "status",
                &format!("note:Left blank, the documentation shows “to{nb}be{nb}defined”."), "note:", "generate", "defaults", "close",
                "note:", "note:MESSAGES", "note:Select a component, a component set or a variant and press Generate documentation.", "note:ℹ The screen freezes while the documentation is generated."
            ]
        );
        let pt = Form { lang: Lang::Pt, ..Form::defaults() };
        let pt = signature(&spec(Lang::Pt, &pt, &Message::Done("Botão de teste".into())));
        assert_eq!(
            pt,
            [
                "note:ESTA PÁGINA", "lang", "stage", "on_rerun", "note:", "note:DOCUMENTAÇÃO", "version", "status",
                &format!("note:Em branco, a documentação mostra “a{nb}definir”."), "note:", "generate", "defaults", "close",
                "note:", "note:MENSAGENS", "note:✔ Documentação gerada: Botão de teste.", "note:ℹ A tela fica travada enquanto a documentação é gerada."
            ]
        );
    }

    #[test]
    fn the_panel_has_the_same_number_of_controls_in_every_state() {
        for (form, message) in every_state() {
            assert_eq!(spec(form.lang, &form, &message)["controls"].as_array().unwrap().len(), 17);
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
            // Every message, stages and reruns, long names.
            let mut states: Vec<(Form, Message)> = every_state().into_iter().filter(|(f, _)| f.lang == lang).collect();
            for form in [Form { lang, stage: StageMode::Dark, rerun: Rerun::New, version: "1.2".into(), status: "Aprovado".into() }, Form { lang, stage: StageMode::Light, ..Form::defaults() }] {
                states.push((form.clone(), Message::Idle));
                states.push((form, Message::Restored));
            }
            assert!(states.len() >= 20);
            for (form, message) in states {
                let sp = spec(lang, &form, &message);
                assert_eq!(shape(&sp), reference, "{lang:?} {message:?} {form:?}");
                assert_eq!(sp["controls"].as_array().unwrap().len(), 17);
            }
            per_lang.push((lang, reference));
        }
        // Across languages: the same controls, in the same order, with the same names.
        let skeleton = |v: &[(String, String, String)]| v.iter().map(|(t, n, _)| (t.clone(), n.clone())).collect::<Vec<_>>();
        assert_eq!(skeleton(&per_lang[0].1), skeleton(&per_lang[1].1));
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
    fn the_blank_hint_never_breaks_inside_the_expression() {
        assert_eq!(t(Lang::Pt, Key::PanelBlankHint), "Em branco, a documentação mostra “a\u{a0}definir”.");
        assert_eq!(t(Lang::En, Key::PanelBlankHint), "Left blank, the documentation shows “to\u{a0}be\u{a0}defined”.");
        // The note keeps those spaces when it is put in the panel.
        let sp = spec(Lang::Pt, &Form::defaults(), &Message::Idle);
        assert!(sp["controls"].as_array().unwrap().iter().any(|c| c["label"] == "Em branco, a documentação mostra “a\u{a0}definir”."));
    }

    #[test]
    fn the_symbols_mark_only_success_and_refusal_and_the_warning() {
        assert_eq!(message_text(Lang::Pt, &Message::Idle), "Selecione um componente, um conjunto ou uma variante e clique em Gerar documentação.");
        assert_eq!(message_text(Lang::Pt, &Message::Done("Botão".into())), "✔ Documentação gerada: Botão.");
        assert_eq!(message_text(Lang::Pt, &Message::NothingSelected), "✖ Nada selecionado. Selecione um componente, um conjunto ou uma variante e clique em Gerar documentação.");
        assert_eq!(
            message_text(Lang::Pt, &Message::WrongKind("Quadro".into())),
            "✖ A seleção é do tipo Quadro, não um componente, conjunto ou variante. Selecione um deles e clique em Gerar documentação."
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
    fn there_is_no_colour_and_no_opacity_control_only_language_stage_rerun_version_and_status() {
        // The redline colours are fixed by stage and by section, and the fill of the measures has one opacity.
        for lang in [Lang::En, Lang::Pt] {
            let sp = spec(lang, &Form::defaults(), &Message::Idle);
            let names: Vec<&str> = sp["controls"].as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
            assert_eq!(names, ["lang", "stage", "on_rerun", "version", "status", "generate", "defaults", "close"]);
            assert!(sp["controls"].as_array().unwrap().iter().all(|c| c["type"] != "color" && c["type"] != "number"));
        }
        assert_eq!(Form::defaults().stage, StageMode::Auto, "the stage is automatic unless the page says otherwise");
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
        by_hand.stage = StageMode::Auto;
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
        assert_eq!(plan(&s, &form, &none()), None, "pt and +9 are left as they are, and so are the line colour and the opacity the page still holds");
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
        f.stage = StageMode::Dark;
        f.rerun = Rerun::New;
        assert_eq!(plan(&s, &f, &none()).unwrap().to_text().unwrap(), r##"{"lang":"pt-BR","on_rerun":"new","stage":"dark"}"##);
    }

    #[test]
    fn restore_defaults_keeps_the_language_and_removes_the_other_keys_that_are_stored() {
        let full = st(r##"{"lang":"pt-BR","line":"#112233","fill_opacity":45,"fill":"#11223344","on_rerun":"new","stage":"dark","tz":"5.5"}"##);
        let form = Form::from_stored(&full).restore_page_defaults();
        assert_eq!(form.lang, Lang::Pt, "the language stays, or the panel would turn English under the user");
        assert_eq!((form.stage, form.rerun), (StageMode::Auto, Rerun::Replace));
        let cleared: BTreeSet<Field> = Field::RESTORABLE.into_iter().collect();
        let p = plan(&full, &form, &cleared).unwrap();
        assert_eq!(p.to_text().as_deref(), Some(r##"{"lang":"pt-BR"}"##), "only the language is left: old keys (tz, the old fill colour) are dropped");
        // The language typed in the panel before the restore is the one kept.
        let en = Form { lang: Lang::En, ..form.clone() };
        assert_eq!(plan(&full, &en, &cleared).unwrap().to_text().as_deref(), Some(r##"{"lang":"en"}"##));
        // Nothing stored but a language: nothing to do.
        let only_lang = st(r##"{"lang":"pt-BR"}"##);
        assert_eq!(plan(&only_lang, &Form::from_stored(&only_lang), &cleared), None);
        let explicit = st(r##"{"line":"#FCA311","tz":"-3"}"##); // a leftover line colour is ignored like the time zone
        let f2 = Form::from_stored(&explicit);
        assert_eq!(plan(&explicit, &f2, &none()), None, "unmarked: untouched");
        assert_eq!(plan(&explicit, &f2.restore_page_defaults(), &cleared).unwrap().to_text(), None);
        let empty = Settings::default();
        assert_eq!(plan(&empty, &Form::defaults(), &cleared), None);
    }

    #[test]
    fn a_line_colour_and_an_opacity_stored_by_earlier_versions_are_ignored_and_kept() {
        // They are no longer settings: the form shows nothing of them, a click that changes the stage writes them back as they were.
        let old = st(r##"{"fill":"#3B82F633","line":"#112233","fill_opacity":45}"##);
        let form = Form::from_stored(&old);
        assert_eq!((form.stage, form.rerun), (StageMode::Auto, Rerun::Replace));
        assert_eq!(plan(&old, &form, &none()), None);
        let mut dark = form.clone();
        dark.stage = StageMode::Dark;
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
        assert_eq!(Field::of_control("line"), None);
        assert_eq!(Field::of_control("fill_opacity"), None);
        assert_eq!(Field::of_control("on_rerun"), Some(Field::Rerun));
        assert_eq!(Field::of_control("version"), None);
        assert_eq!(Field::of_control("generate"), None);
    }

    // ---- small helpers --------------------------------------------------------------------

    #[test]
    fn text_helpers() {
        assert_eq!(safe_text("  a\nb\tc  ", 80), "a b c");
        assert_eq!(safe_text(&"x".repeat(100), 10), format!("{}…", "x".repeat(9)));
        assert_eq!(safe_text("short", 10), "short");
    }
}
