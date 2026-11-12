//! The direct way to run "generate": no panel. A script (or the regression net) passes `direct: true`
//! and, if it wants, the page's settings and the version and status as parameters:
//!
//! `lang`, `line`, `fill_opacity`, `stage`, `on_rerun` go to the current page's settings (an empty one keeps
//! what is stored, `-` clears it, anything else must be valid, with the messages of the dictionary);
//! `version` and `status` go to the documentation's header, nothing is remembered.

use crate::gerar::{generate, host, page_info};
use crate::i18n::{Key, Lang, tf};
use crate::store::{Rerun, SETTINGS_KEY, Settings, StageMode, is_hex_colour, update};
use figcraft_plugin_sdk::{Input, Result, Value, json};

/// Write (or remove, when `text` is `None`) a `pluginData` key of layer `id`.
pub(crate) fn write_data(lang: Lang, id: u64, key: &str, text: Option<String>) -> Result<()> {
    let value = text.map_or(Value::Null, Value::String);
    host(lang, "node.set", &json!({"ids": [id], "props": {"pluginData": {key: value}}}))?;
    Ok(())
}

/// The parameters that set the page's settings.
const SETTING_PARAMS: [&str; 5] = ["lang", "line", "fill_opacity", "stage", "on_rerun"];

/// Whether the run asked for no panel.
pub fn wanted(input: &Input) -> bool {
    input.params.get("direct").and_then(Value::as_bool).unwrap_or(false)
}

/// The new value of a colour field.
fn colour_field(lang: Lang, current: &Option<String>, field: &str) -> Result<Option<String>> {
    let new = update(current, field);
    match &new {
        Some(c) if new != *current && !is_hex_colour(c) => Err(tf(lang, Key::ErrBadColour, &[("value", c)])),
        _ => Ok(new),
    }
}

/// The new fill opacity: empty keeps it, `-` goes back to the default (nothing stored), anything else must be a
/// whole number from 0 to 100.
fn opacity_field(lang: Lang, current: Option<u8>, field: &str) -> Result<Option<u8>> {
    match field {
        "" => Ok(current),
        "-" => Ok(None),
        typed => match parse_opacity(typed) {
            Some(n) => Ok(Some(n)),
            None => Err(tf(lang, Key::ErrBadOpacity, &[("value", typed)])),
        },
    }
}

/// A fill opacity as typed: a whole number from 0 to 100, nothing else (no sign, no fraction, no unit).
pub(crate) fn parse_opacity(text: &str) -> Option<u8> {
    let t = text.trim();
    (!t.is_empty() && t.len() <= 3 && t.bytes().all(|b| b.is_ascii_digit())).then(|| t.parse::<u8>().ok()).flatten().filter(|n| *n <= 100)
}

/// A stored language as it is saved from now on: `pt` (0.9.5 and before) becomes `pt-BR`.
/// A value that names no language is kept as it is.
fn canonical_lang(stored: &Option<String>) -> Option<String> {
    stored.as_ref().map(|s| Lang::strict(s).map_or_else(|| s.clone(), |l| l.code().to_string()))
}

/// The page's settings after the parameters (`field(name)` is a parameter's trimmed text): empty keeps,
/// "-" clears, anything else must be valid. Nothing is written here.
pub(crate) fn page_from_params(lang: Lang, stored: &Settings, asked_lang: Option<Lang>, field: &dyn Fn(&str) -> String) -> Result<Settings> {
    let asked = field("lang");
    let mut new = stored.clone();
    new.lang = if asked == "-" { None } else { asked_lang.map(|l| l.code().to_string()).or_else(|| canonical_lang(&stored.lang)) };
    new.line = colour_field(lang, &stored.line, &field("line"))?;
    new.fill_opacity = opacity_field(lang, stored.fill_opacity, &field("fill_opacity"))?;
    let rerun = field("on_rerun");
    new.on_rerun = if rerun.is_empty() {
        stored.on_rerun.clone()
    } else if rerun == "-" {
        None
    } else {
        Some(Rerun::parse(&rerun).ok_or_else(|| tf(lang, Key::ErrBadRerun, &[("value", &rerun)]))?.name().to_string())
    };
    let stage = field("stage");
    new.stage = if stage.is_empty() {
        stored.stage.clone()
    } else if stage == "-" {
        None
    } else {
        Some(StageMode::parse(&stage).ok_or_else(|| tf(lang, Key::ErrBadStage, &[("value", &stage)]))?.name().to_string())
    };
    Ok(new)
}

