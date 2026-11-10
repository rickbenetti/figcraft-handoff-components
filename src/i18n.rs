//! Every text the "gerar" command shows or writes: the report, the layer names it makes, the notes
//! and the errors. One table, one column per language.
//!
//! To add a language: add a variant to [`Lang`], bump [`LANGS`] and teach [`Lang::parse`] its names;
//! then give the rows a text for it. A row with fewer texts than languages uses its first (English)
//! for the rest, so a text can be added in English now and translated later.
//!
//! The plugin host gives a plugin no locale (the SDK's `Input` has none and the manifest takes one
//! label per field), so the language comes from the `lang` parameter.

/// Number of languages in the table (the strings document has a column for each).
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
    /// The column of the table.
    fn index(self) -> usize {
        match self {
            Lang::En => 0,
            Lang::Pt => 1,
        }
    }

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

    /// The language a `lang` value names, or none: `en`, `pt`, `pt-BR`… and nothing else.
    pub fn strict(text: &str) -> Option<Lang> {
        let lower = text.trim().to_lowercase();
        let head = lower.split(['-', '_', '.', '@', ' ']).next().unwrap_or_default();
        match head {
            "pt" | "por" | "ptbr" | "portuguese" | "português" | "portugues" | "brazilian" => Some(Lang::Pt),
            "en" | "eng" | "english" | "inglês" | "ingles" => Some(Lang::En),
            _ => None,
        }
    }

    /// The code of the language, as the table's documentation names it.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Pt => "pt",
        }
    }

    /// Every language, in column order (for the strings document).
    #[cfg(test)]
    pub const ALL: [Lang; LANGS] = [Lang::En, Lang::Pt];
}

macro_rules! table {
    ($($key:ident => [$($text:expr),+ $(,)?];)+) => {
        /// The key of a text.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Key {
            $($key),+
        }

        impl Key {
            /// Every key, in table order (for the strings document).
            #[cfg(test)]
            pub const ALL: &'static [Key] = &[$(Key::$key),+];

            /// The key's name (for the strings document).
            #[cfg(test)]
            pub fn name(self) -> &'static str {
                match self {
                    $(Key::$key => stringify!($key)),+
                }
            }

            /// The texts of the key, one per language in column order; a row may stop early, and the
            /// languages it leaves out use its first text (English).
            fn row(self) -> &'static [&'static str] {
                match self {
                    $(Key::$key => &[$($text),+]),+
                }
            }
        }
    };
}

