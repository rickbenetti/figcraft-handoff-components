//! The Settings panel (plugin API 2): what the "Settings…" command shows when it is run from the menu.
//!
//! The page's settings (language, line and fill colours, stage, what to do on a rerun, time zone) and
//! the selected component's sheet (version, status, default variant) are edited in one panel. Nothing
//! is written until "Save changes"; every callback compares the form with what is STORED (it is read
//! again each time) to say whether there are unsaved changes.
//!
//! The first half of this file is pure logic (the form, the panel's spec, what a save would write);
//! the second half is the glue to the host. Texts come from the dictionary.

use crate::gerar::{fill_colour, host, load_set, resolve};
use crate::i18n::{Key, Lang, t, tf};
use crate::settings::write_data;
use crate::store::{META_KEY, Meta, Rerun, SETTINGS_KEY, Settings, StageMode, is_hex_colour};
use crate::theme;
use crate::variants::{self, Pairs, SetInfo};
use figcraft_plugin_sdk::{Input, Result, Value, close_ui, json, show_ui};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// Longest label or option the host accepts, longest note, and most variant properties shown (the host
/// takes 64 controls in all; the rest of the panel needs fewer than 25).
const MAX_LABEL: usize = 80;
const MAX_NOTE: usize = 1000;
const MAX_PROPS_SHOWN: usize = 24;
/// Most options a menu may have.
const MAX_OPTIONS: usize = 64;

// ---------------------------------------------------------------------------------------------
// Pure logic
// ---------------------------------------------------------------------------------------------

/// A page setting, as the panel names it: what "Restore defaults" marks, and what an edit unmarks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Field {
    Lang,
    Line,
    Fill,
    Stage,
    Rerun,
    Tz,
}

impl Field {
    pub const PAGE: [Field; 6] = [Field::Lang, Field::Line, Field::Fill, Field::Stage, Field::Rerun, Field::Tz];

    /// The page setting a control edits, by the control's name.
    pub fn of_control(name: &str) -> Option<Field> {
        match name {
            "lang" => Some(Field::Lang),
            "line" => Some(Field::Line),
            "fill_auto" | "fill" => Some(Field::Fill),
            "stage" => Some(Field::Stage),
            "on_rerun" => Some(Field::Rerun),
            "tz" => Some(Field::Tz),
            _ => None,
        }
    }
}

/// What kind of layer the component's sheet is kept on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Component,
    Set,
    /// A variant: its set holds the sheet.
    Variant,
}

/// The component whose sheet the panel edits (loaded when it opens and on Reload).
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    /// The layer that holds the sheet (the set for a variant or a set).
    pub holder: u64,
    pub kind: TargetKind,
    pub title: String,
    /// The set, for a variant or a set: its properties are the default variant's menus.
    pub set: Option<SetInfo>,
}

/// What is stored now: the page's settings and, with a target, its sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Stored {
    pub settings: Settings,
    pub meta: Option<Meta>,
}

/// What the form holds, whatever its language.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub lang: Lang,
    pub line: String,
    pub fill_auto: bool,
    pub fill: String,
    pub stage: StageMode,
    pub rerun: Rerun,
    pub tz: f64,
    pub version: String,
    pub status: String,
    /// The default variant chosen: the pairs that are not "(none)", in the set's property order.
    pub default: Pairs,
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

fn norm_tz(hours: f64) -> f64 {
    if hours == 0.0 { 0.0 } else { hours }
}

impl Form {
    /// The form of a page with nothing defined and no component.
    pub fn defaults() -> Form {
        let line = hex_norm(theme::REDLINE).unwrap_or_default();
        Form {
            lang: Lang::En,
            fill: fill_colour(None, &line).to_uppercase(),
            line,
            fill_auto: true,
            stage: StageMode::Light,
            rerun: Rerun::Replace,
            tz: crate::clock::DEFAULT_TZ,
            version: String::new(),
            status: String::new(),
            default: Pairs::new(),
        }
    }

    /// The pairs of a stored default variant that the set still has, in the set's property order.
    pub fn default_pairs(stored: Option<&str>, set: Option<&SetInfo>) -> Pairs {
        let (Some(text), Some(info)) = (stored, set) else { return Pairs::new() };
        let Ok(pairs) = variants::parse_default(text) else { return Pairs::new() };
        info.props.iter().filter_map(|(p, opts)| pairs.iter().find(|(k, v)| k == p && opts.contains(v)).cloned()).collect()
    }

    /// The form that shows what is stored.
    pub fn from_stored(stored: &Stored, target: Option<&Target>) -> Form {
        let mut f = Form::defaults();
        let s = &stored.settings;
        f.lang = s.lang.as_deref().and_then(Lang::strict).unwrap_or_default();
        if let Some(line) = s.line.as_deref().and_then(hex_norm) {
            f.line = line;
        }
        match s.fill.as_deref().and_then(hex_norm) {
            Some(fill) => {
                f.fill_auto = false;
                f.fill = fill;
            }
            None => f.fill = fill_colour(None, &f.line).to_uppercase(),
        }
        f.stage = s.stage_mode();
        f.rerun = s.rerun();
        f.tz = norm_tz(s.tz_hours());
        if let (Some(meta), Some(target)) = (&stored.meta, target) {
            f.version = meta.version.clone().unwrap_or_default();
            f.status = meta.status.clone().unwrap_or_default();
            f.default = Form::default_pairs(meta.default.as_deref(), target.set.as_ref());
        }
        f
    }

    /// The form the control values say, on top of `base` (a control the panel didn't draw keeps its
    /// value in `base`). Every label is understood in either language, so a language switch loses nothing.
    pub fn from_values(values: &Value, target: Option<&Target>, base: &Form) -> Form {
        let mut f = base.clone();
        let text = |name: &str| values.get(name).and_then(Value::as_str);
        if let Some(l) = text("lang").and_then(parse_lang_label) {
            f.lang = l;
        }
        if let Some(c) = text("line").and_then(hex_norm) {
            f.line = c;
        }
        if let Some(b) = values.get("fill_auto").and_then(Value::as_bool) {
            f.fill_auto = b;
        }
        if let Some(c) = text("fill").and_then(hex_norm) {
            f.fill = c;
        }
        if let Some(s) = text("stage").and_then(parse_stage_label) {
            f.stage = s;
        }
        if let Some(r) = text("on_rerun").and_then(parse_rerun_label) {
            f.rerun = r;
        }
        if let Some(h) = values.get("tz").and_then(Value::as_f64).filter(|h| h.is_finite()) {
            f.tz = norm_tz(h.clamp(-14.0, 14.0));
        }
        if target.is_some() {
            if let Some(v) = text("version") {
                f.version = v.to_string();
            }
            if let Some(v) = text("status") {
                f.status = v.to_string();
            }
            if let Some(info) = target.and_then(|t| t.set.as_ref()) {
                f.default = info
                    .props
                    .iter()
                    .take(MAX_PROPS_SHOWN)
                    .enumerate()
                    .filter_map(|(i, (prop, opts))| {
                        let label = text(&format!("dv_{i}"))?;
                        if is_none_label(label) {
                            return None;
                        }
                        opts.iter().take(MAX_OPTIONS - 1).find(|o| safe_text(o, MAX_LABEL) == label).map(|o| (prop.clone(), o.clone()))
                    })
                    .collect();
                // Properties past the ones shown keep what was stored.
                if info.props.len() > MAX_PROPS_SHOWN {
                    let kept = Form::default_pairs(base_default_text(base).as_deref(), Some(info));
                    f.default.extend(kept.into_iter().filter(|(k, _)| info.props.iter().skip(MAX_PROPS_SHOWN).any(|(p, _)| p == k)));
                }
            }
        }
        f
    }

