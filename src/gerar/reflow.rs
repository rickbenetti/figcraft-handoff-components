//! The Component Master and the width of the column. The frames of a Section are all 1180 wide; the room the component set has inside the
//! Master is 1180 - 2 x 48 (the frame) - 2 x 48 (the stage) = 988. A set that is wider than that is NOT allowed to widen the Master: its
//! variants are laid out again in a grid inside the set (24 px padding, 48 px between variants, as many columns as fit), which changes the
//! positions of the variants and the size of the user's own component set, nothing else.
//!
//! Only when not even one column fits (a single variant plus 48 is wider than 988) the Master grows, and then every frame of the Section
//! grows with it, to the same width, so the column stays straight.
//!
//! The grid is a pure function (tested on the host); `plan_for` reads the set and `apply` moves the variants.

#![warn(dead_code)] // the module above allows dead code (the 0.10.0 drawing); this one is checked

use super::section::{FRAME_PAD, FRAME_W};
use super::*;

/// The padding of the component set around its variants, and the gap between variants (horizontal and vertical).
pub const SET_PAD: f64 = 24.0;
pub const VARIANT_GAP: f64 = 48.0;
/// The padding of the Master's stage around the set.
pub const STAGE_PAD: f64 = 48.0;
/// The room the set has inside the Master's stage in a 1180 frame: 1180 - 2 x 48 - 2 x 48.
pub const AVAILABLE_W: f64 = FRAME_W - 2.0 * FRAME_PAD - 2.0 * STAGE_PAD;
/// What a wide Master frame has more than its set: the stage's padding and the frame's, on both sides.
pub const WIDE_EXTRA: f64 = 2.0 * (FRAME_PAD + STAGE_PAD);

/// A variant of the set: its id, its place inside the set and its size.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The variants in the order a person reads them: top to bottom, then left to right (a pixel of difference in y is the same row).
pub fn reading_order(cells: &[Cell]) -> Vec<Cell> {
    let mut out = cells.to_vec();
    out.sort_by(|a, b| (a.y.round()).total_cmp(&b.y.round()).then(a.x.total_cmp(&b.x)));
    out
}

/// The variants laid out in `columns` columns, in reading order, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    /// (id, x, y) of each variant, inside the set.
    pub places: Vec<(u64, f64, f64)>,
    /// The size the set takes: padding + the columns + the gaps + padding.
    pub width: f64,
    pub height: f64,
}

/// The grid of `cells` (already in reading order) in `columns` columns: each column as wide as its widest variant, each row as tall as its
/// tallest, every variant at the top left of its cell, 48 between cells and 24 around the lot.
pub fn grid(cells: &[Cell], columns: usize) -> Grid {
    let columns = columns.clamp(1, cells.len().max(1));
    let rows = cells.len().div_ceil(columns).max(1);
    let mut col_w = vec![0.0f64; columns];
    let mut row_h = vec![0.0f64; rows];
    for (i, c) in cells.iter().enumerate() {
        col_w[i % columns] = col_w[i % columns].max(c.w);
        row_h[i / columns] = row_h[i / columns].max(c.h);
    }
    let x_of = |col: usize| SET_PAD + col_w[..col].iter().map(|w| w + VARIANT_GAP).sum::<f64>();
    let y_of = |row: usize| SET_PAD + row_h[..row].iter().map(|h| h + VARIANT_GAP).sum::<f64>();
    let places = cells.iter().enumerate().map(|(i, c)| (c.id, x_of(i % columns), y_of(i / columns))).collect();
    Grid { columns, rows, places, width: SET_PAD + col_w.iter().sum::<f64>() + VARIANT_GAP * (columns as f64 - 1.0) + SET_PAD, height: SET_PAD + row_h.iter().sum::<f64>() + VARIANT_GAP * (rows as f64 - 1.0) + SET_PAD }
}

/// The grid with the most columns that fits in `available` wide; none when not even one column does.
pub fn best_grid(cells: &[Cell], available: f64) -> Option<Grid> {
    let cells = reading_order(cells);
    (1..=cells.len().max(1)).rev().map(|k| grid(&cells, k)).find(|g| g.width <= available + 0.5)
}

