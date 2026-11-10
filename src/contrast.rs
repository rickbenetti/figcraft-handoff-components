//! WCAG contrast between a component's visible colours and a stage, for the "auto" stage.
//!
//! The rule (docs/stage-auto.md): the component's effective background is its root's solid fill, else
//! the first ancestor's, else none. With a background, that colour is what meets the stage; without
//! one, the colours of the visible elements (text, icons, strokes, fills) do. Of the light stage
//! (#E5E5E5) and the dark one (#14213D) the stage is the one that maximises the lowest contrast of
//! those colours with it; a tie is light. Colours with transparency are composited over the stage
//! first; the contrast ratio is (L1 + 0.05) / (L2 + 0.05) with L the relative luminance of sRGB.

/// An sRGB colour with alpha, channels 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

/// The light stage, #E5E5E5.
pub const LIGHT_STAGE: Rgba = Rgba { r: 229.0 / 255.0, g: 229.0 / 255.0, b: 229.0 / 255.0, a: 1.0 };

/// The dark stage, #14213D.
pub const DARK_STAGE: Rgba = Rgba { r: 20.0 / 255.0, g: 33.0 / 255.0, b: 61.0 / 255.0, a: 1.0 };

/// `#RGB`, `#RRGGBB` or `#RRGGBBAA`; `opacity` multiplies the alpha.
pub fn parse_hex(text: &str, opacity: f64) -> Option<Rgba> {
    let h = text.trim().strip_prefix('#')?;
    let (r, g, b, a) = match h.len() {
        3 => {
            let v: Vec<u8> = h.chars().map(|c| c.to_digit(16).map(|d| (d * 17) as u8)).collect::<Option<_>>()?;
            (v[0], v[1], v[2], 255)
        }
        6 | 8 => {
            let byte = |i: usize| h.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok());
            (byte(0)?, byte(2)?, byte(4)?, if h.len() == 8 { byte(6)? } else { 255 })
        }
        _ => return None,
    };
    let f = |v: u8| f64::from(v) / 255.0;
    Some(Rgba { r: f(r), g: f(g), b: f(b), a: (f(a) * opacity).clamp(0.0, 1.0) })
}

/// The colour as it looks over `under` (alpha dropped).
pub fn composite(top: Rgba, under: Rgba) -> Rgba {
    let mix = |t: f64, u: f64| t * top.a + u * (1.0 - top.a);
    Rgba { r: mix(top.r, under.r), g: mix(top.g, under.g), b: mix(top.b, under.b), a: 1.0 }
}

fn channel(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// Relative luminance.
pub fn luminance(c: Rgba) -> f64 {
    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

/// The contrast ratio of two opaque colours (1 to 21).
pub fn ratio(a: Rgba, b: Rgba) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// The lowest contrast of `colours` (each composited over `stage` when transparent) with `stage`.
pub fn lowest_against(colours: &[Rgba], stage: Rgba) -> Option<f64> {
    colours.iter().map(|c| ratio(composite(*c, stage), stage)).fold(None, |m: Option<f64>, r| Some(m.map_or(r, |m| m.min(r))))
}

/// What the auto stage decides: whether it is dark, and the lowest contrast on the stage chosen
/// (None when there is nothing to judge: light).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pick {
    pub dark: bool,
    pub lowest: Option<f64>,
}

/// The stage for a component with `backdrop` (its effective background, if it has one) and the
/// colours of its visible `elements`.
pub fn pick_stage(backdrop: Option<Rgba>, elements: &[Rgba]) -> Pick {
    let judged: Vec<Rgba> = match backdrop {
        Some(b) => vec![b],
        None => elements.to_vec(),
    };
    let (Some(light), Some(dark)) = (lowest_against(&judged, LIGHT_STAGE), lowest_against(&judged, DARK_STAGE)) else {
        return Pick { dark: false, lowest: None };
    };
    if dark > light + 1e-9 { Pick { dark: true, lowest: Some(dark) } } else { Pick { dark: false, lowest: Some(light) } }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Rgba {
        parse_hex(s, 1.0).unwrap()
    }

    #[test]
    fn black_on_white_is_twenty_one() {
        assert!((ratio(hex("#000000"), hex("#FFFFFF")) - 21.0).abs() < 1e-9);
    }

    #[test]
    fn white_text_on_nothing_picks_the_dark_stage_and_dark_text_the_light() {
        assert!(pick_stage(None, &[hex("#FFFFFF")]).dark);
        assert!(!pick_stage(None, &[hex("#171719")]).dark);
        // The lowest contrast decides: white and near-black together, the dark stage keeps the white readable.
        let both = pick_stage(None, &[hex("#FFFFFF"), hex("#171719")]);
        assert!(both.lowest.unwrap() > 1.0);
        // Nothing to judge, or a tie: light.
        assert_eq!(pick_stage(None, &[]), Pick { dark: false, lowest: None });
    }

    #[test]
    fn a_background_is_what_meets_the_stage() {
        // White text on a blue button: the blue is judged, not the text; the blue reads on the light stage.
        assert!(!pick_stage(Some(hex("#3B5BDB")), &[hex("#FFFFFF")]).dark);
        // A white card on the stage: the dark one separates it better.
        assert!(pick_stage(Some(hex("#FFFFFF")), &[hex("#171719")]).dark);
    }

    #[test]
    fn transparent_colours_are_composited_over_the_stage_first() {
        // Black at 10% over #E5E5E5 is close to the stage; at 100% it is far.
        let faint = parse_hex("#000000", 0.1).unwrap();
        assert!(lowest_against(&[faint], LIGHT_STAGE).unwrap() < 2.0);
        assert!(lowest_against(&[parse_hex("#000000", 1.0).unwrap()], LIGHT_STAGE).unwrap() > 10.0);
        // The alpha of an 8-digit colour counts too.
        assert!((parse_hex("#00000080", 1.0).unwrap().a - 128.0 / 255.0).abs() < 1e-9);
        // White at 50% over the dark stage is still readable there.
        assert!(pick_stage(None, &[parse_hex("#FFFFFF", 0.5).unwrap()]).dark);
    }

    #[test]
    fn hex_forms() {
        assert_eq!(parse_hex("#FFF", 1.0), parse_hex("#FFFFFF", 1.0));
        assert_eq!(parse_hex("FFF", 1.0), None);
        assert_eq!(parse_hex("#GGGGGG", 1.0), None);
    }
}