    /// The page's part of the form back to "nothing defined".
    pub fn restore_page_defaults(&self) -> Form {
        let d = Form::defaults();
        Form { lang: d.lang, line: d.line.clone(), fill_auto: true, fill: fill_colour(None, &d.line).to_uppercase(), stage: d.stage, rerun: d.rerun, tz: d.tz, ..self.clone() }
    }
}

fn base_default_text(base: &Form) -> Option<String> {
    (!base.default.is_empty()).then(|| variants::join(&base.default))
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

fn is_none_label(label: &str) -> bool {
    [Lang::En, Lang::Pt].iter().any(|l| t(*l, Key::PanelNone) == label)
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

// ---- what a save writes -----------------------------------------------------------------------

/// Why a save can't go ahead.
#[derive(Clone, Debug, PartialEq)]
pub enum PlanError {
    /// No variant has the combination of the default variant menus.
    NoSuchVariant(String),
}

/// What a save would write: the new page settings and/or sheet, each present only when it differs
/// from what is stored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub settings: Option<Settings>,
    pub meta: Option<Meta>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.settings.is_none() && self.meta.is_none()
    }
}

/// One stored text of a setting that the form says `want`. `eff` is what the stored text means (none
/// when absent or unreadable); `clear` is a "Restore defaults" not yet saved. A field the user didn't
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

/// What saving the form would change, given what is stored. `cleared` are the page settings marked by
/// "Restore defaults".
pub fn plan(stored: &Stored, form: &Form, cleared: &BTreeSet<Field>, target: Option<&Target>) -> std::result::Result<Plan, PlanError> {
    let s = &stored.settings;
    let has = |f: Field| cleared.contains(&f);
    let default_line = hex_norm(theme::REDLINE).unwrap_or_default();
    let line_eff = s.line.as_deref().and_then(hex_norm);
    let fill_eff = s.fill.as_deref().and_then(hex_norm);
    let new = Settings {
        lang: choose(&s.lang, s.lang.as_deref().and_then(Lang::strict), has(Field::Lang), form.lang, Lang::En, |l| l.code().to_string()),
        line: choose(&s.line, line_eff, has(Field::Line), form.line.clone(), default_line, |c| c.clone()),
        fill: if has(Field::Fill) {
            None
        } else if form.fill_auto {
            if fill_eff.is_some() { None } else { s.fill.clone() }
        } else if fill_eff.as_deref() == Some(form.fill.as_str()) {
            s.fill.clone()
        } else {
            Some(form.fill.clone())
        },
        on_rerun: choose(&s.on_rerun, s.on_rerun.as_deref().and_then(Rerun::parse), has(Field::Rerun), form.rerun, Rerun::Replace, |r| r.name().to_string()),
        stage: choose(&s.stage, s.stage.as_deref().and_then(StageMode::parse), has(Field::Stage), form.stage, StageMode::Light, |m| m.name().to_string()),
        tz: choose(&s.tz, s.tz.as_deref().and_then(crate::clock::parse_tz), has(Field::Tz), form.tz, crate::clock::DEFAULT_TZ, |h| crate::clock::tz_text(*h)),
    };
    let mut out = Plan { settings: (new != *s).then_some(new), meta: None };
    if let (Some(meta), Some(target)) = (&stored.meta, target) {
        let keep = |raw: &Option<String>, want: &str| -> Option<String> {
            let want = want.trim();
            let before = raw.as_deref().map(str::trim).filter(|v| !v.is_empty());
            if before == (!want.is_empty()).then_some(want) { raw.clone() } else { (!want.is_empty()).then(|| want.to_string()) }
        };
        let mut next = Meta { version: keep(&meta.version, &form.version), status: keep(&meta.status, &form.status), default: meta.default.clone(), other: meta.other.clone() };
        if let Some(info) = &target.set {
            let before = Form::default_pairs(meta.default.as_deref(), Some(info));
            let (mut a, mut b) = (before.clone(), form.default.clone());
            a.sort();
            b.sort();
            // Properties past the ones the panel shows are left alone, so they are not compared here.
            if a != b {
                next.default = if form.default.is_empty() {
                    None
                } else {
                    variants::complete(info, &form.default).map_err(|e| match e {
                        variants::DefaultError::NoSuchVariant(name) => PlanError::NoSuchVariant(name),
                        other => PlanError::NoSuchVariant(format!("{other:?}")),
                    })?;
                    Some(variants::join(&form.default))
                };
            }
        }
        out.meta = (next != *meta).then_some(next);
    }
    Ok(out)
}

// ---- the panel's spec ---------------------------------------------------------------------------

/// What the panel says besides the fields.
#[derive(Clone, Debug, Default)]
pub struct Notes {
    pub dirty: bool,
    /// The result of the last button (a save, a refusal, a reload…).
    pub result: Option<String>,
    /// The selection is not the one the panel was loaded with.
    pub moved: bool,
    /// Where the values came from.
    pub loaded_from: String,
}

fn note(label: &str) -> Value {
    json!({"type": "note", "label": safe_text(label, MAX_NOTE)})
}

fn button(name: &str, label: &str) -> Value {
    json!({"type": "button", "name": name, "label": safe_text(label, MAX_LABEL)})
}

