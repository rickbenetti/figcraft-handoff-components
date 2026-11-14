//! The stacked format of a change in a variant's list: the label in bold, then a line of tokens (chips), then a line of values
//! (a colour's square and hex, or a number or a text), the old side, an arrow and the new side lined up in columns. Pure
//! planning (which layout fits the legend, how many nodes it takes, how many the run may spend); `gerar.rs` draws it.

use crate::delta::{Cell, Cells, DASH};

/// The room a block gives its lines, in px: the legend's width (the same as `gerar.rs`'s `LEGEND_W`) less the block's padding.
pub const WIDTH: f64 = 520.0 - 2.0 * crate::theme::STRIPE_PAD;
/// What a character of the 15 px text takes, in px (an estimate: only the entries near the limit are measured).
pub const CHAR_W: f64 = 7.7;
/// What a chip adds to its text: the padding (8 + 8) and the border (1 + 1).
pub const CHIP_EXTRA: f64 = 18.0;
/// The colour's square and the gap before its hex; the gap between columns; the arrow.
pub const SWATCH: f64 = 16.0;
pub const SWATCH_GAP: f64 = 6.0;
pub const COL_GAP: f64 = 8.0;
pub const ARROW_W: f64 = 15.0;
/// An estimate this many times the width is trusted: such an entry is not drawn in the grid to be measured. Every other grid is
/// drawn and measured (the estimate is only the average character; a token of wide letters can be far wider than it says).
pub const OBVIOUS: f64 = 1.3;
/// The most nodes the stacked format may create in a run (its changes).
pub const BUDGET: usize = 400;

/// How a change is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// A grid of two lines (tokens, values) and three columns (old, arrow, new); it is measured once drawn.
    Grid,
    /// The pair of tokens does not fit one line: the old chip, then the arrow with the new chip, then the values on one line.
    Fallback,
    /// The values do not fit one line: the change stays one line of text.
    Line,
}

fn est(s: &str) -> f64 {
    s.chars().count() as f64 * CHAR_W
}

fn has_tokens(c: &Cells) -> bool {
    c.from.token.is_some() || c.to.token.is_some()
}

fn token_w(c: &Cell) -> f64 {
    c.token.as_deref().map_or_else(|| est(DASH), |t| est(t) + CHIP_EXTRA)
}

fn value_w(c: &Cell) -> f64 {
    est(&c.value) + if c.colour.is_some() { SWATCH + SWATCH_GAP } else { 0.0 }
}

/// The estimated width of the values on one line (old, arrow, new).
pub fn values_width(c: &Cells) -> f64 {
    value_w(&c.from) + COL_GAP + ARROW_W + COL_GAP + value_w(&c.to)
}

/// The estimated width of the grid: each column as wide as the wider of its two lines.
pub fn grid_width(c: &Cells) -> f64 {
    let col = |cell: &Cell| if has_tokens(c) { token_w(cell).max(value_w(cell)) } else { value_w(cell) };
    col(&c.from) + COL_GAP + ARROW_W + COL_GAP + col(&c.to)
}

/// The layout to try first for a change (see [`Layout`]): the grid, unless the estimate says it is far too wide for the legend.
pub fn plan(c: &Cells) -> Layout {
    if values_width(c) > WIDTH * OBVIOUS {
        Layout::Line
    } else if grid_width(c) > WIDTH * OBVIOUS && has_tokens(c) {
        Layout::Fallback
    } else {
        Layout::Grid
    }
}

/// What a layout becomes when its measure says it overflows the legend: the grid gives way to the arrow on the second line (when
/// there are tokens to move), and that to a line of text.
pub fn after_overflow(c: &Cells, layout: Layout) -> Layout {
    match layout {
        Layout::Grid if has_tokens(c) => Layout::Fallback,
        _ => Layout::Line,
    }
}

fn cell_cost(c: &Cell) -> usize {
    if c.colour.is_some() { 3 } else { 1 }
}

fn token_cost(c: &Cell) -> usize {
    if c.token.is_some() { 2 } else { 1 }
}

/// The nodes a change takes in a layout: its frame (which carries the divider), its label and the lines.
pub fn cost(c: &Cells, layout: Layout) -> usize {
    let values = cell_cost(&c.from) + 1 + cell_cost(&c.to);
    match layout {
        Layout::Grid => {
            let tokens = if has_tokens(c) { token_cost(&c.from) + 1 + token_cost(&c.to) } else { 0 };
            2 + 1 + tokens + values
        }
        // The frame, the label, the old chip, the row with the arrow and the new chip, and the row of values.
        Layout::Fallback => 2 + token_cost(&c.from) + 1 + 1 + token_cost(&c.to) + 1 + values,
        // One line of text in a block's frame: not counted in the budget (the budget is for the stacked format).
        Layout::Line => 0,
    }
}