/// What the Master does with a set that is `set_w` wide.
#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    /// It fits (or there is nothing to move): the variants stay where the person left them.
    Keep,
    /// It does not fit: the variants go into this grid, which does.
    Reflow(Grid),
    /// Not even a single column fits: the variants go into one column and the Master (and the whole Section) grows to `width` + 192.
    Wide { grid: Option<Grid>, width: f64 },
}

/// The plan for a set of `cells` that is `set_w` wide in `available` (988 in a 1180 frame).
pub fn plan(set_w: f64, cells: &[Cell], available: f64) -> Plan {
    if set_w <= available + 0.5 {
        return Plan::Keep;
    }
    if cells.is_empty() {
        // A single component, wider than the room: nothing to lay out, the Master grows.
        return Plan::Wide { grid: None, width: set_w };
    }
    match best_grid(cells, available) {
        Some(g) => Plan::Reflow(g),
        None => {
            let one = grid(&reading_order(cells), 1);
            let width = one.width;
            Plan::Wide { grid: Some(one), width }
        }
    }
}

impl Plan {
    /// The width of every frame of the Section: 1180, or the content + 192 when the Master had to grow.
    pub fn frame_width(&self) -> f64 {
        match self {
            Plan::Wide { width, .. } => FRAME_W.max(width + WIDE_EXTRA),
            _ => FRAME_W,
        }
    }

    pub fn wide(&self) -> bool {
        matches!(self, Plan::Wide { .. })
    }

    /// The grid the variants go into, when they move.
    pub fn grid(&self) -> Option<&Grid> {
        match self {
            Plan::Reflow(g) => Some(g),
            Plan::Wide { grid, .. } => grid.as_ref(),
            Plan::Keep => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Plan::Keep => "keep",
            Plan::Reflow(_) => "reflow",
            Plan::Wide { .. } => "wide",
        }
    }
}

/// The columns of a table scaled from the 1084 of a 1180 frame to `useful` wide, in whole pixels adding up to exactly `useful`.
pub fn scale_columns(cols: &[f64], useful: f64) -> Vec<f64> {
    let total: f64 = cols.iter().sum();
    if (total - useful).abs() < 0.5 || total <= 0.0 {
        return cols.to_vec();
    }
    let mut out: Vec<f64> = cols.iter().map(|c| (c * useful / total).floor()).collect();
    let left = (useful - out.iter().sum::<f64>()).round() as usize;
    for i in 0..left {
        let n = out.len();
        out[i % n] += 1.0;
    }
    out
}

/// Read the set (or the component) that goes into the Master and decide. A set is read in one call: its size and its variants' boxes.
pub fn plan_for(lang: Lang, holder: u64, is_set: bool) -> Result<Plan> {
    let node = host(lang, "node.get", &json!({"id": holder, "fields": ["absoluteBounds", "children"]}))?;
    let set = bounds(&node).unwrap_or_default();
    let cells: Vec<Cell> = if is_set {
        node.get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|c| Some(Cell { id: c.get("id").and_then(Value::as_u64)?, x: bounds(c)?.x - set.x, y: bounds(c)?.y - set.y, w: bounds(c)?.w, h: bounds(c)?.h }))
            .collect()
    } else {
        Vec::new()
    };
    // (`apply` reads the places again: a variant that is already where the grid puts it is not touched.)
    Ok(plan(set.w, &cells, AVAILABLE_W))
}