/// The panel's spec in the language `lang` (the form's own), with every field's value from `form`.
pub fn spec(lang: Lang, form: &Form, target: Option<&Target>, notes: &Notes) -> Value {
    let label = |k: Key| safe_text(t(lang, k), MAX_LABEL);
    let mut c: Vec<Value> = Vec::new();
    c.push(note(t(lang, if notes.dirty { Key::PanelUnsaved } else { Key::PanelAllSaved })));
    if let Some(r) = &notes.result {
        c.push(note(r));
    }
    if notes.moved {
        c.push(note(t(lang, Key::PanelSelectionMoved)));
    }
    c.push(button("save", t(lang, Key::PanelSave)));
    c.push(button("reload", t(lang, Key::PanelReload)));
    // A section's note looks like a title (its text is decorated in the dictionary) and has an empty
    // note before it as a spacer: the host has no section, divider or spacing of its own.
    c.push(note(""));
    c.push(note(t(lang, Key::PanelPageHeading)));
    c.push(json!({"type": "select", "name": "lang", "label": label(Key::PanelLang), "options": [lang_label(Lang::En), lang_label(Lang::Pt)], "value": lang_label(form.lang)}));
    c.push(json!({"type": "color", "name": "line", "label": label(Key::PanelLineColour), "value": form.line}));
    c.push(json!({"type": "boolean", "name": "fill_auto", "label": label(Key::PanelFillAuto), "value": form.fill_auto}));
    if !form.fill_auto {
        c.push(json!({"type": "color", "name": "fill", "label": label(Key::PanelFillColour), "value": form.fill}));
    }
    let stage: Vec<&str> = [StageMode::Light, StageMode::Dark, StageMode::Auto].iter().map(|s| t(lang, stage_key(*s))).collect();
    c.push(json!({"type": "select", "name": "stage", "label": label(Key::PanelStage), "options": stage, "value": t(lang, stage_key(form.stage))}));
    let rerun: Vec<&str> = [Rerun::Replace, Rerun::New].iter().map(|r| t(lang, rerun_key(*r))).collect();
    c.push(json!({"type": "select", "name": "on_rerun", "label": label(Key::PanelRerun), "options": rerun, "value": t(lang, rerun_key(form.rerun))}));
    c.push(json!({"type": "number", "name": "tz", "label": label(Key::PanelTz), "min": -14, "max": 14, "value": form.tz}));
    c.push(note(t(lang, Key::PanelTzNote)));
    match target {
        None => c.push(note(t(lang, Key::PanelNoTarget))),
        Some(tg) => {
            let heading = match tg.kind {
                TargetKind::Component => Key::PanelTargetComponent,
                TargetKind::Set => Key::PanelTargetSet,
                TargetKind::Variant => Key::PanelTargetVariant,
            };
            c.push(note(""));
            c.push(note(&tf(lang, heading, &[("name", &tg.title)])));
            c.push(json!({"type": "text", "name": "version", "label": label(Key::PanelVersion), "value": form.version}));
            c.push(json!({"type": "text", "name": "status", "label": label(Key::PanelStatus), "value": form.status}));
            if let Some(info) = &tg.set {
                let none = t(lang, Key::PanelNone);
                for (i, (prop, opts)) in info.props.iter().take(MAX_PROPS_SHOWN).enumerate() {
                    let mut options = vec![none.to_string()];
                    options.extend(opts.iter().take(MAX_OPTIONS - 1).map(|o| safe_text(o, MAX_LABEL)));
                    let chosen = form.default.iter().find(|(k, _)| k == prop).map(|(_, v)| safe_text(v, MAX_LABEL)).unwrap_or_else(|| none.to_string());
                    let text = tf(lang, Key::PanelDefaultVariant, &[("prop", prop)]);
                    c.push(json!({"type": "select", "name": format!("dv_{i}"), "label": safe_text(&text, MAX_LABEL), "options": options, "value": chosen}));
                }
                if info.props.len() > MAX_PROPS_SHOWN {
                    c.push(note(&tf(lang, Key::PanelMoreProps, &[("n", &(info.props.len() - MAX_PROPS_SHOWN).to_string())])));
                }
            }
        }
    }
    c.push(note(&notes.loaded_from));
    c.push(button("defaults", t(lang, Key::PanelDefaults)));
    c.push(button("close", t(lang, Key::PanelClose)));
    json!({"title": safe_text(t(lang, Key::PanelTitle), 64), "controls": c})
}

/// Where the values shown came from, in words.
pub fn loaded_from(lang: Lang, stored: &Stored, target: Option<&Target>) -> String {
    let defaults = t(lang, Key::PanelSourceDefaults);
    let page = if stored.settings == Settings::default() { defaults.to_string() } else { SETTINGS_KEY.to_string() };
    match (target, &stored.meta) {
        (Some(_), Some(meta)) => {
            let component = if *meta == Meta::default() { defaults.to_string() } else { META_KEY.to_string() };
            tf(lang, Key::PanelLoadedBoth, &[("page", &page), ("component", &component)])
        }
        _ => tf(lang, Key::PanelLoadedPage, &[("page", &page)]),
    }
}

// ---------------------------------------------------------------------------------------------
// The glue to the host
// ---------------------------------------------------------------------------------------------