// Placeholders are written `{name}`; see `tf`.
table! {
    // Layer names and visible texts of the report.
    FrameName => ["Report — {name}", "Relatório — {name}"];
    Anatomy => ["Anatomy", "Anatomia"];
    LayerTitle => ["Component", "Componente"];
    LayerSubtitle => ["Version · date · status", "Versão · data · status"];
    LayerStage => ["Stage", "Palco"];
    LayerMeasure => ["Measure {value}", "Medida {value}"];
    LayerValue => ["Value {value}", "Valor {value}"];
    LayerNote => ["Note", "Nota"];
    LayerSummary => ["Summary", "Resumo"];
    LayerMore => ["More", "Mais"];
    LayerAnatomyRow => ["Anatomy layout", "Layout da anatomia"];
    LayerMarker => ["Marker {n}", "Marcador {n}"];
    LayerLeader => ["Leader {n}", "Guia {n}"];
    LayerWarning => ["Warning", "Aviso"];
    // Table columns.
    // Property labels.
    PropRadius => ["Radius", "Raio"];
    PropGap => ["Gap", "Espaçamento"];
    PropPadH => ["Horizontal padding", "Padding horizontal"];
    PropPadV => ["Vertical padding", "Padding vertical"];
    PropFill => ["Fill", "Preenchimento"];
    PropStroke => ["Stroke", "Contorno"];
    // States.
    // Summary, warning and notes.
    Summary => [
        "{bound} bound · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties",
        "{bound} vinculadas · {unbound} sem token · {missing} ausentes · {suspect} suspeitas · de {total} propriedades"
    ];
    SummaryChecks => [
        "{bound} bound ({check} to check) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties",
        "{bound} vinculadas ({check} a verificar) · {unbound} sem token · {missing} ausentes · {suspect} suspeitas · de {total} propriedades"
    ];
    // New in 0.9.3: English only (Portuguese falls back to English).
    SummaryStyles => ["{bound} bound ({vars} variables, {styles} styles) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties"];
    SummaryStylesChecks => ["{bound} bound ({vars} variables, {styles} styles; {check} to check) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties"];
    PropTypography => ["Typography"];
    LegendStyle => ["style"];
    LegendNoStyle => ["no style"];
    Collision => [
        "Warning: {n} bindings with an incompatible type. This component probably came from another file; numeric tokens may be wrong.",
        "Atenção: {n} vínculos com tipo incompatível. Este componente provavelmente veio de outro arquivo; tokens numéricos podem estar errados."
    ];
    OverrideNote => [
        "Instance with overrides: {list}; generated from the component",
        "Instância com overrides: {list}; gerado do componente"
    ];
    Variants => ["{n} variants; first one shown", "{n} variantes; mostrada a primeira"];
    NoteExport1x => [
        "This report is taller than 8,000 px, so its PNG export is set to 1x (at 2x it would pass 16,384 px).",
        "Este relatório tem mais de 8.000 px de altura, então a exportação em PNG está em 1x (em 2x passaria de 16.384 px)."
    ];
    InstanceOf => ["Instance of {name}", "Instância de {name}"];
    LayerKind => ["Layer kind", "Tipo da camada"];
    Unnumbered => ["+{n} layers without a number", "+{n} camadas sem número"];
    MoreLines => ["+{n} lines not shown", "+{n} linhas não exibidas"];
    NoteLineNoCoords => ["measure.padding line {i} has no coordinates", "a linha {i} do measure.padding não tem coordenadas"];
    NoteLineOblique => [
        "measure.padding line {i} (\"{label}\") is neither horizontal nor vertical",
        "a linha {i} (\"{label}\") do measure.padding não é horizontal nem vertical"
    ];
    Done => ["Report generated: {name}", "Relatório gerado: {name}"];
    // Override categories the app reports.
    OvCorners => ["corners", "cantos"];
    OvPadding => ["padding", "padding"];
    OvSize => ["size", "tamanho"];
    OvPosition => ["position", "posição"];
    OvStrokes => ["strokes", "contornos"];
    OvTextStyle => ["text style", "estilo de texto"];
    OvFills => ["fills", "preenchimentos"];
    OvEffects => ["effects", "efeitos"];
    // New in 0.8.0: English only (the other languages use it).
    LayerHeaderBlock => ["Header"];
    LayerAnatomySection => ["Anatomy section"];
    LayerDivider => ["Divider"];
    LayerLegend => ["Legend"];
    LayerGroup => ["Legend: {name}"];
    LayerBadge => ["Badge {n}"];
    LayerChip => ["Token chip"];
    LegendToken => ["token"];
    LegendCheck => ["token (check)"];
    LegendSuspect => ["suspect"];
    LegendMissing => ["missing"];
    LegendNone => ["no token"];
    // New in 0.8.1: English only.
    SpacingTitle => ["Spacing"];
    LayerSpacingSection => ["Spacing section"];
    LayerBody => ["Properties"];
    LayerBlockHeader => ["Block header"];
    SpDirection => ["Direction"];
    SpAlignment => ["Alignment"];
    DirHorizontal => ["Horizontal"];
    DirVertical => ["Vertical"];
    DirGrid => ["Grid"];
    // New in 0.9.0: English only.
    PropertiesTitle => ["Properties"];
    LayerProperties => ["Properties section"];
    LayerPropertyRow => ["Property: {name}"];
    LayerAxis => ["Axis: {name}"];
    LayerAxisRow => ["Variant: {name}"];
    LayerAxisNote => ["Note"];
    TypeVariant => ["Variant"];
    TypeBoolean => ["Boolean"];
    TypeText => ["Text"];
    TypeInstance => ["Instance"];
    ValueTrue => ["True"];
    ValueFalse => ["False"];
    LegendDefault => ["Default"];
    DefaultLine => ["Default: {pairs} ({source})"];
    SourceSaved => ["saved"];
    SourceFirst => ["first variant in the set"];
    NotDefined => ["Not defined: {list}"];
    MoreVariants => ["+{n} variants not shown"];
    ErrNoVariants => ["This component has no variants, so it has no default variant"];
    ErrDefaultMalformed => ["\"{text}\" is not Property=Option (for example Size=Small, State=Enabled)"];
    ErrDefaultProp => ["\"{prop}\" is not a variant property. Valid: {valid}"];
    ErrDefaultOption => ["\"{value}\" is not an option of {prop}. Valid: {valid}"];
    ErrDefaultNoVariant => ["No variant is named \"{name}\""];
    // New in 0.9.1: English only.
    ErrBadStage => ["stage must be light, dark or auto, not \"{value}\""];
    SpHResizing => ["Horizontal resizing"];
    SpVResizing => ["Vertical resizing"];
    ResizeHug => ["Hug"];
    ResizeFixed => ["Fixed"];
    ResizeFill => ["Fill"];
    AlignTop => ["Top"];
    AlignMiddle => ["Middle"];
    AlignBottom => ["Bottom"];
    AlignLeft => ["Left"];
    AlignCenter => ["Center"];
    AlignRight => ["Right"];
    AlignSpaceBetween => ["Space between"];
    AlignSpaceAround => ["Space around"];
    AlignSpaceEvenly => ["Space evenly"];
    SourceComponent => ["component default"];
    // New in 0.9.4: English only. What a variant changes from the default variant.
    DeltaWidth => ["Width"];
    DeltaHeight => ["Height"];
    DeltaStrokeWeight => ["Stroke weight"];
    DeltaOpacity => ["Opacity"];
    DeltaEffects => ["Effects"];
    DeltaContent => ["Text"];
    DeltaHidden => ["hidden"];
    DeltaVisible => ["visible"];
    DeltaMore => ["+{n} more differences"];
    DeltaNone => ["No differences from default"];
    DeltaEffectOnly => ["No readable differences (visual effect only)"];
    LayerDeltas => ["Differences"];
    // Errors.
    ErrNoSelection => ["Select a component or an instance", "Selecione um componente ou uma instância"];
    ErrMetaSelection => [
        "Select the component or instance whose version, date or status you are setting",
        "Selecione o componente ou a instância cuja versão, data ou status você está definindo"
    ];
    ErrBadLang => ["language must be en or pt, not \"{value}\"", "o idioma deve ser en ou pt, não \"{value}\""];
    ErrBadRerun => ["on rerun must be replace or new, not \"{value}\"", "ao repetir deve ser replace ou new, não \"{value}\""];
    ErrBadColour => ["\"{value}\" is not a hex colour like #FF4D6D", "\"{value}\" não é uma cor hexadecimal como #FF4D6D"];
    SettingsSaved => ["Settings saved", "Configurações salvas"];
    SettingsKept => ["Nothing to change", "Nada a alterar"];
    ErrNotComponent => [
        "layer {id} is a {kind}, not a component or an instance",
        "a camada {id} é {kind}, não um componente nem uma instância"
    ];
    ErrNoMain => ["instance {id} has no main component", "a instância {id} não tem componente principal"];
    ErrEmptySet => ["component set {id} has no variants", "o conjunto de componentes {id} não tem variantes"];
    ErrCall => ["{cmd}: {reason}", "{cmd}: {reason}"];
    ErrNoId => ["{what}: no id in the reply", "{what}: sem id na resposta"];
    ErrNoPage => ["page.list: no page", "page.list: nenhuma página"];
    ErrNotList => ["page.list: the reply isn't a list", "page.list: a resposta não é uma lista"];
    ErrParent => [
        "instance.place put the instance in {parent}, not in the stage {stage}",
        "instance.place colocou a instância em {parent}, não no palco {stage}"
    ];
}

