//! The glyphs of the report: one for each kind of layer, each property and each state.
//!
//! A glyph never stands alone: it comes first and the word follows ([`with_word`]).

use figcraft_plugin_sdk::Value;

/// Main component.
pub const MAIN_COMPONENT: &str = "❖";
/// Variant of a component set.
pub const VARIANT: &str = "◆";
/// Instance.
pub const INSTANCE: &str = "◇";
/// Frame.
pub const FRAME: &str = "⌗";
/// Frame with auto layout.
pub const FRAME_AUTO: &str = "⫿⫿";
/// Group.
pub const GROUP: &str = "⬚";
/// Text.
pub const TEXT: &str = "T";
/// Rectangle.
pub const RECTANGLE: &str = "□";
/// Ellipse.
pub const ELLIPSE: &str = "〇";

/// A boolean component property.
pub const BOOLEAN: &str = "↔";

/// Padding.
pub const PADDING: &str = "⧈";
/// Gap.
pub const GAP: &str = "]𝍠[";
/// Fill.
pub const FILL: &str = "⬙";
/// Stroke.
pub const STROKE: &str = "━";
/// Radius.
pub const RADIUS: &str = "⛶";

/// A layer inside another (the prefix of its name).
pub const NESTED: &str = "↳";

/// "✓ token".
pub const OK: &str = "✓";
/// "✕ missing".
pub const MISSING: &str = "✕";
/// "! suspect".
pub const SUSPECT: &str = "!";

/// `glyph` and `word`, the glyph first, a space between.
pub fn with_word(glyph: &str, word: &str) -> String {
    format!("{glyph} {word}")
}

/// The glyph of a layer as `node.get` gives it, or none when its kind isn't mapped. `root` is the
/// component itself (a variant when `variant` says so); a frame has the auto layout glyph when it
/// lays out its children.
pub fn layer(node: &Value, root: bool, variant: bool) -> Option<&'static str> {
    let kind = node.get("type").and_then(Value::as_str)?;
    match kind {
        _ if root && matches!(kind, "COMPONENT" | "INSTANCE") => Some(if variant { VARIANT } else { MAIN_COMPONENT }),
        "COMPONENT" => Some(MAIN_COMPONENT),
        "INSTANCE" => Some(INSTANCE),
        "FRAME" => {
            let auto = matches!(node.get("layout").and_then(|l| l.get("mode")).and_then(Value::as_str), Some("horizontal" | "vertical" | "grid"));
            Some(if auto { FRAME_AUTO } else { FRAME })
        }
        "GROUP" => Some(GROUP),
        "TEXT" => Some(TEXT),
        "RECTANGLE" => Some(RECTANGLE),
        "ELLIPSE" => Some(ELLIPSE),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figcraft_plugin_sdk::json;

    /// Every constant, character by character, as code points.
    #[test]
    fn each_glyph_is_exactly_the_agreed_string() {
        let expected: [(&str, &[u32]); 19] = [
            (MAIN_COMPONENT, &[0x2756]),
            (VARIANT, &[0x25C6]),
            (INSTANCE, &[0x25C7]),
            (FRAME, &[0x2317]),
            (FRAME_AUTO, &[0x2AFF, 0x2AFF]),
            (GROUP, &[0x2B1A]),
            (TEXT, &[0x54]),
            (RECTANGLE, &[0x25A1]),
            (ELLIPSE, &[0x3007]),
            (BOOLEAN, &[0x2194]),
            (PADDING, &[0x29C8]),
            (GAP, &[0x5D, 0x1D360, 0x5B]),
            (FILL, &[0x2B19]),
            (STROKE, &[0x2501]),
            (RADIUS, &[0x26F6]),
            (NESTED, &[0x21B3]),
            (OK, &[0x2713]),
            (MISSING, &[0x2715]),
            (SUSPECT, &[0x21]),
        ];
        for (got, want) in expected {
            let chars: Vec<u32> = got.chars().map(u32::from).collect();
            assert_eq!(chars, want, "{got:?}");
        }
    }

    #[test]
    fn the_gap_glyph_has_no_backslashes() {
        assert!(!GAP.contains('\\'));
        assert_eq!(GAP.chars().count(), 3);
    }

    #[test]
    fn a_glyph_is_never_alone() {
        assert_eq!(with_word(PADDING, "Padding"), "⧈ Padding");
        assert_eq!(with_word(OK, "token"), "✓ token");
    }

    #[test]
    fn layers_map_to_their_glyph_and_unknown_kinds_to_none() {
        let t = |k: &str| json!({"type": k});
        assert_eq!(layer(&t("INSTANCE"), false, false), Some(INSTANCE));
        assert_eq!(layer(&t("INSTANCE"), true, false), Some(MAIN_COMPONENT));
        assert_eq!(layer(&t("COMPONENT"), true, true), Some(VARIANT));
        assert_eq!(layer(&t("FRAME"), false, false), Some(FRAME));
        assert_eq!(layer(&json!({"type": "FRAME", "layout": {"mode": "vertical"}}), false, false), Some(FRAME_AUTO));
        assert_eq!(layer(&json!({"type": "FRAME", "layout": {"mode": null}}), false, false), Some(FRAME));
        assert_eq!(layer(&t("GROUP"), false, false), Some(GROUP));
        assert_eq!(layer(&t("TEXT"), false, false), Some(TEXT));
        assert_eq!(layer(&t("RECTANGLE"), false, false), Some(RECTANGLE));
        assert_eq!(layer(&t("ELLIPSE"), false, false), Some(ELLIPSE));
        assert_eq!(layer(&t("VECTOR"), false, false), None);
        assert_eq!(layer(&json!({}), false, false), None);
    }
}
