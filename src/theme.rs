//! The report's colours and type sizes, in one place.

/// The report's background.
pub const BG: &str = "#232323";
/// The stage, and the cards (the token chips).
pub const PANEL: &str = "#14213D";
/// The border of the stage and the cards, and the divider: #E5E5E5 at 20% opacity.
pub const BORDER: &str = "#E5E5E533";
/// Main text.
pub const TEXT: &str = "#FFFFFF";
/// Secondary text and headings.
pub const MUTED: &str = "#E5E5E5";
/// The second line of a property (value and state): the muted text at 70%.
pub const MUTED_70: &str = "#E5E5E5B3";
/// The redlines have a fixed palette, by stage and by section (no colour setting): on the light stage the anatomy is
/// #C54600 (3.92:1 on #E5E5E5) and the spacing #007D00 (4.24:1); on the dark stage the anatomy is #FCA311 (7.90:1 on
/// #14213D) and the spacing #4ADE80 (9.17:1).
pub const LIGHT_ANATOMY: &str = "#C54600";
pub const LIGHT_SPACING: &str = "#007D00";
pub const DARK_ANATOMY: &str = "#FCA311";
pub const DARK_SPACING: &str = "#4ADE80";
/// The number inside a marker or badge on the dark stage's colours (7.77:1 on #FCA311, 9.02:1 on #4ADE80).
pub const ON_REDLINE: &str = "#232323";
/// The number inside a marker or badge on the light stage's colours (4.94:1 on #C54600, 5.34:1 on #007D00; the dark
/// number would be 3.18:1 and 2.94:1).
pub const ON_LIGHT_ACCENT: &str = "#FFFFFF";
/// The warning band's outline: orange, whatever the palette (the band sits on the report's dark background).
pub const WARN_LINE: &str = "#FCA311";
/// How opaque the fill of the measure rectangles is, in percent (the spacing colour at this opacity).
pub const MEASURE_FILL_PERCENT: u8 = 30;
/// The warning band behind the collision notice: the redline at 12% opacity.
pub const WARN_BG: &str = "#FCA3111F";
/// The font, and the least size of any text.
pub const FONT_FAMILY: &str = "Inter";
pub const MIN_SIZE: f64 = 14.0;
/// Sizes of the title of the report, of a section and of the metadata line.
pub const TITLE_SIZE: f64 = 32.0;
pub const SECTION_SIZE: f64 = 24.0;
/// The light stage (Alabaster).
pub const STAGE_LIGHT: &str = "#E5E5E5";
/// The redlines on the light stage (Prussian blue), with the default line colour.
pub const PRUSSIAN: &str = "#14213D";
/// The ring around a badge on the light stage.
pub const RING_WEIGHT: f64 = 1.5;
/// A variant row of the axes sections is a block: this fill, this padding on its four sides and this radius (its border is `BORDER`).
pub const BLOCK_FILL: &str = "#2E2E2E";
pub const BLOCK_PAD: f64 = 20.0;
pub const BLOCK_RADIUS: f64 = 12.0;
/// The weight of its border, which the host counts in the layout like padding.
pub const BLOCK_BORDER: f64 = 1.0;