/// The colour of a group's heading, and of a layer's title in a line of a layer shown, hidden or swapped: the secondary text colour,
/// at full strength on the stripe's fill (the dimmer one would not keep its contrast there) and dimmed on a block with no fill.
pub fn title_colour(filled: bool) -> &'static str {
    if filled { crate::theme::MUTED } else { crate::theme::MUTED_70 }
}

/// The block each line of a list is in. `lines` says for each line whether it is a heading (`.0`) and whether it is a change of a
/// property (`.1`). A block is a group's heading with the first change right under it, or one line on its own (a change of the
/// component itself, a layer shown or hidden, an instance swapped). The blocks are numbered from 0 in order.
pub fn blocks(lines: &[(bool, bool)]) -> Vec<usize> {
    let mut block = 0;
    let mut under_heading = false;
    lines
        .iter()
        .enumerate()
        .map(|(i, &(heading, entry))| {
            if i > 0 && !(under_heading && entry) {
                block += 1;
            }
            under_heading = heading;
            block
        })
        .collect()
}

/// The nodes the run may still spend on the stacked format. Once an entry does not fit, it and every later one are one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub limit: usize,
    pub used: usize,
    pub exhausted: bool,
    /// How many entries were measured, and how many of those overflowed and were drawn again.
    pub measured: usize,
    pub redrawn: usize,
}

impl Budget {
    pub fn new() -> Budget {
        Budget { limit: BUDGET, used: 0, exhausted: false, measured: 0, redrawn: 0 }
    }

    /// Spend `nodes`; false (and the budget is closed for good) when they do not fit.
    pub fn take(&mut self, nodes: usize) -> bool {
        if self.exhausted || self.used + nodes > self.limit {
            self.exhausted = true;
            return false;
        }
        self.used += nodes;
        true
    }

    /// Spend `nodes` that were already created (a measured entry that fell back to a costlier layout).
    pub fn spend(&mut self, nodes: usize) {
        self.used += nodes;
    }
}

