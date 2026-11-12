//! What the plugin remembers, and where.
//!
//! - **Settings** (language, colours, what to do on a rerun, the stage) are one JSON object in the
//!   `pluginData` of the current page, under [`SETTINGS_KEY`].
//! - **The component's sheet** (version, status and, for a set, the default variant) is one JSON object in the `pluginData` of
//!   the main component, under [`META_KEY`]; for a variant, of its component set, so that every
//!   variant shares it.
//!
//! `pluginData` values are strings, so each object is stored as JSON text. A field that was never
//! set is left out of the object, and an object with nothing in it is removed.
//!
//! Sheets saved by 0.9.5 and before may hold a `date`, and a page may hold a `tz` (a time zone, from
//! the days the report had a date). They are not read, but they are not deleted either: a key the
//! plugin doesn't know is written back as it was.

use figcraft_plugin_sdk::Value;

/// The `pluginData` key of the settings, on the page.
pub const SETTINGS_KEY: &str = "handoff-settings";
/// The `pluginData` key of the component's sheet, on the main component (or its set).
pub const META_KEY: &str = "handoff-meta";
/// The `pluginData` key that marks a report frame with the id of the component it shows.
pub const REPORT_KEY: &str = "handoff-report";

/// What to do when a report of the component is already on the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rerun {
    /// Delete the old report and make the new one where it was.
    #[default]
    Replace,
    /// Keep the old one and make a new one to the right of everything.
    New,
}

impl Rerun {
    /// `replace` or `new` (any case); anything else is no mode.
    pub fn parse(text: &str) -> Option<Rerun> {
        match text.trim().to_lowercase().as_str() {
            "replace" => Some(Rerun::Replace),
            "new" => Some(Rerun::New),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rerun::Replace => "replace",
            Rerun::New => "new",
        }
    }
}

/// The page's settings; a field that is `None` takes its default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub lang: Option<String>,
    pub line: Option<String>,
    pub fill: Option<String>,
    pub on_rerun: Option<String>,
    /// `light`, `dark` or `auto`.
    pub stage: Option<String>,
    /// The keys of the stored object that this version doesn't know (a `tz` from 0.9.6 and before):
    /// ignored when reading, written back when the settings are saved.
    pub other: serde_json::Map<String, Value>,
}

/// The stage behind the instances: light (the default), dark, or chosen row by row from contrast.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StageMode {
    #[default]
    Light,
    Dark,
    Auto,
}

impl StageMode {
    pub fn parse(text: &str) -> Option<StageMode> {
        match text.trim().to_lowercase().as_str() {
            "light" => Some(StageMode::Light),
            "dark" => Some(StageMode::Dark),
            "auto" => Some(StageMode::Auto),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            StageMode::Light => "light",
            StageMode::Dark => "dark",
            StageMode::Auto => "auto",
        }
    }
}

/// The component's sheet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    pub version: Option<String>,
    pub status: Option<String>,
    /// The default variant of a component set ("Size=Small, State=Enabled"; may be partial).
    pub default: Option<String>,
    /// The keys of the stored object that this version doesn't know (a `date` from 0.9.5 and
    /// before): ignored when reading, written back when the sheet is saved.
    pub other: serde_json::Map<String, Value>,
}

fn text_of(obj: &Value, key: &str) -> Option<String> {
    obj.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()).map(ToString::to_string)
}

/// The JSON object a `pluginData` key holds (its value is JSON text); null when absent or broken.
fn stored(plugin_data: &Value, key: &str) -> Value {
    plugin_data.get(key).and_then(Value::as_str).and_then(|s| serde_json::from_str::<Value>(s).ok()).filter(Value::is_object).unwrap_or(Value::Null)
}