/// Run "generate" with no panel: write the page's settings the parameters ask for, then generate.
pub fn run(input: Input) -> Result<Value> {
    // No page setting asked for: nothing to read or write first.
    if !SETTING_PARAMS.iter().any(|n| input.params.get(n).is_some()) {
        return generate(input);
    }
    let page = page_info()?;
    let stored = page.settings.clone();
    // A parameter is text; a number (`"fill_opacity": 50`) is read as its text.
    let field = |name: &str| match input.params.get(name) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };

    // The language of the messages: the one being set, else the stored one.
    let asked = field("lang");
    let asked_lang = if asked.is_empty() || asked == "-" { None } else { Lang::strict(&asked) };
    let lang = asked_lang.or_else(|| stored.lang.as_deref().and_then(Lang::strict)).unwrap_or_default();
    if !asked.is_empty() && asked != "-" && asked_lang.is_none() {
        return Err(tf(lang, Key::ErrBadLang, &[("value", &asked)]));
    }
    let new = page_from_params(lang, &stored, asked_lang, &field)?;
    if new != stored {
        write_data(lang, page.id, SETTINGS_KEY, new.to_text())?;
    }
    generate(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn params(pairs: &[(&str, &str)]) -> impl Fn(&str) -> String {
        let m: BTreeMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |name: &str| m.get(name).cloned().unwrap_or_default()
    }

    fn by_params(stored: &Settings, pairs: &[(&str, &str)]) -> Result<Settings> {
        let field = params(pairs);
        let asked = field("lang");
        let asked_lang = if asked.is_empty() || asked == "-" { None } else { Lang::strict(&asked) };
        let lang = asked_lang.or_else(|| stored.lang.as_deref().and_then(Lang::strict)).unwrap_or_default();
        page_from_params(lang, stored, asked_lang, &field)
    }

    #[test]
    fn direct_is_asked_for_with_a_true_parameter_only() {
        let with = |p: Value| Input { params: p, ..Input::default() };
        assert!(wanted(&with(json!({"direct": true}))));
        assert!(!wanted(&with(json!({}))), "the menu sends {{}}: it opens the panel");
        assert!(!wanted(&with(Value::Null)));
        assert!(!wanted(&with(json!({"direct": false}))));
        assert!(!wanted(&with(json!({"direct": "true"}))));
    }

    #[test]
    fn the_parameters_write_the_page_settings_like_the_old_form_did() {
        let empty = Settings::default();
        let s = by_params(&empty, &[("lang", "pt-BR"), ("line", "#3B82F6"), ("fill_opacity", "45"), ("on_rerun", "new"), ("stage", "dark")]).unwrap_or_default();
        assert_eq!(s.to_text().unwrap_or_default(), r##"{"fill_opacity":45,"lang":"pt-BR","line":"#3B82F6","on_rerun":"new","stage":"dark"}"##);
        // Empty keeps, "-" clears.
        let kept = by_params(&s, &[]).unwrap_or_default();
        assert_eq!(kept, s);
        let cleared = by_params(&s, &[("lang", "-"), ("line", "-"), ("fill_opacity", "-"), ("on_rerun", "-"), ("stage", "-")]).unwrap_or_default();
        assert_eq!(cleared.to_text(), None);
    }

    #[test]
    fn a_stored_pt_becomes_pt_br_and_a_leftover_key_is_kept() {
        let old = Settings::from_plugin_data(&json!({SETTINGS_KEY: r##"{"lang":"pt","tz":"-3"}"##}));
        let s = by_params(&old, &[("stage", "light")]).unwrap_or_default();
        assert_eq!(s.to_text().unwrap_or_default(), r##"{"lang":"pt-BR","stage":"light","tz":"-3"}"##);
    }

    #[test]
    fn a_bad_value_is_refused_with_the_dictionarys_message_in_the_language_of_the_page() {
        let err = |p: &[(&str, &str)]| by_params(&Settings::default(), p).err().unwrap_or_default();
        assert!(err(&[("line", "red")]).contains("\"red\""));
        for bad in ["nope", "101", "-5", "12.5", "30%", " ", "1e1", "300", "+5"] {
            let e = err(&[("fill_opacity", bad)]);
            if bad.trim().is_empty() {
                continue; // empty keeps the stored value: not an error
            }
            assert!(e.contains(&format!("\"{bad}\"")) && e.contains("0 to 100"), "{bad}: {e}");
        }
        assert!(err(&[("on_rerun", "twice")]).contains("replace or new"));
        assert!(err(&[("stage", "loud")]).contains("light, dark or auto"));
        let pt = Settings::from_plugin_data(&json!({SETTINGS_KEY: r##"{"lang":"pt-BR"}"##}));
        let e = by_params(&pt, &[("stage", "loud")]).err().unwrap_or_default();
        assert!(e.contains("palco") || e.contains("claro"), "{e}");
    }
}