/// The text of `key` in `lang`.
pub fn t(lang: Lang, key: Key) -> &'static str {
    let row = key.row();
    row.get(lang.index()).copied().filter(|s| !s.is_empty()).or_else(|| row.first().copied()).unwrap_or_default()
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

/// The table as Markdown: `key | EN | PT`, for `docs/i18n-strings.md`.
#[cfg(test)]
pub fn markdown() -> String {
    let cell = |s: &str| s.replace('|', "\\|");
    let mut out = String::from("# Handoff Components: strings\n\n");
    out.push_str("Generated from `src/i18n.rs` (the test `i18n::tests::strings_doc_is_current` checks it; run it with\n");
    out.push_str("`I18N_WRITE=1` to rewrite this file). Placeholders are `{name}`. The language is the `lang`\n");
    out.push_str("parameter of the `gerar` command (`en` by default, `pt` or `pt-BR` for Portuguese).\n\n");
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
        let row = key.row();
        for i in 0..LANGS {
            match row.get(i) {
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

    #[test]
    fn language_names() {
        for pt in ["pt", "PT", "pt-BR", "pt_br", " Pt-br ", "português", "Portuguese"] {
            assert_eq!(Lang::parse(pt), Lang::Pt, "{pt}");
        }
        for en in ["", "en", "EN", "en-US", "fr", "xx"] {
            assert_eq!(Lang::parse(en), Lang::En, "{en}");
        }
    }

    #[test]
    fn every_text_is_filled_and_placeholders_match() {
        for key in Key::ALL {
            let row = key.row();
            assert!(row.len() <= LANGS, "{} has more texts than languages", key.name());
            for (i, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{} has no text in column {i}", key.name());
            }
            let names = |s: &str| {
                let mut v: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split('}').next()).map(ToString::to_string).collect();
                v.sort();
                v
            };
            for text in &row[1..] {
                assert_eq!(names(row[0]), names(text), "{} placeholders differ", key.name());
            }
        }
    }

    #[test]
    fn a_language_a_row_leaves_out_uses_english() {
        assert_eq!(t(Lang::Pt, Key::LegendSuspect), "suspect");
        assert_eq!(t(Lang::En, Key::LegendSuspect), "suspect");
        assert_eq!(t(Lang::Pt, Key::Anatomy), "Anatomia");
    }

    #[test]
    fn replaces_placeholders() {
    }

    #[test]
    fn override_categories_fall_back_to_the_raw_name() {
        assert_eq!(override_name(Lang::Pt, "corners"), "cantos");
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