/// The JSON text of an object made of the fields that are set (and the `other` keys kept as they
/// were); `None` when there is none.
fn to_text(fields: &[(&str, &Option<String>)], other: &serde_json::Map<String, Value>) -> Option<String> {
    let mut map = other.clone();
    for (k, v) in fields {
        if let Some(v) = v {
            map.insert((*k).to_string(), Value::String(v.clone()));
        }
    }
    (!map.is_empty()).then(|| Value::Object(map).to_string())
}

impl Settings {
    /// The settings in a node's `pluginData` (the `pluginData` object of `node.get`).
    pub fn from_plugin_data(plugin_data: &Value) -> Settings {
        let o = stored(plugin_data, SETTINGS_KEY);
        let other = o.as_object().map(|m| m.iter().filter(|(k, _)| !matches!(k.as_str(), "lang" | "line" | "fill" | "on_rerun" | "stage")).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
        Settings { lang: text_of(&o, "lang"), line: text_of(&o, "line"), fill: text_of(&o, "fill"), on_rerun: text_of(&o, "on_rerun"), stage: text_of(&o, "stage"), other }
    }

    pub fn to_text(&self) -> Option<String> {
        to_text(&[("lang", &self.lang), ("line", &self.line), ("fill", &self.fill), ("on_rerun", &self.on_rerun), ("stage", &self.stage)], &self.other)
    }

    /// Whether none of the settings this version knows is set (a leftover `tz` doesn't count).
    pub fn is_default(&self) -> bool {
        Settings { other: serde_json::Map::new(), ..self.clone() } == Settings::default()
    }

    /// The stage mode: the stored one, or light.
    pub fn stage_mode(&self) -> StageMode {
        self.stage.as_deref().and_then(StageMode::parse).unwrap_or_default()
    }

    /// The rerun mode: the stored one, or the default.
    pub fn rerun(&self) -> Rerun {
        self.on_rerun.as_deref().and_then(Rerun::parse).unwrap_or_default()
    }
}

impl Meta {
    pub fn from_plugin_data(plugin_data: &Value) -> Meta {
        let o = stored(plugin_data, META_KEY);
        let other = o.as_object().map(|m| m.iter().filter(|(k, _)| !matches!(k.as_str(), "version" | "status" | "default")).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
        Meta { version: text_of(&o, "version"), status: text_of(&o, "status"), default: text_of(&o, "default"), other }
    }

    pub fn to_text(&self) -> Option<String> {
        to_text(&[("default", &self.default), ("version", &self.version), ("status", &self.status)], &self.other)
    }
}

/// A form field's effect on a stored value: empty keeps it, "-" clears it, anything else replaces it.
pub fn update(current: &Option<String>, field: &str) -> Option<String> {
    match field.trim() {
        "" => current.clone(),
        "-" => None,
        v => Some(v.to_string()),
    }
}

/// Whether `text` is a hex colour: `#RGB`, `#RRGGBB` or `#RRGGBBAA`.
pub fn is_hex_colour(text: &str) -> bool {
    text.strip_prefix('#').is_some_and(|h| matches!(h.len(), 3 | 6 | 8) && h.chars().all(|c| c.is_ascii_hexdigit()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use figcraft_plugin_sdk::json;

    #[test]
    fn settings_round_trip_through_plugin_data() {
        let s = Settings { lang: Some("pt".into()), line: Some("#3B82F6".into()), fill: None, on_rerun: Some("new".into()), stage: None, other: serde_json::Map::new() };
        let text = s.to_text().unwrap();
        assert_eq!(text, r##"{"lang":"pt","line":"#3B82F6","on_rerun":"new"}"##);
        let back = Settings::from_plugin_data(&json!({SETTINGS_KEY: text}));
        assert_eq!(back, s);
        assert_eq!(back.rerun(), Rerun::New);
    }

    #[test]
    fn a_stored_time_zone_is_ignored_but_kept() {
        let back = Settings::from_plugin_data(&json!({SETTINGS_KEY: r##"{"lang":"pt","tz":"5.5"}"##}));
        assert_eq!(back.lang.as_deref(), Some("pt"));
        assert!(!back.is_default());
        assert_eq!(back.to_text().unwrap(), r##"{"lang":"pt","tz":"5.5"}"##);
        let only_tz = Settings::from_plugin_data(&json!({SETTINGS_KEY: r##"{"tz":"-3"}"##}));
        assert!(only_tz.is_default());
        assert_eq!(only_tz.to_text().unwrap(), r##"{"tz":"-3"}"##);
    }

    #[test]
    fn nothing_set_means_nothing_stored_and_defaults() {
        assert_eq!(Settings::default().to_text(), None);
        assert_eq!(Meta::default().to_text(), None);
        assert_eq!(Settings::default().rerun(), Rerun::Replace);
        // Absent, empty or broken data all read as nothing.
        for data in [json!({}), json!(null), json!({SETTINGS_KEY: "not json"}), json!({SETTINGS_KEY: "[1]"})] {
            assert_eq!(Settings::from_plugin_data(&data), Settings::default());
        }
    }

    #[test]
    fn a_field_keeps_clears_or_replaces() {
        let cur = Some("1.0".to_string());
        assert_eq!(update(&cur, ""), cur);
        assert_eq!(update(&cur, "   "), cur);
        assert_eq!(update(&cur, "-"), None);
        assert_eq!(update(&cur, " 2.0 "), Some("2.0".to_string()));
        assert_eq!(update(&None, ""), None);
    }

    #[test]
    fn meta_round_trip() {
        let m = Meta { version: Some("1.2.0".into()), status: Some("Aprovado".into()), default: Some("Size=Small".into()), other: serde_json::Map::new() };
        assert_eq!(Meta::from_plugin_data(&json!({META_KEY: m.to_text().unwrap()})), m);
    }

    #[test]
    fn a_date_saved_by_an_older_version_is_not_read_but_is_kept() {
        let old = r#"{"date":"2026-11-01","default":"Size=Small","status":"Aprovado","version":"1.2.0"}"#;
        let m = Meta::from_plugin_data(&json!({META_KEY: old}));
        assert_eq!((m.version.as_deref(), m.status.as_deref(), m.default.as_deref()), (Some("1.2.0"), Some("Aprovado"), Some("Size=Small")));
        // Changing the status writes the old date back untouched.
        let next = Meta { status: Some("Rascunho".into()), ..m };
        assert_eq!(next.to_text().as_deref(), Some(r#"{"date":"2026-11-01","default":"Size=Small","status":"Rascunho","version":"1.2.0"}"#));
        // A sheet that holds only the old date is not emptied.
        let only = Meta::from_plugin_data(&json!({META_KEY: r#"{"date":"2026-11-01"}"#}));
        assert_eq!(only.to_text().as_deref(), Some(r#"{"date":"2026-11-01"}"#));
        // Clearing every field the plugin knows leaves just that.
        let cleared = Meta { version: None, status: None, default: None, ..only };
        assert_eq!(cleared.to_text().as_deref(), Some(r#"{"date":"2026-11-01"}"#));
    }

    #[test]
    fn colours_and_modes() {
        for ok in ["#FFF", "#FF4D6D", "#FF4D6D33"] {
            assert!(is_hex_colour(ok), "{ok}");
        }
        for bad in ["FF4D6D", "#FF4D", "#GG4D6D", "", "red"] {
            assert!(!is_hex_colour(bad), "{bad}");
        }
        assert_eq!(Rerun::parse(" Replace "), Some(Rerun::Replace));
        assert_eq!(Rerun::parse("NEW"), Some(Rerun::New));
        assert_eq!(StageMode::parse(" Auto "), Some(StageMode::Auto));
        assert_eq!(StageMode::parse("both"), None);
        assert_eq!(Settings::default().stage_mode(), StageMode::Light);
        assert_eq!(Rerun::parse("both"), None);
    }
}
