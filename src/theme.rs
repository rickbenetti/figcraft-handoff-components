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
/// The redlines (markers, guides, measures, overlay) unless the settings say otherwise.
pub const REDLINE: &str = "#FCA311";
/// The overlay's fill unless the settings say otherwise: the redline at 30% opacity.
pub const REDLINE_FILL: &str = "#FCA3114D";
/// The number inside a marker or badge (white on orange is 2.0:1).
pub const ON_REDLINE: &str = "#232323";
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
