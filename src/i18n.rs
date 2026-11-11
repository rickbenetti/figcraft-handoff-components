//! Every text the plugin shows or writes: the report, the layer names it makes, the notes and the
//! errors. The texts live in `locales/<code>.json` (a flat `"Key": "text"` object per language),
//! embedded in the module with `include_str!` and read on first use.
//!
//! To add a language: add a variant to [`Lang`], bump [`LANGS`], teach [`Lang::parse`] its names,
//! add its `locales/<code>.json` and a line to [`table`]. A key a language leaves out uses the
//! English text at run time (the test `pt_br_misses_no_key` keeps that from happening for pt-BR).
//!
//! The plugin host gives a plugin no locale (the SDK's `Input` has none and the manifest takes one
//! label per field), so the language comes from the page's settings.

use figcraft_plugin_sdk::Value;
use std::sync::OnceLock;

/// Number of languages (the strings document has a column for each).
#[cfg(test)]
pub const LANGS: usize = 2;

/// A language of the report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    /// English, the default.
    #[default]
    En,
    /// Brazilian Portuguese.
    Pt,
}

impl Lang {
    /// The language a `lang` value names: `en`, `pt`, `pt-BR`, `PT`, `pt_br`, `português`…
    /// Anything else (and nothing) is the default, English.
    pub fn parse(text: &str) -> Lang {
        let lower = text.trim().to_lowercase();
        let head = lower.split(['-', '_', '.', '@', ' ']).next().unwrap_or_default();
        match head {
            "pt" | "por" | "ptbr" | "portuguese" | "português" | "portugues" | "brazilian" => Lang::Pt,
            _ => Lang::En,
        }
    }

    /// The language a `lang` value names, or none: `en`, `pt`, `pt-BR`… and nothing else. Both `pt`
    /// (the code saved by 0.9.5 and before) and `pt-BR` are Brazilian Portuguese.
    pub fn strict(text: &str) -> Option<Lang> {
        let lower = text.trim().to_lowercase();
        let head = lower.split(['-', '_', '.', '@', ' ']).next().unwrap_or_default();
        match head {
            "pt" | "por" | "ptbr" | "portuguese" | "português" | "portugues" | "brazilian" => Some(Lang::Pt),
            "en" | "eng" | "english" | "inglês" | "ingles" => Some(Lang::En),
            _ => None,
        }
    }

    /// The code of the language: the one the Settings saves and the name of its `locales/` file.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Pt => "pt-BR",
        }
    }

    /// Every language, in column order (for the strings document).
    #[cfg(test)]
    pub const ALL: [Lang; LANGS] = [Lang::En, Lang::Pt];
}

/// The texts of one language: the parsed `locales/<code>.json`.
type Table = serde_json::Map<String, Value>;

static EN: OnceLock<Table> = OnceLock::new();
static PT_BR: OnceLock<Table> = OnceLock::new();

/// The parsed texts of `lang` (parsed on first use; a broken file reads as empty).
fn table(lang: Lang) -> &'static Table {
    let (cell, json) = match lang {
        Lang::En => (&EN, include_str!("../locales/en.json")),
        Lang::Pt => (&PT_BR, include_str!("../locales/pt-BR.json")),
    };
    cell.get_or_init(|| match serde_json::from_str::<Value>(json) {
        Ok(Value::Object(map)) => map,
        _ => Table::new(),
    })
}

/// The text of `name` in `primary`, else in `fallback()` (not asked for unless needed), else nothing.
fn pick<'a>(primary: &'a Table, fallback: impl FnOnce() -> &'a Table, name: &str) -> &'a str {
    let text = |t: &'a Table| t.get(name).and_then(Value::as_str).filter(|s| !s.is_empty());
    text(primary).or_else(|| text(fallback())).unwrap_or_default()
}

macro_rules! keys {
    ($($key:ident),+ $(,)?) => {
        /// The key of a text: its name is the key in the JSON files.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Key {
            $($key),+
        }

        impl Key {
            /// Every key, in file order (for the strings document).
            #[cfg(test)]
            pub const ALL: &'static [Key] = &[$(Key::$key),+];

            /// The key's name, as in the JSON files.
            pub fn name(self) -> &'static str {
                match self {
                    $(Key::$key => stringify!($key)),+
                }
            }
        }
    };
}