impl Default for Budget {
    fn default() -> Self {
        Budget::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::delta::Swatch;

    fn cell(token: Option<&str>, value: &str, colour: Option<Swatch>) -> Cell {
        Cell { token: token.map(ToString::to_string), value: value.to_string(), colour }
    }

    fn cells(from: Cell, to: Cell) -> Cells {
        Cells { label: "Fill".to_string(), from, to }
    }

    fn fill(a: &str, b: &str) -> Cells {
        cells(cell(Some(a), "#C6005C", Some(Swatch::Solid)), cell(Some(b), "#F4F4F5", Some(Swatch::Solid)))
    }

    #[test]
    fn a_pair_is_a_grid_to_be_measured_and_only_an_obvious_overflow_is_not_drawn() {
        assert_eq!(plan(&fill("Colors/color/primary", "Colors/color/secondary")), Layout::Grid);
        // Two 33-character tokens: 33 × 7.7 + 18 = 272 px each, 272 + 8 + 15 + 8 + 272 = 575 px, 10% over 520: still drawn, and measured.
        let long = fill("Colors/color/secondary-foreground", "Colors/color/secondary-foreground");
        assert!(grid_width(&long) > WIDTH && grid_width(&long) < WIDTH * OBVIOUS);
        assert_eq!(plan(&long), Layout::Grid);
        // Far over: the arrow goes to the second line without drawing the grid first.
        let huge = fill(&"x".repeat(60), &"y".repeat(60));
        assert_eq!(plan(&huge), Layout::Fallback);
    }

    #[test]
    fn values_that_do_not_fit_one_line_leave_the_change_as_text() {
        let typo = cells(cell(Some("text/h3"), "Inter 24/32 SemiBold, letter spacing -1% and more words to be sure", None), cell(None, "Inter 24/32 SemiBold, letter spacing -1% and more", None));
        assert_eq!(plan(&typo), Layout::Line);
    }

    #[test]
    fn tokens_of_wide_letters_pass_the_estimate_and_the_measure_decides() {
        // 24 letters m: the estimate says 24 × 7.7 + 18 = 203 px a chip, 437 px the pair, under 520. At 15 px an m is about 13 px
        // wide, so the pair is really about 690 px: only the measure can tell. The grid is drawn, and when it overflows the arrow goes
        // to the second line (the values still fit), and when the values do not fit either, the change is a line.
        let wide = fill(&"m".repeat(24), &"w".repeat(24));
        assert!(grid_width(&wide) < WIDTH);
        assert_eq!(plan(&wide), Layout::Grid);
        assert_eq!(after_overflow(&wide, Layout::Grid), Layout::Fallback);
        assert_eq!(after_overflow(&wide, Layout::Fallback), Layout::Line);
        // No token to move: the grid gives way to a line.
        let colours = cells(cell(None, "#C6005C", Some(Swatch::Solid)), cell(None, "#F4F4F5", Some(Swatch::Solid)));
        assert_eq!(after_overflow(&colours, Layout::Grid), Layout::Line);
    }

    #[test]
    fn a_title_is_the_secondary_colour_at_full_strength_on_a_stripe_and_dimmed_on_a_plain_block() {
        assert_eq!(title_colour(true), "#E5E5E5");
        assert_eq!(title_colour(false), "#E5E5E5B3");
        // On the stripe's fill the full colour has the contrast the dimmed one lacks (4.28 : 1 against #4C4C4C): the full one has 6.8 : 1.
        let lum = |h: &str| {
            let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_or(0.0, |v| f64::from(v) / 255.0);
            let f = |v: f64| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            0.2126 * f(c(1)) + 0.7152 * f(c(3)) + 0.0722 * f(c(5))
        };
        let ratio = |a: &str, b: &str| (lum(a).max(lum(b)) + 0.05) / (lum(a).min(lum(b)) + 0.05);
        assert!(ratio(title_colour(true), crate::theme::STRIPE_FILL) > 6.5);
        assert!(ratio("#B7B7B7", crate::theme::STRIPE_FILL) < 4.5);
    }

    #[test]
    fn a_heading_and_the_change_under_it_are_one_block() {
        // (heading, change of a property)
        let (h, e, o) = ((true, false), (false, true), (false, false));
        assert_eq!(blocks(&[e, e]), [0, 1]);
        assert_eq!(blocks(&[h, e, e]), [0, 0, 1]);
        assert_eq!(blocks(&[e, h, e, e]), [0, 1, 1, 2]);
        // A layer shown or hidden or an instance swapped is a block of one line, after a heading too.
        assert_eq!(blocks(&[e, o, h, e, o]), [0, 1, 2, 2, 3]);
        assert_eq!(blocks(&[o, o]), [0, 1]);
        assert_eq!(blocks(&[]), Vec::<usize>::new());
    }

    #[test]
    fn a_side_with_no_token_is_one_node_where_a_chip_is_two() {
        let two_chips = fill("Colors/color/primary", "Colors/color/secondary");
        // frame + label + grid + (chip 2 + arrow + chip 2) + (colour 3 + arrow + colour 3)
        assert_eq!(cost(&two_chips, Layout::Grid), 15);
        let one_chip = cells(cell(Some("Radius/radius/default"), "10", None), cell(None, "8", None));
        // the dash takes one node; a number is one node
        assert_eq!(cost(&one_chip, Layout::Grid), 2 + 1 + (2 + 1 + 1) + (1 + 1 + 1));
        // no token at all: no line of tokens
        let colours = cells(cell(None, "none", None), cell(None, "#E3E7E8", Some(Swatch::Solid)));
        assert_eq!(cost(&colours, Layout::Grid), 2 + 1 + (1 + 1 + 3));
        assert_eq!(cost(&two_chips, Layout::Fallback), 2 + 2 + 1 + 1 + 2 + 1 + (3 + 1 + 3));
    }

    #[test]
    fn the_budget_closes_for_good_at_the_first_entry_that_does_not_fit() {
        let mut b = Budget { limit: 30, used: 0, exhausted: false, measured: 0, redrawn: 0 };
        assert!(b.take(15));
        assert!(b.take(15));
        assert!(!b.take(1));
        // Closed: even a cheaper entry is a line now.
        let mut c = Budget { limit: 30, used: 20, exhausted: false, measured: 0, redrawn: 0 };
        assert!(!c.take(15));
        assert!(!c.take(1));
        assert_eq!(c.used, 20);
    }

    #[test]
    fn the_worst_run_stays_under_the_budget_of_the_stacked_format() {
        // 24 rows of 12 changes, each the costliest kind: far more than the budget; the budget stops it at 400 nodes.
        let e = fill("Colors/color/primary", "Colors/color/secondary");
        let mut b = Budget::new();
        let mut stacked = 0;
        for _ in 0..24 * 12 {
            if b.take(cost(&e, Layout::Grid)) {
                stacked += 1;
            }
        }
        assert_eq!(stacked, 26);
        assert!(b.used <= BUDGET);
    }
}