/// Put the variants of `holder` in the grid and size the set to it. A variant that already stands there is not touched, so doing this
/// again changes nothing. Returns how many variants moved.
pub fn apply(lang: Lang, holder: u64, grid: &Grid) -> Result<usize> {
    let node = host(lang, "node.get", &json!({"id": holder, "fields": ["absoluteBounds", "children"]}))?;
    let set = bounds(&node).unwrap_or_default();
    let mut moved = 0;
    for (id, x, y) in &grid.places {
        let here = node.get("children").and_then(Value::as_array).into_iter().flatten().find(|c| c.get("id").and_then(Value::as_u64) == Some(*id)).and_then(bounds);
        let same = here.is_some_and(|b| (b.x - set.x - x).abs() < 0.5 && (b.y - set.y - y).abs() < 0.5);
        if !same {
            host(lang, "node.set", &json!({"ids": [id], "props": {"x": x, "y": y}}))?;
            moved += 1;
        }
    }
    if (set.w - grid.width).abs() > 0.5 || (set.h - grid.height).abs() > 0.5 {
        host(lang, "node.set", &json!({"ids": [holder], "props": {"width": grid.width, "height": grid.height}}))?;
    }
    Ok(moved)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn cell(id: u64, x: f64, y: f64, w: f64, h: f64) -> Cell {
        Cell { id, x, y, w, h }
    }

    /// The Card of the demo: three variants side by side, 1528 wide (20 + 384 + 156 + 320 + 200 + 448).
    fn card() -> Vec<Cell> {
        vec![cell(667, 20.0, 20.0, 384.0, 422.0), cell(630, 540.0, 20.0, 320.0, 238.0), cell(704, 1060.0, 20.0, 448.0, 382.0)]
    }

    #[test]
    fn the_room_is_988_in_a_1180_frame() {
        assert_eq!(AVAILABLE_W, 988.0);
        assert_eq!(WIDE_EXTRA, 192.0);
    }

    #[test]
    fn variants_are_read_top_to_bottom_then_left_to_right() {
        let cells = vec![cell(3, 300.0, 100.0, 10.0, 10.0), cell(1, 0.0, 0.0, 10.0, 10.0), cell(4, 0.0, 100.0, 10.0, 10.0), cell(2, 300.0, 0.4, 10.0, 10.0)];
        assert_eq!(reading_order(&cells).iter().map(|c| c.id).collect::<Vec<_>>(), [1, 2, 4, 3], "0.4 px is the same row");
    }

    #[test]
    fn a_set_that_fits_is_not_touched() {
        assert_eq!(plan(988.0, &card(), AVAILABLE_W), Plan::Keep);
        assert_eq!(plan(400.0, &card(), AVAILABLE_W), Plan::Keep);
        assert_eq!(plan(1528.0, &[], AVAILABLE_W).frame_width(), 1528.0 + 192.0, "a single component that is too wide: no grid, the Master grows");
    }

    #[test]
    fn the_card_goes_into_two_columns_in_reading_order_with_gap_48_and_padding_24() {
        let Plan::Reflow(g) = plan(1528.0, &card(), AVAILABLE_W) else { panic!("the Card does not fit in 988") };
        // Three columns would be 24 + 384 + 320 + 448 + 96 + 24 = 1296 > 988; two: the first column holds variants 1 and 3 (384, 448), the second variant 2.
        assert_eq!((g.columns, g.rows), (2, 2));
        assert_eq!(g.width, 24.0 + 448.0 + 48.0 + 320.0 + 24.0);
        assert_eq!(g.height, 24.0 + 422.0 + 48.0 + 382.0 + 24.0);
        assert_eq!(g.places, vec![(667, 24.0, 24.0), (630, 24.0 + 448.0 + 48.0, 24.0), (704, 24.0, 24.0 + 422.0 + 48.0)]);
        assert!(g.width <= AVAILABLE_W);
        assert_eq!(Plan::Reflow(g).frame_width(), 1180.0, "the frame stays 1180");
    }

    #[test]
    fn a_wider_choice_of_columns_wins_when_it_fits_and_every_cell_is_at_its_top_left() {
        // Six 200 x 100 variants in a line (1400 + padding): 4 columns do not fit (24 + 800 + 144 + 24 = 992 > 988), 3 do.
        let cells: Vec<Cell> = (0..6).map(|i| cell(i + 1, 24.0 + i as f64 * 248.0, 24.0, 200.0, 100.0)).collect();
        let g = best_grid(&cells, AVAILABLE_W).unwrap();
        assert_eq!((g.columns, g.rows), (3, 2));
        assert_eq!(g.width, 24.0 + 600.0 + 96.0 + 24.0);
        assert_eq!(g.places[3], (4, 24.0, 24.0 + 100.0 + 48.0), "the fourth is the first of the second row");
        // 5 columns of 150 fit exactly: 24 + 750 + 192 + 24 = 990 > 988: no; with 4 of 150: 24 + 600 + 144 + 24 = 792.
        let small: Vec<Cell> = (0..5).map(|i| cell(i + 1, 24.0 + i as f64 * 198.0, 24.0, 150.0, 80.0)).collect();
        assert_eq!(best_grid(&small, AVAILABLE_W).unwrap().columns, 4);
    }

    #[test]
    fn the_rows_are_as_tall_as_their_tallest_and_the_columns_as_wide_as_their_widest() {
        let cells = vec![cell(1, 0.0, 0.0, 100.0, 50.0), cell(2, 200.0, 0.0, 300.0, 80.0), cell(3, 0.0, 200.0, 250.0, 30.0), cell(4, 200.0, 200.0, 60.0, 90.0)];
        let g = grid(&cells, 2);
        assert_eq!(g.places, vec![(1, 24.0, 24.0), (2, 24.0 + 250.0 + 48.0, 24.0), (3, 24.0, 24.0 + 80.0 + 48.0), (4, 24.0 + 250.0 + 48.0, 24.0 + 80.0 + 48.0)]);
        assert_eq!((g.width, g.height), (24.0 + 250.0 + 48.0 + 300.0 + 24.0, 24.0 + 80.0 + 48.0 + 90.0 + 24.0));
    }

    #[test]
    fn a_single_column_that_is_too_wide_makes_the_master_and_the_section_wider() {
        // One variant 1000 wide: 24 + 1000 + 24 = 1048 > 988 even alone.
        let cells = vec![cell(1, 0.0, 0.0, 1000.0, 200.0), cell(2, 1100.0, 0.0, 300.0, 100.0)];
        let p = plan(1424.0, &cells, AVAILABLE_W);
        assert!(p.wide());
        assert_eq!(p.grid().unwrap().columns, 1);
        assert_eq!(p.frame_width(), 1048.0 + 192.0, "the content + 192");
        // A variant of 940 plus the padding is exactly 988: it fits and the frame stays 1180.
        assert_eq!(plan(1300.0, &[cell(1, 0.0, 0.0, 940.0, 100.0), cell(2, 1000.0, 0.0, 200.0, 100.0)], AVAILABLE_W).frame_width(), 1180.0);
    }

    #[test]
    fn doing_it_twice_changes_nothing() {
        // The grid of the Card, as the set holds it afterwards: it fits, so the plan is to keep it, and the grid of those very positions is the same.
        let Plan::Reflow(g) = plan(1528.0, &card(), AVAILABLE_W) else { panic!() };
        let after: Vec<Cell> = g.places.iter().map(|(id, x, y)| { let c = card().into_iter().find(|c| c.id == *id).unwrap(); Cell { x: *x, y: *y, ..c } }).collect();
        assert_eq!(plan(g.width, &after, AVAILABLE_W), Plan::Keep);
        assert_eq!(best_grid(&after, AVAILABLE_W).unwrap().places, g.places, "the same positions come out of the new reading order");
    }

    #[test]
    fn table_columns_scale_to_a_wider_frame_in_whole_pixels_that_add_up() {
        let cols = [240.0, 140.0, 460.0, 244.0];
        assert_eq!(scale_columns(&cols, 1084.0), cols.to_vec());
        for useful in [1300.0, 1500.0, 1777.0] {
            let s = scale_columns(&cols, useful);
            assert_eq!(s.iter().sum::<f64>(), useful);
            assert!(s.iter().all(|c| c.fract() == 0.0));
        }
        assert_eq!(scale_columns(&[160.0, 180.0, 744.0], 1084.0 * 2.0).iter().sum::<f64>(), 2168.0);
    }
}
