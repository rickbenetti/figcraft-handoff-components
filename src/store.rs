//! What the plugin remembers, and where.
//!
//! The page's **settings** (language, colours, what to do on a rerun, the stage) are one JSON object in the
//! `pluginData` of the current page, under [`SETTINGS_KEY`]. Nothing else is remembered: Version and Status are
//! typed for each documentation.
//!
//! `pluginData` values are strings, so the object is stored as JSON text. A field that was never set is left
//! out of the object, and an object with nothing in it is removed.
//!
//! A page may still hold a `tz` (a time zone, from the days the documentation had a date), and a component the
//! `handoff-meta` sheet of earlier versions (version, status, default variant). None of it is read, and none
//! of it is deleted: a key the plugin doesn't know is written back as it was, and the sheet is never touched.

use figcraft_plugin_sdk::Value;

/// The `pluginData` key of the settings, on the page.
pub const SETTINGS_KEY: &str = "handoff-settings";
/// The `pluginData` key that marks a documentation frame with the id of the component it shows. It keeps
/// its old name so that a rerun still finds (and replaces) what earlier versions made.
pub const REPORT_KEY: &str = "handoff-report";

/// What to do when a documentation of the component is already on the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rerun {
    /// Delete the old documentation and make the new one where it was.
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

/// The page's settings; a field that is `None` takes its default. A `line` colour or a `fill_opacity` stored by earlier
/// versions is no longer a setting: it is kept in `other`, ignored, and written back as it was.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub lang: Option<String>,
    pub on_rerun: Option<String>,
    /// `light`, `dark` or `auto`.
    pub stage: Option<String>,
    /// The keys of the stored object that this version doesn't know (a `tz`, a `fill` colour, a `line` colour or a
    /// `fill_opacity` from earlier versions): ignored when reading, written back when the settings are saved.
    pub other: serde_json::Map<String, Value>,
}

/// The stage behind the instances: light, dark, or chosen row by row from contrast (the default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StageMode {
    Light,
    Dark,
    #[default]
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
        // Any key this version doesn't know (a `tz`, a `line` colour, a `fill_opacity`) stays in `other`, untouched.
        let known = |k: &str| matches!(k, "lang" | "on_rerun" | "stage");
        let other = o.as_object().map(|m| m.iter().filter(|(k, _)| !known(k)).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
        Settings { lang: text_of(&o, "lang"), on_rerun: text_of(&o, "on_rerun"), stage: text_of(&o, "stage"), other }
    }

    pub fn to_text(&self) -> Option<String> {
        to_text(&[("lang", &self.lang), ("on_rerun", &self.on_rerun), ("stage", &self.stage)], &self.other)
    }

    /// The stage mode: the stored one, or auto.
    pub fn stage_mode(&self) -> StageMode {
        self.stage.as_deref().and_then(StageMode::parse).unwrap_or_default()
    }

    /// The rerun mode: the stored one, or the default.
    pub fn rerun(&self) -> Rerun {
        self.on_rerun.as_deref().and_then(Rerun::parse).unwrap_or_default()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use figcraft_plugin_sdk::json;

    #[test]
    fn settings_round_trip_through_plugin_data() {
        let s = Settings { lang: Some("pt".into()), on_rerun: Some("new".into()), stage: Some("dark".into()), other: serde_json::Map::new() };
        let text = s.to_text().unwrap();
        assert_eq!(text, r##"{"lang":"pt","on_rerun":"new","stage":"dark"}"##);
        let back = Settings::from_plugin_data(&json!({SETTINGS_KEY: text}));
        assert_eq!(back, s);
        assert_eq!(back.rerun(), Rerun::New);
    }

    #[test]
    fn a_stored_time_zone_is_ignored_but_kept() {
        let back = Settings::from_plugin_data(&json!({SETTINGS_KEY: r##"{"lang":"pt","tz":"5.5"}"##}));
        assert_eq!(back.lang.as_deref(), Some("pt"));
        assert_eq!(back.to_text().unwrap(), r##"{"lang":"pt","tz":"5.5"}"##);
        let only_tz = Settings::from_plugin_data(&json!({SETTINGS_KEY: r##"{"tz":"-3"}"##}));
        assert_eq!(only_tz.to_text().unwrap(), r##"{"tz":"-3"}"##);
    }

    #[test]
    fn a_stored_line_colour_or_opacity_is_ignored_but_kept() {
        // Settings of earlier versions: the line colour and the fill opacity are no longer settings; they stay as they were.
        for stored in [r##"{"line":"#3B82F6"}"##, r#"{"fill_opacity":45}"#, r#"{"fill_opacity":150}"#, r##"{"lang":"pt-BR","line":"#112233","fill_opacity":0,"stage":"dark"}"##] {
            let s = Settings::from_plugin_data(&json!({SETTINGS_KEY: stored}));
            assert!(s.other.contains_key("line") || s.other.contains_key("fill_opacity"), "{stored}");
            let back: Value = serde_json::from_str(&s.to_text().unwrap()).unwrap();
            let want: Value = serde_json::from_str(stored).unwrap();
            assert_eq!(back, want, "{stored}");
        }
    }

    #[test]
    fn nothing_set_means_nothing_stored_and_defaults() {
        assert_eq!(Settings::default().to_text(), None);
        assert_eq!(Settings::default().rerun(), Rerun::Replace);
        // Absent, empty or broken data all read as nothing.
        for data in [json!({}), json!(null), json!({SETTINGS_KEY: "not json"}), json!({SETTINGS_KEY: "[1]"})] {
            assert_eq!(Settings::from_plugin_data(&data), Settings::default());
        }
    }

    #[test]
    fn rerun_and_stage_modes() {
        assert_eq!(Rerun::parse(" Replace "), Some(Rerun::Replace));
        assert_eq!(Rerun::parse("NEW"), Some(Rerun::New));
        assert_eq!(StageMode::parse(" Auto "), Some(StageMode::Auto));
        assert_eq!(StageMode::parse("both"), None);
        assert_eq!(Settings::default().stage_mode(), StageMode::Auto);
        assert_eq!(Rerun::parse("both"), None);
    }
}
