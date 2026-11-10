//! The "settings" command: a form of text fields that keeps what the report uses, so "generate"
//! needs no parameters. A field left empty keeps the stored value; "-" clears it.
//!
//! Language, colours and what to do on a rerun go to the current page; version, date and status
//! go to the selected component (its set, for a variant) and need a selection.

use crate::gerar::{host, load_set, page_info, resolve};
use crate::i18n::{Key, Lang, t, tf};
use crate::store::{META_KEY, Meta, Rerun, SETTINGS_KEY, StageMode, is_hex_colour, update};
use crate::variants::{self, DefaultError};
use figcraft_plugin_sdk::{Input, Result, Value, json, notify};

/// Write (or remove, when `text` is `None`) a `pluginData` key of layer `id`.
fn write_data(lang: Lang, id: u64, key: &str, text: Option<String>) -> Result<()> {
    let value = text.map_or(Value::Null, Value::String);
    host(lang, "node.set", &json!({"ids": [id], "props": {"pluginData": {key: value}}}))?;
    Ok(())
}

/// The words for a refused default variant.
fn default_error(lang: Lang, e: DefaultError) -> String {
    let list = |v: &[String]| v.join(", ");
    match e {
        DefaultError::Malformed(text) => tf(lang, Key::ErrDefaultMalformed, &[("text", &text)]),
        DefaultError::UnknownProp { prop, valid } => tf(lang, Key::ErrDefaultProp, &[("prop", &prop), ("valid", &list(&valid))]),
        DefaultError::UnknownOption { prop, value, valid } => tf(lang, Key::ErrDefaultOption, &[("prop", &prop), ("value", &value), ("valid", &list(&valid))]),
        DefaultError::NoSuchVariant(name) => tf(lang, Key::ErrDefaultNoVariant, &[("name", &name)]),
    }
}

/// The new default variant: empty keeps it, "-" clears it, anything else must name valid
/// properties and options of the component's set (and may be partial).
fn default_field(lang: Lang, target: &crate::gerar::Picked, current: &Option<String>, field: &str) -> Result<Option<String>> {
    match update(current, field) {
        new if new == *current => Ok(new),
        None => Ok(None),
        Some(typed) => {
            if target.holder == target.component {
                return Err(t(lang, Key::ErrNoVariants).to_string());
            }
            let (info, _) = load_set(lang, target.holder)?;
            let pairs = variants::parse_default(&typed).map_err(|e| default_error(lang, e))?;
            variants::complete(&info, &pairs).map_err(|e| default_error(lang, e))?;
            Ok(Some(variants::join(&pairs)))
        }
    }
}

/// The stored stage mode, kept when the field is empty.
fn stage_keep(stored: &crate::store::Settings) -> Option<String> {
    stored.stage.clone()
}

/// The new value of a colour field.
fn colour_field(lang: Lang, current: &Option<String>, field: &str) -> Result<Option<String>> {
    let new = update(current, field);
    match &new {
        Some(c) if new != *current && !is_hex_colour(c) => Err(tf(lang, Key::ErrBadColour, &[("value", c)])),
        _ => Ok(new),
    }
}

pub fn settings(input: Input) -> Result<Value> {
    let page = page_info()?;
    let stored = page.settings.clone();
    let field = |name: &str| input.str(name).unwrap_or_default().trim().to_string();

    // The language of the messages: the one being set, else the stored one.
    let asked = field("lang");
    let asked_lang = if asked.is_empty() || asked == "-" { None } else { Lang::strict(&asked) };
    let lang = asked_lang.or_else(|| stored.lang.as_deref().and_then(Lang::strict)).unwrap_or_default();
    if !asked.is_empty() && asked != "-" && asked_lang.is_none() {
        return Err(tf(lang, Key::ErrBadLang, &[("value", &asked)]));
    }

    let mut new = stored.clone();
    new.lang = if asked == "-" { None } else { asked_lang.map(|l| l.code().to_string()).or_else(|| stored.lang.clone()) };
    new.line = colour_field(lang, &stored.line, &field("cor_linha"))?;
    new.fill = colour_field(lang, &stored.fill, &field("cor_preenchimento"))?;
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
        stage_keep(&stored)
    } else if stage == "-" {
        None
    } else {
        Some(StageMode::parse(&stage).ok_or_else(|| tf(lang, Key::ErrBadStage, &[("value", &stage)]))?.name().to_string())
    };

    // The component's sheet: needs a selection when any of its fields is given.
    let sheet_given = ["versao", "data", "status", "default_variant"].iter().any(|n| !field(n).is_empty());
    let mut meta_out = Value::Null;
    let mut meta_written = false;
    let mut holder_out = Value::Null;
    if sheet_given {
        if input.selection.is_empty() {
            return Err(t(lang, Key::ErrMetaSelection).to_string());
        }
        let target = resolve(lang, &input.selection)?;
        let data = host(lang, "node.get", &json!({"id": target.holder, "fields": ["pluginData"]}))?;
        let current = Meta::from_plugin_data(data.get("pluginData").unwrap_or(&Value::Null));
        let next = Meta {
            version: update(&current.version, &field("versao")),
            date: update(&current.date, &field("data")),
            status: update(&current.status, &field("status")),
            default: default_field(lang, &target, &current.default, &field("default_variant"))?,
        };
        if next != current {
            write_data(lang, target.holder, META_KEY, next.to_text())?;
            meta_written = true;
        }
        meta_out = json!({"version": next.version, "date": next.date, "status": next.status, "default": next.default});
        holder_out = json!(target.holder);
    }

    let settings_written = new != stored;
    if settings_written {
        write_data(lang, page.id, SETTINGS_KEY, new.to_text())?;
    }
    notify(t(lang, if settings_written || meta_written { Key::SettingsSaved } else { Key::SettingsKept }));
    Ok(json!({
        "page": page.id,
        "settings": {"lang": new.lang, "line": new.line, "fill": new.fill, "on_rerun": new.on_rerun, "stage": new.stage, "written": settings_written, "key": SETTINGS_KEY},
        "meta": meta_out, "meta_holder": holder_out, "meta_written": meta_written, "meta_key": META_KEY,
        "stored_json": new.to_text()
    }))
}