// Placeholders are written `{name}`; see `tf`. The order is that of `locales/en.json`.
keys! {
    AlignBottom, AlignCenter, AlignLeft, AlignMiddle, AlignRight,
    AlignSpaceAround, AlignSpaceBetween, AlignSpaceEvenly, AlignTop, Anatomy,
    Collision, DateUnavailable, DefaultLine, DeltaContent, DeltaEffectOnly,
    DeltaEffects, DeltaHeight, DeltaHidden, DeltaMore, DeltaNone,
    DeltaOpacity, DeltaStrokeWeight, DeltaVisible, DeltaWidth, DirGrid,
    DirHorizontal, DirVertical, Done, ErrBadColour, ErrBadLang,
    ErrBadRerun, ErrBadStage, ErrBadTz, ErrCall, ErrDefaultMalformed,
    ErrDefaultNoVariant, ErrDefaultOption, ErrDefaultProp, ErrEmptySet, ErrMetaSelection,
    ErrNoId, ErrNoMain, ErrNoPage, ErrNoSelection, ErrNoVariants,
    ErrNotComponent, ErrNotList, ErrParent, FrameName, GeneratedOn,
    InstanceOf, LayerAnatomyRow, LayerAnatomySection, LayerAxis, LayerAxisNote,
    LayerAxisRow, LayerBadge, LayerBlockHeader, LayerBody, LayerChip,
    LayerDeltas, LayerDivider, LayerGenerated, LayerGroup, LayerHeaderBlock,
    LayerKind, LayerLeader, LayerLegend, LayerMarker, LayerMeasure,
    LayerMore, LayerNote, LayerProperties, LayerPropertyRow, LayerSpacingSection,
    LayerStage, LayerSubtitle, LayerSummary, LayerTitle, LayerValue,
    LayerWarning, LegendCheck, LegendDefault, LegendMissing, LegendNoStyle,
    LegendNone, LegendStyle, LegendSuspect, LegendToken, MoreLines,
    MoreVariants, NotDefined, NoteExport1x, NoteLineNoCoords, NoteLineOblique,
    OvCorners, OvEffects, OvFills, OvPadding, OvPosition,
    OvSize, OvStrokes, OvTextStyle, OverrideNote, PropFill,
    PropGap, PropPadH, PropPadV, PropRadius, PropStroke,
    PropTypography, PropertiesTitle, ResizeFill, ResizeFixed, ResizeHug,
    SettingsKept, SettingsSaved, SourceComponent, SourceFirst, SourceSaved,
    SpAlignment, SpDirection, SpHResizing, SpVResizing, SpacingTitle,
    Summary, SummaryChecks, SummaryStyles, SummaryStylesChecks, TypeBoolean,
    TypeInstance, TypeText, TypeVariant, Unnumbered, ValueFalse,
    ValueTrue, Variants, WarnNoDate, WarnTmpComment, KindFrame,
    KindGroup, KindRectangle, KindEllipse, KindVector, KindLine,
    KindStar, KindPolygon, KindBooleanOperation, KindComponent, KindComponentSet,
    KindInstance, EffectDropShadow, EffectInnerShadow, EffectLayerBlur, EffectBackgroundBlur,
    EffectDefault, EffectBlur, EffectSpread, AtPercent, StyleSuspect,
    TokenAndStyle, LineHeightAuto, LineHeightAutoHost, TypoLetterSpacing, DecoUnderline, DecoStrikethrough,
    ValueNone, AliasBroken, ErrUnknownCommand, AlignTopLeft, AlignTopCenter,
    AlignTopRight, AlignMiddleLeft, AlignMiddleCenter, AlignMiddleRight, AlignBottomLeft,
    AlignBottomCenter, AlignBottomRight, AlignCompose,
}

/// The text of `key` in `lang` (English when the language has none).
pub fn t(lang: Lang, key: Key) -> &'static str {
    pick(table(lang), || table(Lang::En), key.name())
}