/// What the panel remembers between callbacks (the module stays loaded while the panel is open).
#[derive(Default)]
struct State {
    target: Option<Target>,
    /// The selection the panel was loaded with.
    loaded_selection: Vec<u64>,
    /// The page settings "Restore defaults" marked and no edit has taken back.
    cleared: BTreeSet<Field>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

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

/// What is stored now, read again: the current page and the target's sheet.
fn read_stored(lang: Lang, target: Option<&Target>) -> Result<(u64, Stored)> {
    let (page, settings) = read_page(lang)?;
    let meta = match target {
        Some(tg) => {
            let d = host(lang, "node.get", &json!({"id": tg.holder, "fields": ["pluginData"]}))?;
            Some(Meta::from_plugin_data(d.get("pluginData").unwrap_or(&Value::Null)))
        }
        None => None,
    };
    Ok((page, Stored { settings, meta }))
}

/// The component a selection stands for, when it stands for one.
fn target_of(lang: Lang, selection: &[u64]) -> Option<Target> {
    if selection.is_empty() {
        return None;
    }
    let p = resolve(lang, selection).ok()?;
    let kind = if p.set.is_some() {
        TargetKind::Set
    } else if p.holder != p.component {
        TargetKind::Variant
    } else {
        TargetKind::Component
    };
    let set = match (&p.set, kind) {
        (Some(s), _) => Some(s.info.clone()),
        (None, TargetKind::Variant) => load_set(lang, p.holder).ok().map(|(info, _)| info),
        _ => None,
    };
    let title = set.as_ref().map(|i| i.name.clone()).filter(|n| !n.is_empty()).unwrap_or(p.title);
    Some(Target { holder: p.holder, kind, title, set })
}

/// Draw the panel and describe the callback for the caller (scripts and tests read it).
fn draw(lang: Lang, form: &Form, target: Option<&Target>, notes: &Notes, phase: &str, extra: Value) -> Result<Value> {
    show_ui(&spec(lang, form, target, notes))?;
    Ok(json!({"phase": phase, "lang": lang.code(), "dirty": notes.dirty, "result": notes.result, "target": target.map(|t| t.holder), "extra": extra}))
}

/// Run from the menu: load the page's settings and the selected component's sheet, and show the panel.
pub fn open(input: Input) -> Result<Value> {
    let page_lang = read_page(Lang::En)?.1.lang.as_deref().and_then(Lang::strict).unwrap_or_default();
    let target = target_of(page_lang, &input.selection);
    let (_, stored) = read_stored(page_lang, target.as_ref())?;
    let form = Form::from_stored(&stored, target.as_ref());
    STATE.with(|s| *s.borrow_mut() = State { target: target.clone(), loaded_selection: input.selection.clone(), cleared: BTreeSet::new() });
    let notes = Notes { dirty: false, result: None, moved: false, loaded_from: loaded_from(form.lang, &stored, target.as_ref()) };
    draw(form.lang, &form, target.as_ref(), &notes, "open", Value::Null)
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
    let (_, stored) = read_stored(Lang::En, state.target.as_ref())?;
    let base = Form::from_stored(&stored, state.target.as_ref());
    let mut form = Form::from_values(ev_values, state.target.as_ref(), &base);
    let lang = form.lang;
    let mut result: Option<String> = None;
    let mut saved = false;
    let mut wrote = (false, false);

    match control {
        "reload" => {
            state.target = target_of(lang, &input.selection);
            state.loaded_selection = input.selection.clone();
            state.cleared.clear();
            let (_, fresh) = read_stored(lang, state.target.as_ref())?;
            form = Form::from_stored(&fresh, state.target.as_ref());
            result = Some(t(form.lang, Key::PanelReloaded).to_string());
            return finish(input, &form, state, &fresh, result, "reload", false, wrote);
        }
        "defaults" => {
            state.cleared = Field::PAGE.into_iter().collect();
            form = form.restore_page_defaults();
            result = Some(t(form.lang, Key::PanelRestored).to_string());
        }
        "save" => {
            let (done, note, w) = save(input, state, &stored, &form)?;
            saved = done;
            wrote = w;
            result = Some(note);
            if done {
                state.cleared.clear();
                // What was saved is what is stored now: draw from it.
                let (_, fresh) = read_stored(lang, state.target.as_ref())?;
                form = Form::from_values(ev_values, state.target.as_ref(), &Form::from_stored(&fresh, state.target.as_ref()));
                return finish(input, &form, state, &fresh, result, "save", true, wrote);
            }
        }
        other => {
            if let Some(f) = Field::of_control(other) {
                state.cleared.remove(&f);
            }
        }
    }
    finish(input, &form, state, &stored, result, control, saved, wrote)
}

/// Redraw with the notes for the current form against what is stored.
#[allow(clippy::too_many_arguments)]
fn finish(input: &Input, form: &Form, state: &State, stored: &Stored, result: Option<String>, phase: &str, saved: bool, wrote: (bool, bool)) -> Result<Value> {
    let dirty = plan(stored, form, &state.cleared, state.target.as_ref()).map_or(true, |p| !p.is_empty());
    let moved = input.selection != state.loaded_selection;
    let notes = Notes { dirty, result, moved, loaded_from: loaded_from(form.lang, stored, state.target.as_ref()) };
    let mut out = draw(form.lang, form, state.target.as_ref(), &notes, phase, json!({"saved": saved, "wrote_page": wrote.0, "wrote_component": wrote.1}))?;
    out["selection_loaded"] = json!(state.loaded_selection);
    Ok(out)
}

/// Save: check the target, validate, write what changed. Returns (saved, the note, (wrote page, wrote component)).
fn save(input: &Input, state: &mut State, stored: &Stored, form: &Form) -> Result<(bool, String, (bool, bool))> {
    let lang = form.lang;
    if let Some(tg) = &state.target
        && input.selection != state.loaded_selection
    {
        match resolve(lang, &input.selection) {
            Ok(p) if p.holder == tg.holder => state.loaded_selection = input.selection.clone(),
            _ => return Ok((false, t(lang, Key::PanelSelectionRefused).to_string(), (false, false))),
        }
    }
    let plan = match plan(stored, form, &state.cleared, state.target.as_ref()) {
        Ok(p) => p,
        Err(PlanError::NoSuchVariant(name)) => return Ok((false, tf(lang, Key::ErrDefaultNoVariant, &[("name", &name)]), (false, false))),
    };
    if plan.is_empty() {
        return Ok((false, t(lang, Key::SettingsKept).to_string(), (false, false)));
    }
    let mut wrote = (false, false);
    if let Some(new) = &plan.settings {
        let page = page_id(lang)?;
        write_data(lang, page, SETTINGS_KEY, new.to_text())?;
        wrote.0 = true;
    }
    if let (Some(next), Some(tg)) = (&plan.meta, &state.target) {
        write_data(lang, tg.holder, META_KEY, next.to_text())?;
        wrote.1 = true;
    }
    Ok((true, t(lang, Key::SettingsSaved).to_string(), wrote))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::variants::Variant;
    use std::collections::BTreeMap;

    fn variant(id: u64, name: &str) -> Variant {
        Variant { id, name: name.into(), w: 10.0, h: 10.0 }
    }

    fn set_info(extra_props: usize) -> SetInfo {
        let mut names = ["Size=Small, State=Enabled".to_string(), "Size=Small, State=Hover".to_string(), "Size=Large, State=Enabled".to_string(), "Size=Large, State=Hover".to_string()];
        // Extra properties with one option each (to go past the panel's cap).
        for n in names.iter_mut() {
            for i in 0..extra_props {
                n.push_str(&format!(", P{i}=x"));
            }
        }
        SetInfo::new(7, "Button".into(), names.iter().enumerate().map(|(i, n)| variant(i as u64, n)).collect())
    }

    fn set_target() -> Target {
        Target { holder: 7, kind: TargetKind::Set, title: "Button".into(), set: Some(set_info(0)) }
    }

    fn component_target() -> Target {
        Target { holder: 3, kind: TargetKind::Component, title: "Card".into(), set: None }
    }

    fn stored(settings: Settings, meta: Option<Meta>) -> Stored {
        Stored { settings, meta }
    }

    fn no_meta_stored() -> Stored {
        stored(Settings::default(), None)
    }

    fn none() -> BTreeSet<Field> {
        BTreeSet::new()
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
        assert_eq!(first_button["name"], "save", "Save is the primary button");
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
                "number" => assert!((-14.0..=14.0).contains(&c["value"].as_f64().unwrap())),
                _ => {}
            }
        }
    }

    fn all_targets() -> Vec<Option<Target>> {
        let variant = Target { holder: 7, kind: TargetKind::Variant, title: "Button".into(), set: Some(set_info(0)) };
        let wide = Target { holder: 7, kind: TargetKind::Set, title: "x".repeat(300) + "\n", set: Some(set_info(30)) };
        vec![None, Some(component_target()), Some(set_target()), Some(variant), Some(wide)]
    }

    #[test]
    fn the_spec_fits_what_the_host_accepts_in_both_languages_for_every_kind_of_target() {
        for lang in [Lang::En, Lang::Pt] {
            for target in all_targets() {
                for auto in [true, false] {
                    let mut form = Form::defaults();
                    form.lang = lang;
                    form.fill_auto = auto;
                    form.version = "1.0".into();
                    for notes in [Notes::default(), Notes { dirty: true, result: Some("x".into()), moved: true, loaded_from: "y".into() }] {
                        check_spec(&spec(lang, &form, target.as_ref(), &notes));
                    }
                }
            }
        }
    }

    /// A spec as a list: a note by its text, any other control by its name.
    fn signature(sp: &Value) -> Vec<String> {
        sp["controls"].as_array().unwrap().iter().map(|c| if c["type"] == "note" { format!("note:{}", c["label"].as_str().unwrap()) } else { c["name"].as_str().unwrap().to_string() }).collect()
    }

    #[test]
    fn a_section_note_looks_like_a_title_and_an_empty_note_goes_before_it() {
        let en = signature(&spec(Lang::En, &Form::defaults(), None, &Notes { loaded_from: "L".into(), ..Notes::default() }));
        assert_eq!(
            en,
            [
                "note:All changes are saved", "save", "reload", "note:", "note:── This page ──", "lang", "line", "fill_auto", "stage", "on_rerun", "tz",
                "note:Offset in hours from UTC (e.g. -3). Fixed: no daylight saving time.", "note:Select a component and press Reload", "note:L", "defaults", "close"
            ]
        );
        let pt = signature(&spec(Lang::Pt, &Form::defaults(), Some(&set_target()), &Notes { loaded_from: "L".into(), ..Notes::default() }));
        assert_eq!(
            pt,
            [
                "note:Todas as alterações estão salvas", "save", "reload", "note:", "note:── Esta página ──", "lang", "line", "fill_auto", "stage", "on_rerun", "tz",
                "note:Diferença em horas em relação ao UTC (ex.: -3). Fixo: sem horário de verão.", "note:", "note:── Conjunto de componentes: Button ──", "version", "status", "dv_0", "dv_1", "note:L", "defaults", "close"
            ]
        );
        let comp = signature(&spec(Lang::En, &Form::defaults(), Some(&component_target()), &Notes::default()));
        assert!(comp.windows(2).any(|w| w[0] == "note:" && w[1] == "note:── Component: Card ──"));
        let variant = Target { holder: 7, kind: TargetKind::Variant, title: "Button".into(), set: Some(set_info(0)) };
        let v = signature(&spec(Lang::Pt, &Form::defaults(), Some(&variant), &Notes::default()));
        assert!(v.windows(2).any(|w| w[0] == "note:" && w[1] == "note:── Variante do conjunto: Button ──"));
    }

    #[test]
    fn the_spacers_are_exactly_one_per_section_note_and_nothing_else_is_an_empty_note() {
        for target in all_targets() {
            let sp = spec(Lang::En, &Form::defaults(), target.as_ref(), &Notes { loaded_from: "Loaded from: page: defaults".into(), ..Notes::default() });
            let controls = sp["controls"].as_array().unwrap();
            let empty: Vec<usize> = controls.iter().enumerate().filter(|(_, c)| c["type"] == "note" && c["label"] == "").map(|(i, _)| i).collect();
            assert_eq!(empty.len(), if target.is_some() { 2 } else { 1 });
            for i in empty {
                let next = controls[i + 1]["label"].as_str().unwrap();
                assert!(next.starts_with("── ") && next.ends_with(" ──"), "a spacer must come right before a section note, not before {next:?}");
            }
            // The decorated notes keep inside the host's limits, with the longest title the tests use.
            check_spec(&sp);
        }
    }

    #[test]
    fn the_fill_colour_is_drawn_only_when_the_fill_is_not_automatic() {
        let mut form = Form::defaults();
        let has_fill = |f: &Form| spec(Lang::En, f, None, &Notes::default())["controls"].as_array().unwrap().iter().any(|c| c["name"] == "fill");
        assert!(!has_fill(&form));
        form.fill_auto = false;
        assert!(has_fill(&form));
    }

    #[test]
    fn the_panel_without_a_component_has_the_page_fields_and_the_note_and_no_component_fields() {
        let sp = spec(Lang::Pt, &Form::defaults(), None, &Notes::default());
        let names: Vec<&str> = sp["controls"].as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
        assert_eq!(names, ["save", "reload", "lang", "line", "fill_auto", "stage", "on_rerun", "tz", "defaults", "close"]);
        let notes: Vec<&str> = sp["controls"].as_array().unwrap().iter().filter(|c| c["type"] == "note").map(|c| c["label"].as_str().unwrap()).collect();
        assert!(notes.contains(&"Selecione um componente e aperte Recarregar"));
        assert!(notes.contains(&"Diferença em horas em relação ao UTC (ex.: -3). Fixo: sem horário de verão."));
        assert_eq!(sp["title"], "Handoff Components: Configurações");
    }

    #[test]
    fn the_panel_of_a_set_has_a_menu_per_variant_property_with_a_none_option() {
        let sp = spec(Lang::En, &Form::defaults(), Some(&set_target()), &Notes::default());
        let controls = sp["controls"].as_array().unwrap();
        let dv: Vec<&Value> = controls.iter().filter(|c| c["name"].as_str().is_some_and(|n| n.starts_with("dv_"))).collect();
        assert_eq!(dv.len(), 2);
        assert_eq!(dv[0]["label"], "Default variant: Size");
        assert_eq!(dv[0]["options"], json!(["(none)", "Small", "Large"]));
        assert_eq!(dv[1]["options"], json!(["(none)", "Enabled", "Hover"]));
        let pt = spec(Lang::Pt, &Form::defaults(), Some(&set_target()), &Notes::default());
        assert_eq!(pt["controls"].as_array().unwrap().iter().find(|c| c["name"] == "dv_0").unwrap()["label"], "Variante padrão: Size");
        // A component has the version and status but no variant menus.
        let c = spec(Lang::En, &Form::defaults(), Some(&component_target()), &Notes::default());
        let names: Vec<&str> = c["controls"].as_array().unwrap().iter().filter_map(|x| x["name"].as_str()).collect();
        assert!(names.contains(&"version") && names.contains(&"status") && !names.iter().any(|n| n.starts_with("dv_")));
    }

    #[test]
    fn a_set_with_many_properties_shows_the_first_ones_and_says_how_many_are_left() {
        let tg = Target { holder: 7, kind: TargetKind::Set, title: "Big".into(), set: Some(set_info(30)) };
        let sp = spec(Lang::En, &Form::defaults(), Some(&tg), &Notes::default());
        check_spec(&sp);
        let controls = sp["controls"].as_array().unwrap();
        assert_eq!(controls.iter().filter(|c| c["name"].as_str().is_some_and(|n| n.starts_with("dv_"))).count(), MAX_PROPS_SHOWN);
        assert!(controls.iter().any(|c| c["label"] == "+8 more variant properties not shown"));
    }

    // ---- the language switch loses nothing --------------------------------------------------

    fn busy_form() -> Form {
        Form {
            lang: Lang::Pt,
            line: "#3B82F6".into(),
            fill_auto: false,
            fill: "#3B82F633".into(),
            stage: StageMode::Dark,
            rerun: Rerun::New,
            tz: 5.5,
            version: "1.2.0 beta".into(),
            status: "Em revisão".into(),
            default: vec![("Size".into(), "Large".into()), ("State".into(), "Hover".into())],
        }
    }

    #[test]
    fn switching_the_language_redraws_everything_the_user_edited() {
        let target = set_target();
        let base = Form::defaults();
        let pt = busy_form();
        // The values the host would send after the switch to English: the controls as drawn in Portuguese.
        let drawn_pt = spec(Lang::Pt, &pt, Some(&target), &Notes::default());
        let mut values = values_of(&drawn_pt);
        values["lang"] = json!("English");
        let parsed = Form::from_values(&values, Some(&target), &base);
        assert_eq!(parsed, Form { lang: Lang::En, ..pt.clone() });
        // Redrawn in English, every field keeps its value (with its English words), and back again.
        let drawn_en = spec(Lang::En, &parsed, Some(&target), &Notes::default());
        check_spec(&drawn_en);
        let again = Form::from_values(&values_of(&drawn_en), Some(&target), &base);
        assert_eq!(again, parsed);
        let mut back = values_of(&drawn_en);
        back["lang"] = json!("Português (Brasil)");
        assert_eq!(Form::from_values(&back, Some(&target), &base), pt);
        // The English words are the English ones.
        let ctl = |sp: &Value, n: &str| sp["controls"].as_array().unwrap().iter().find(|c| c["name"] == n).unwrap().clone();
        assert_eq!(ctl(&drawn_pt, "stage")["value"], "Escuro");
        assert_eq!(ctl(&drawn_en, "stage")["value"], "Dark");
        assert_eq!(ctl(&drawn_en, "on_rerun")["value"], "New");
        assert_eq!(ctl(&drawn_en, "version")["value"], "1.2.0 beta");
        assert_eq!(ctl(&drawn_en, "dv_1")["value"], "Hover");
    }

    #[test]
    fn every_stage_and_rerun_and_none_label_is_understood_in_either_language() {
        for l in [Lang::En, Lang::Pt] {
            for s in [StageMode::Light, StageMode::Dark, StageMode::Auto] {
                assert_eq!(parse_stage_label(t(l, stage_key(s))), Some(s));
            }
            for r in [Rerun::Replace, Rerun::New] {
                assert_eq!(parse_rerun_label(t(l, rerun_key(r))), Some(r));
            }
            assert!(is_none_label(t(l, Key::PanelNone)));
            assert_eq!(parse_lang_label(lang_label(l)), Some(l));
        }
        assert_eq!(parse_stage_label("nope"), None);
        assert_eq!(stage_key(StageMode::Auto).name(), "PanelStageAuto");
        // The values stored stay in English whatever the label.
        assert_eq!(StageMode::Dark.name(), "dark");
        assert_eq!(Rerun::New.name(), "new");
    }

    #[test]
    fn a_none_menu_omits_the_property_and_a_property_with_a_long_option_still_fits() {
        let target = set_target();
        let mut values = values_of(&spec(Lang::En, &Form::defaults(), Some(&target), &Notes::default()));
        values["dv_0"] = json!("Large");
        values["dv_1"] = json!("(none)");
        let f = Form::from_values(&values, Some(&target), &Form::defaults());
        assert_eq!(f.default, vec![("Size".to_string(), "Large".to_string())]);
        values["dv_0"] = json!("(nenhuma)");
        assert!(Form::from_values(&values, Some(&target), &Form::defaults()).default.is_empty());
        let long = SetInfo::new(9, "S".into(), vec![variant(1, &format!("P={}", "y".repeat(200)))]);
        let tg = Target { holder: 9, kind: TargetKind::Set, title: "S".into(), set: Some(long) };
        let sp = spec(Lang::En, &Form::defaults(), Some(&tg), &Notes::default());
        check_spec(&sp);
        let mut vals = values_of(&sp);
        vals["dv_0"] = sp["controls"].as_array().unwrap().iter().find(|c| c["name"] == "dv_0").unwrap()["options"][1].clone();
        assert_eq!(Form::from_values(&vals, Some(&tg), &Form::defaults()).default.len(), 1, "a cut option maps back to the real one");
    }

    // ---- what a save writes -------------------------------------------------------------------

    fn st(text: &str) -> Settings {
        Settings::from_plugin_data(&json!({SETTINGS_KEY: text}))
    }

    #[test]
    fn nothing_changed_writes_nothing_and_a_key_that_is_absent_stays_absent() {
        let s = no_meta_stored();
        let form = Form::from_stored(&s, None);
        assert_eq!(plan(&s, &form, &none(), None).unwrap(), Plan::default());
        // The form shows the defaults: choosing them by hand changes nothing either (the edge case).
        let mut by_hand = Form::defaults();
        by_hand.line = "#FCA311".into();
        by_hand.tz = -3.0;
        by_hand.lang = Lang::En;
        assert_eq!(plan(&s, &by_hand, &none(), None).unwrap(), Plan::default());
    }

    #[test]
    fn a_field_the_user_did_not_change_keeps_its_stored_text_exactly() {
        let s = stored(st(r##"{"lang":"pt","line":"#3b82f6","fill":"#3B82F633","on_rerun":"new","stage":"dark","tz":"+9"}"##), None);
        let form = Form::from_stored(&s, None);
        assert_eq!(form.lang, Lang::Pt);
        assert_eq!(plan(&s, &form, &none(), None).unwrap(), Plan::default(), "pt, lower-case hex and +9 are left as they are");
        // One change touches one key.
        let mut changed = form.clone();
        changed.stage = StageMode::Auto;
        let p = plan(&s, &changed, &none(), None).unwrap();
        assert_eq!(p.settings.unwrap().to_text().unwrap(), r##"{"fill":"#3B82F633","lang":"pt","line":"#3b82f6","on_rerun":"new","stage":"auto","tz":"+9"}"##);
    }

    #[test]
    fn a_changed_field_is_written_in_its_canonical_form() {
        let s = no_meta_stored();
        let mut f = Form::from_stored(&s, None);
        f.lang = Lang::Pt;
        f.line = "#112233".into();
        f.stage = StageMode::Dark;
        f.rerun = Rerun::New;
        f.tz = 5.5;
        let p = plan(&s, &f, &none(), None).unwrap();
        assert_eq!(p.settings.unwrap().to_text().unwrap(), r##"{"lang":"pt-BR","line":"#112233","on_rerun":"new","stage":"dark","tz":"5.5"}"##);
        assert!(p.meta.is_none());
    }

    #[test]
    fn restore_defaults_removes_the_keys_that_are_stored_and_nothing_when_none_is() {
        let full = stored(st(r##"{"lang":"pt-BR","line":"#112233","fill":"#11223344","on_rerun":"new","stage":"dark","tz":"5.5"}"##), None);
        let form = Form::from_stored(&full, None).restore_page_defaults();
        let cleared: BTreeSet<Field> = Field::PAGE.into_iter().collect();
        let p = plan(&full, &form, &cleared, None).unwrap();
        assert_eq!(p.settings.as_ref().unwrap(), &Settings::default());
        assert_eq!(p.settings.unwrap().to_text(), None, "every key goes: the stored text is removed");
        // An explicit default that is stored is removed too, because it was marked.
        let explicit = stored(st(r##"{"line":"#FCA311","tz":"-3"}"##), None);
        let f2 = Form::from_stored(&explicit, None);
        assert_eq!(plan(&explicit, &f2, &none(), None).unwrap(), Plan::default(), "unmarked: untouched");
        assert_eq!(plan(&explicit, &f2.restore_page_defaults(), &cleared, None).unwrap().settings.unwrap().to_text(), None);
        // Nothing stored: nothing to do.
        let empty = no_meta_stored();
        assert_eq!(plan(&empty, &Form::defaults(), &cleared, None).unwrap(), Plan::default());
    }

    #[test]
    fn the_fill_follows_the_checkbox() {
        let with_fill = stored(st(r##"{"fill":"#3B82F633","line":"#112233"}"##), None);
        let mut f = Form::from_stored(&with_fill, None);
        assert!(!f.fill_auto && f.fill == "#3B82F633");
        f.fill_auto = true;
        assert_eq!(plan(&with_fill, &f, &none(), None).unwrap().settings.unwrap().to_text().unwrap(), r##"{"line":"#112233"}"##, "only the fill goes");
        let without = stored(st(r##"{"line":"#112233"}"##), None);
        let mut g = Form::from_stored(&without, None);
        assert!(g.fill_auto);
        assert_eq!(g.fill, "#1122334D", "the colour shown is the line at 30%");
        g.fill_auto = false;
        assert_eq!(plan(&without, &g, &none(), None).unwrap().settings.unwrap().to_text().unwrap(), r##"{"fill":"#1122334D","line":"#112233"}"##, "unchecking is a change");
        g.fill = "#ABCDEF80".into();
        assert!(plan(&without, &g, &none(), None).unwrap().settings.unwrap().to_text().unwrap().contains("#ABCDEF80"));
    }

    #[test]
    fn a_stored_value_that_cannot_be_read_is_left_alone_unless_restored() {
        let junk = stored(st(r##"{"line":"red","fill":"blue","stage":"loud","tz":"99"}"##), None);
        let form = Form::from_stored(&junk, None);
        assert_eq!(form.line, "#FCA311");
        assert!(form.fill_auto);
        assert_eq!(plan(&junk, &form, &none(), None).unwrap(), Plan::default());
        let cleared: BTreeSet<Field> = Field::PAGE.into_iter().collect();
        assert_eq!(plan(&junk, &form, &cleared, None).unwrap().settings.unwrap().to_text(), None);
    }

    fn meta_stored(text: &str) -> Stored {
        stored(Settings::default(), Some(Meta::from_plugin_data(&json!({META_KEY: text}))))
    }

    #[test]
    fn version_and_status_are_trimmed_and_empty_text_removes_them() {
        let s = meta_stored(r#"{"date":"2026-01-05","status":"Aprovado","version":"1.2.0"}"#);
        let tg = component_target();
        let mut f = Form::from_stored(&s, Some(&tg));
        assert_eq!((f.version.as_str(), f.status.as_str()), ("1.2.0", "Aprovado"));
        assert_eq!(plan(&s, &f, &none(), Some(&tg)).unwrap(), Plan::default());
        f.status = String::new();
        f.version = "  1.3.0  ".into();
        let p = plan(&s, &f, &none(), Some(&tg)).unwrap();
        assert_eq!(p.meta.unwrap().to_text().unwrap(), r#"{"date":"2026-01-05","version":"1.3.0"}"#, "the date from 0.9.5 is written back untouched");
        f.version = String::new();
        assert_eq!(plan(&s, &f, &none(), Some(&tg)).unwrap().meta.unwrap().to_text().unwrap(), r#"{"date":"2026-01-05"}"#);
        // Whitespace only is empty.
        f.version = "   ".into();
        assert!(plan(&meta_stored("{}"), &f, &none(), Some(&tg)).unwrap().is_empty());
    }

    #[test]
    fn the_default_variant_is_the_chosen_pairs_in_the_canonical_text() {
        let tg = set_target();
        let s = meta_stored(r#"{"default":"State=Hover, Size=Small"}"#);
        let f = Form::from_stored(&s, Some(&tg));
        assert_eq!(f.default, vec![("Size".to_string(), "Small".to_string()), ("State".to_string(), "Hover".to_string())], "in the set's order");
        assert!(plan(&s, &f, &none(), Some(&tg)).unwrap().is_empty(), "order of the stored text is not a change");
        let mut g = f.clone();
        g.default = vec![("Size".into(), "Large".into())];
        assert_eq!(plan(&s, &g, &none(), Some(&tg)).unwrap().meta.unwrap().to_text().unwrap(), r#"{"default":"Size=Large"}"#);
        g.default.clear();
        assert_eq!(plan(&s, &g, &none(), Some(&tg)).unwrap().meta.unwrap().to_text(), None, "(none) in every menu removes it");
        // A stale stored pair is not shown and not a change.
        let stale = meta_stored(r#"{"default":"Size=Huge"}"#);
        let fs = Form::from_stored(&stale, Some(&tg));
        assert!(fs.default.is_empty());
        assert!(plan(&stale, &fs, &none(), Some(&tg)).unwrap().is_empty());
    }

    #[test]
    fn a_combination_no_variant_has_is_refused_before_anything_is_written() {
        let info = SetInfo::new(7, "B".into(), vec![variant(1, "Size=Small, State=Enabled"), variant(2, "Size=Large, State=Hover")]);
        let tg = Target { holder: 7, kind: TargetKind::Set, title: "B".into(), set: Some(info) };
        let s = meta_stored("{}");
        let mut f = Form::from_stored(&s, Some(&tg));
        f.default = vec![("Size".into(), "Large".into()), ("State".into(), "Enabled".into())];
        assert_eq!(plan(&s, &f, &none(), Some(&tg)), Err(PlanError::NoSuchVariant("Size=Large, State=Enabled".into())));
    }

    #[test]
    fn without_a_component_the_sheet_is_never_touched_even_if_the_form_has_text() {
        let s = no_meta_stored();
        let mut f = Form::defaults();
        f.version = "9".into();
        assert!(plan(&s, &f, &none(), None).unwrap().is_empty());
        // And the version typed is ignored when parsing values without a target.
        let parsed = Form::from_values(&json!({"version": "zzz"}), None, &Form::defaults());
        assert_eq!(parsed.version, "");
    }

    #[test]
    fn an_edit_takes_back_the_restore_mark_of_its_own_field_only() {
        assert_eq!(Field::of_control("fill_auto"), Some(Field::Fill));
        assert_eq!(Field::of_control("fill"), Some(Field::Fill));
        assert_eq!(Field::of_control("on_rerun"), Some(Field::Rerun));
        assert_eq!(Field::of_control("version"), None);
        assert_eq!(Field::of_control("save"), None);
    }

    // ---- the parameters path and the panel write the same JSON ------------------------------

    fn params(pairs: &[(&str, &str)]) -> impl Fn(&str) -> String {
        let m: BTreeMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |name: &str| m.get(name).cloned().unwrap_or_default()
    }

    fn by_params(stored_settings: &Settings, pairs: &[(&str, &str)]) -> Settings {
        let field = params(pairs);
        let asked = field("lang");
        let asked_lang = if asked.is_empty() || asked == "-" { None } else { Lang::strict(&asked) };
        let lang = asked_lang.or_else(|| stored_settings.lang.as_deref().and_then(Lang::strict)).unwrap_or_default();
        crate::settings::page_from_params(lang, stored_settings, asked_lang, &field).unwrap()
    }

    #[test]
    fn the_parameters_path_and_the_panel_write_the_same_page_json() {
        let empty = Settings::default();
        // Several fields at once.
        let legacy = by_params(&empty, &[("lang", "pt-BR"), ("cor_linha", "#3B82F6"), ("cor_preenchimento", "#3B82F633"), ("on_rerun", "new"), ("stage", "dark"), ("tz", "5.5")]);
        let form = Form { lang: Lang::Pt, line: "#3B82F6".into(), fill_auto: false, fill: "#3B82F633".into(), stage: StageMode::Dark, rerun: Rerun::New, tz: 5.5, ..Form::defaults() };
        let panel = plan(&no_meta_stored(), &form, &none(), None).unwrap().settings.unwrap();
        assert_eq!(panel.to_text(), legacy.to_text());
        assert_eq!(panel.to_text().unwrap(), r##"{"fill":"#3B82F633","lang":"pt-BR","line":"#3B82F6","on_rerun":"new","stage":"dark","tz":"5.5"}"##);
        // One field.
        let legacy = by_params(&empty, &[("stage", "auto")]);
        let panel = plan(&no_meta_stored(), &Form { stage: StageMode::Auto, ..Form::defaults() }, &none(), None).unwrap().settings.unwrap();
        assert_eq!(panel.to_text(), legacy.to_text());
        // Everything cleared: "-" in every field, and "Restore defaults".
        let full = st(r##"{"lang":"pt-BR","line":"#112233","fill":"#11223344","on_rerun":"new","stage":"dark","tz":"5.5"}"##);
        let legacy = by_params(&full, &[("lang", "-"), ("cor_linha", "-"), ("cor_preenchimento", "-"), ("on_rerun", "-"), ("stage", "-"), ("tz", "-")]);
        let cleared: BTreeSet<Field> = Field::PAGE.into_iter().collect();
        let panel = plan(&stored(full.clone(), None), &Form::from_stored(&stored(full, None), None).restore_page_defaults(), &cleared, None).unwrap().settings.unwrap();
        assert_eq!(panel.to_text(), legacy.to_text());
        assert_eq!(panel.to_text(), None);
    }

    #[test]
    fn the_two_paths_differ_only_where_the_panel_does_not_write_what_the_user_did_not_change() {
        // Typing the default by hand stores it through the parameters (as always) but not through the panel.
        let legacy = by_params(&Settings::default(), &[("tz", "-3"), ("stage", "light")]);
        assert_eq!(legacy.to_text().unwrap(), r#"{"stage":"light","tz":"-3"}"#);
        let panel = plan(&no_meta_stored(), &Form::defaults(), &none(), None).unwrap();
        assert!(panel.is_empty());
    }

    #[test]
    fn the_sheet_is_the_same_json_through_both_paths() {
        // version and status typed (legacy) or edited (panel), default variant chosen.
        let tg = set_target();
        let s = meta_stored("{}");
        let mut f = Form::from_stored(&s, Some(&tg));
        f.version = "1.2.0".into();
        f.status = "Aprovado".into();
        f.default = vec![("Size".into(), "Small".into())];
        let panel = plan(&s, &f, &none(), Some(&tg)).unwrap().meta.unwrap();
        let legacy = Meta { version: crate::store::update(&None, "1.2.0"), status: crate::store::update(&None, "Aprovado"), default: Some("Size=Small".into()), other: serde_json::Map::new() };
        assert_eq!(panel.to_text(), legacy.to_text());
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
        assert_eq!(norm_tz(-0.0).to_string(), "0");
    }

    #[test]
    fn where_the_values_came_from_is_said_in_words() {
        let defaults = no_meta_stored();
        assert_eq!(loaded_from(Lang::En, &defaults, None), "Loaded from: page: defaults");
        assert_eq!(loaded_from(Lang::Pt, &defaults, None), "Carregado de: página: padrões");
        let both = stored(st(r#"{"stage":"dark"}"#), Some(Meta::from_plugin_data(&json!({META_KEY: r#"{"version":"1"}"#}))));
        assert_eq!(loaded_from(Lang::En, &both, Some(&component_target())), "Loaded from: page: handoff-settings; component: handoff-meta");
        let new_comp = stored(Settings::default(), Some(Meta::default()));
        assert_eq!(loaded_from(Lang::Pt, &new_comp, Some(&component_target())), "Carregado de: página: padrões; componente: padrões");
    }

    #[test]
    fn the_notes_say_whether_there_are_unsaved_changes_in_both_languages() {
        let sp = |lang: Lang, dirty: bool| spec(lang, &Form::defaults(), None, &Notes { dirty, ..Notes::default() })["controls"][0]["label"].as_str().unwrap().to_string();
        assert_eq!(sp(Lang::En, true), "There are unsaved changes");
        assert_eq!(sp(Lang::En, false), "All changes are saved");
        assert_eq!(sp(Lang::Pt, true), "Há alterações não salvas");
        assert_eq!(sp(Lang::Pt, false), "Todas as alterações estão salvas");
    }
}