/// The text of `key` with each `{name}` replaced by its value.
pub fn tf(lang: Lang, key: Key, values: &[(&str, &str)]) -> String {
    let mut s = t(lang, key).to_string();
    for (name, value) in values {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}

/// An override category as the app names it, in `lang`; the raw name when the map doesn't know it.
pub fn override_name(lang: Lang, raw: &str) -> String {
    let key = match raw {
        "corners" => Key::OvCorners,
        "padding" => Key::OvPadding,
        "size" => Key::OvSize,
        "position" => Key::OvPosition,
        "strokes" => Key::OvStrokes,
        "textStyle" => Key::OvTextStyle,
        "fills" => Key::OvFills,
        "effects" => Key::OvEffects,
        _ => return raw.to_string(),
    };
    t(lang, key).to_string()
}

/// The table as Markdown: `key | EN | PT-BR`, for `docs/i18n-strings.md`. A key a language leaves out
/// shows as `= EN`.
#[cfg(test)]
pub fn markdown() -> String {
    let cell = |s: &str| s.replace('|', "\\|");
    let mut out = String::from("# Handoff Components: strings\n\n");
    out.push_str("Generated from `locales/*.json` (the test `i18n::tests::strings_doc_is_current` checks it; run it with\n");
    out.push_str("`I18N_WRITE=1` to rewrite this file). Placeholders are `{name}`. The language is the page's setting\n");
    out.push_str("(`en` by default, `pt-BR` for Brazilian Portuguese; `pt` is read as `pt-BR`).\n\n");
    out.push_str("| key |");
    for l in Lang::ALL {
        out.push_str(&format!(" {} |", l.code().to_uppercase()));
    }
    out.push_str("\n|---|");
    for _ in Lang::ALL {
        out.push_str("---|");
    }
    out.push('\n');
    for key in Key::ALL {
        out.push_str(&format!("| {} |", key.name()));
        for l in Lang::ALL {
            match table(l).get(key.name()).and_then(Value::as_str) {
                Some(text) => out.push_str(&format!(" {} |", cell(text))),
                None => out.push_str(" = EN |"),
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The keys pt-BR keeps in English on purpose, for lack of a decision on their translation. Each
    /// has pt-BR text equal to the English one; the test below keeps this list and the files in step.
    const KEPT_IN_ENGLISH: &[&str] = &["AliasBroken", "ErrUnknownCommand", "EffectDefault"];

    /// The keys whose text is the same in both languages by nature (a word that is the same, or a
    /// technical form): not a gap.
    const SAME_BY_NATURE: &[&str] = &["DirHorizontal", "DirVertical", "ErrCall", "LegendToken", "OvPadding"];

    fn names(s: &str) -> Vec<String> {
        let mut v: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split('}').next()).map(ToString::to_string).collect();
        v.sort();
        v
    }

    fn all_keys() -> BTreeSet<&'static str> {
        Key::ALL.iter().map(|k| k.name()).collect()
    }

    #[test]
    fn language_names() {
        for pt in ["pt", "PT", "pt-BR", "pt_br", " Pt-br ", "português", "Portuguese"] {
            assert_eq!(Lang::parse(pt), Lang::Pt, "{pt}");
            assert_eq!(Lang::strict(pt), Some(Lang::Pt), "{pt}");
        }
        for en in ["", "en", "EN", "en-US", "fr", "xx"] {
            assert_eq!(Lang::parse(en), Lang::En, "{en}");
        }
        for bad in ["", "fr", "xx"] {
            assert_eq!(Lang::strict(bad), None, "{bad}");
        }
    }

    #[test]
    fn pt_and_pt_br_both_read_as_pt_br_and_the_code_saved_is_pt_br() {
        assert_eq!(Lang::parse("pt"), Lang::parse("pt-BR"));
        assert_eq!(Lang::strict("pt").map(Lang::code), Some("pt-BR"));
        assert_eq!(Lang::strict("PT-br").map(Lang::code), Some("pt-BR"));
        assert_eq!(Lang::strict("en").map(Lang::code), Some("en"));
        // The code saved reads back as the same language.
        for l in Lang::ALL {
            assert_eq!(Lang::strict(l.code()), Some(l));
        }
    }

    #[test]
    fn the_json_files_hold_exactly_the_keys_of_the_enum() {
        for l in Lang::ALL {
            let in_file: BTreeSet<&str> = table(l).keys().map(String::as_str).collect();
            let known = all_keys();
            let extra: Vec<_> = in_file.difference(&known).collect();
            assert!(extra.is_empty(), "{}: keys in the file that the code doesn't know: {extra:?}", l.code());
        }
        let in_en: BTreeSet<&str> = table(Lang::En).keys().map(String::as_str).collect();
        let missing: Vec<_> = all_keys().difference(&in_en).copied().collect();
        assert!(missing.is_empty(), "keys of the enum missing from en.json: {missing:?}");
    }

    #[test]
    fn every_text_is_filled_and_placeholders_match_the_english_ones() {
        for l in Lang::ALL {
            for key in Key::ALL {
                let text = t(l, *key);
                assert!(!text.is_empty(), "{} has no text in {}", key.name(), l.code());
                assert_eq!(names(t(Lang::En, *key)), names(text), "{} placeholders differ in {}", key.name(), l.code());
            }
        }
    }

    #[test]
    fn pt_br_misses_no_key() {
        // The English keys that pt-BR lacks: this list must stay empty.
        let missing: Vec<&str> = Key::ALL.iter().map(|k| k.name()).filter(|n| !table(Lang::Pt).contains_key(*n)).collect();
        assert!(missing.is_empty(), "keys in en.json that pt-BR lacks ({}): {missing:?}", missing.len());
    }

    #[test]
    fn pt_br_in_english_only_where_it_was_decided() {
        // The keys whose pt-BR text equals the English one must be exactly these two lists: the ones left
        // in English for lack of a decision, and the ones that are the same by nature. A translated key
        // that was reverted to English, or a new key left untranslated, breaks this.
        let same: BTreeSet<&str> = Key::ALL.iter().filter(|k| t(Lang::Pt, **k) == t(Lang::En, **k)).map(|k| k.name()).collect();
        let expected: BTreeSet<&str> = KEPT_IN_ENGLISH.iter().chain(SAME_BY_NATURE).copied().collect();
        assert_eq!(same, expected);
    }

    #[test]
    fn a_key_a_language_leaves_out_uses_english() {
        let en: Table = serde_json::from_str(r#"{"A":"one","B":"two"}"#).unwrap();
        let pt: Table = serde_json::from_str(r#"{"A":"um","B":""}"#).unwrap();
        assert_eq!(pick(&pt, || &en, "A"), "um");
        assert_eq!(pick(&pt, || &en, "B"), "two");
        assert_eq!(pick(&Table::new(), || &en, "A"), "one");
        assert_eq!(pick(&pt, || &en, "C"), "");
        // The fallback is only looked at when needed.
        let asked = std::cell::Cell::new(false);
        assert_eq!(pick(&pt, || { asked.set(true); &en }, "A"), "um");
        assert!(!asked.get());
    }

    #[test]
    fn replaces_placeholders() {
        assert_eq!(tf(Lang::En, Key::Unnumbered, &[("n", "3")]), "+3 layers without a number");
        assert_eq!(tf(Lang::Pt, Key::Unnumbered, &[("n", "3")]), "+3 camadas sem número");
        assert_eq!(tf(Lang::Pt, Key::DefaultLine, &[("pairs", "Size=Small"), ("source", "salva")]), "Variante padrão: Size=Small (salva)");
    }

    #[test]
    fn override_categories_fall_back_to_the_raw_name() {
        assert_eq!(override_name(Lang::Pt, "corners"), "cantos");
        assert_eq!(override_name(Lang::Pt, "strokes"), "traços");
        assert_eq!(override_name(Lang::Pt, "somethingNew"), "somethingNew");
    }

    #[test]
    fn strings_doc_is_current() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/i18n-strings.md");
        let want = markdown();
        if std::env::var("I18N_WRITE").is_ok() {
            std::fs::write(path, &want).unwrap();
        }
        assert_eq!(std::fs::read_to_string(path).unwrap(), want, "run with I18N_WRITE=1 to rewrite docs/i18n-strings.md");
    }
}
