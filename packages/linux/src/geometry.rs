//! Grid geometry and chord classification.
//!
//! Direct port of the Python prototype's `core.py`, which in turn mirrors
//! the macOS Swift `Geometry.swift` / `GridNavigator.swift` behavior.

/// Keyboard rows, mirroring the macOS app's layout.
pub const ROWS: [&str; 4] = ["1234567890", "qwertyuiop", "asdfghjkl;", "zxcvbnm,./"];

pub const REFINEMENT_WIDTH_EXPANSION_RATIO: f64 = 0.5;
pub const FINAL_CLICK_BASE_WIDTH_EXPANSION_RATIO: f64 = 0.5;
pub const FINAL_CLICK_LAPTOP_HEIGHT_EXPANSION_RATIO: f64 = 0.08;
pub const FINAL_CLICK_DESKTOP_HEIGHT_EXPANSION_RATIO: f64 = 0.03;
pub const FINAL_CLICK_LAPTOP_TARGET_KEY_WIDTH: f64 = 22.0;
pub const FINAL_CLICK_DESKTOP_TARGET_KEY_WIDTH: f64 = 19.0;
pub const FINAL_CLICK_MAX_SCREEN_WIDTH_FRACTION: f64 = 0.4;
pub const COMPACT_SCREEN_WIDTH_THRESHOLD: f64 = 1600.0;

/// A rectangle: (x, y, width, height).
pub type Bounds = (i32, i32, i32, i32);

/// A key's position within the keyboard grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellTarget {
    pub key: char,
    pub row: usize,
    pub column: usize,
    pub row_count: usize,
    pub column_count: usize,
}

/// Resolve a pressed key (single char, e.g. `"a"`, `";"`) to its grid cell.
/// Mirrors the Python prototype: the lowercased key must appear in a row.
pub fn find_cell_for_key(key: &str) -> Option<CellTarget> {
    let key_str = key.trim().to_lowercase();
    if key_str.is_empty() {
        return None;
    }
    for (row_index, row) in ROWS.iter().enumerate() {
        if let Some(column) = row.find(&key_str) {
            let ch = key_str.chars().next()?;
            return Some(CellTarget {
                key: ch,
                row: row_index,
                column,
                row_count: ROWS.len(),
                column_count: row.chars().count(),
            });
        }
    }
    None
}

/// Pixel bounds of a cell within `bounds`.
pub fn cell_bounds(bounds: Bounds, target: &CellTarget) -> Bounds {
    let (x, y, width, height) = bounds;
    let col_w = width as f64 / target.column_count as f64;
    let row_h = height as f64 / target.row_count as f64;
    let left = x + (target.column as f64 * col_w) as i32;
    let top = y + (target.row as f64 * row_h) as i32;
    let right = x + ((target.column + 1) as f64 * col_w) as i32;
    let bottom = y + ((target.row + 1) as f64 * row_h) as i32;
    (left, top, (right - left).max(1), (bottom - top).max(1))
}

/// Center point of a cell.
pub fn cell_center(bounds: Bounds, target: &CellTarget) -> (i32, i32) {
    let (left, top, width, height) = cell_bounds(bounds, target);
    (left + width / 2, top + height / 2)
}

/// Bounding box of a set of cell rectangles.
pub fn combine_bounds(rects: &[Bounds]) -> Bounds {
    let left = rects.iter().map(|r| r.0).min().unwrap_or(0);
    let top = rects.iter().map(|r| r.1).min().unwrap_or(0);
    let right = rects.iter().map(|r| r.0 + r.2).max().unwrap_or(0);
    let bottom = rects.iter().map(|r| r.1 + r.3).max().unwrap_or(0);
    (left, top, (right - left).max(1), (bottom - top).max(1))
}

pub fn rect_center(bounds: Bounds) -> (i32, i32) {
    let (x, y, w, h) = bounds;
    (x + w / 2, y + h / 2)
}

/// Classify a multi-key chord: `pair` (horizontally, vertically, or
/// diagonally adjacent cells) or `quad` (2x2 block).
/// Returns `None` when the keys do not form a chord.
pub fn classify_chord(targets: &[CellTarget]) -> Option<&'static str> {
    if targets.len() == 2 {
        let rows: std::collections::HashSet<_> = targets.iter().map(|t| t.row).collect();
        let cols: std::collections::HashSet<_> = targets.iter().map(|t| t.column).collect();
        let same_row =
            rows.len() == 1 && cols.iter().max().unwrap() - cols.iter().min().unwrap() == 1;
        let same_col =
            cols.len() == 1 && rows.iter().max().unwrap() - rows.iter().min().unwrap() == 1;
        let diagonal = rows.len() == 2
            && cols.len() == 2
            && rows.iter().max().unwrap() - rows.iter().min().unwrap() == 1
            && cols.iter().max().unwrap() - cols.iter().min().unwrap() == 1;
        if same_row || same_col || diagonal {
            return Some("pair");
        }
    }
    if targets.len() == 4 {
        let mut rows: Vec<_> = targets.iter().map(|t| t.row).collect();
        let mut cols: Vec<_> = targets.iter().map(|t| t.column).collect();
        rows.sort_unstable();
        rows.dedup();
        cols.sort_unstable();
        cols.dedup();
        if rows.len() == 2 && cols.len() == 2 {
            let expected: std::collections::HashSet<_> = rows
                .iter()
                .flat_map(|r| cols.iter().map(move |c| (*r, *c)))
                .collect();
            let actual: std::collections::HashSet<_> =
                targets.iter().map(|t| (t.row, t.column)).collect();
            if actual == expected {
                return Some("quad");
            }
        }
    }
    None
}

/// Expand a selected region for the next refinement step, clipped to the
/// screen. `next_depth` is the current step (1-based) at selection time.
pub fn expanded_bounds(rect: Bounds, screen_bounds: Bounds, next_depth: u32) -> Bounds {
    let mut width_ratio = 0.0;
    let mut height_ratio = 0.0;
    let (_, _, screen_width, _) = screen_bounds;
    let (_, _, rect_width, _) = rect;

    if next_depth == 1 {
        width_ratio = REFINEMENT_WIDTH_EXPANSION_RATIO;
    } else if next_depth == 2 {
        let compact_factor =
            ((COMPACT_SCREEN_WIDTH_THRESHOLD - screen_width as f64) / 500.0).clamp(0.0, 1.0);
        let target_key_width = FINAL_CLICK_DESKTOP_TARGET_KEY_WIDTH
            + (FINAL_CLICK_LAPTOP_TARGET_KEY_WIDTH - FINAL_CLICK_DESKTOP_TARGET_KEY_WIDTH)
                * compact_factor;
        height_ratio = FINAL_CLICK_DESKTOP_HEIGHT_EXPANSION_RATIO
            + (FINAL_CLICK_LAPTOP_HEIGHT_EXPANSION_RATIO
                - FINAL_CLICK_DESKTOP_HEIGHT_EXPANSION_RATIO)
                * compact_factor;
        let base_width = rect_width as f64 * (1.0 + 2.0 * FINAL_CLICK_BASE_WIDTH_EXPANSION_RATIO);
        let target_width_from_keys = target_key_width * crate::geometry::max_columns() as f64;
        let max_allowed_width = screen_width as f64 * FINAL_CLICK_MAX_SCREEN_WIDTH_FRACTION;
        let desired_width = base_width
            .max(target_width_from_keys)
            .min(max_allowed_width);
        width_ratio = ((desired_width / rect_width.max(1) as f64 - 1.0) / 2.0).max(0.0);
    }

    if width_ratio <= 0.0 && height_ratio <= 0.0 {
        return rect;
    }
    inset_and_clip(rect, screen_bounds, width_ratio, height_ratio)
}

/// Maximum columns across all rows (widest keyboard row).
pub fn max_columns() -> usize {
    ROWS.iter().map(|r| r.chars().count()).max().unwrap_or(10)
}

fn inset_and_clip(
    rect: Bounds,
    screen_bounds: Bounds,
    width_ratio: f64,
    height_ratio: f64,
) -> Bounds {
    let (x, y, width, height) = rect;
    let (sx, sy, sw, sh) = screen_bounds;
    let expanded_x = x - (width as f64 * width_ratio) as i32;
    let expanded_y = y - (height as f64 * height_ratio) as i32;
    let expanded_w = width + (width as f64 * width_ratio * 2.0) as i32;
    let expanded_h = height + (height as f64 * height_ratio * 2.0) as i32;

    let left = expanded_x.max(sx);
    let top = expanded_y.max(sy);
    let right = (expanded_x + expanded_w).min(sx + sw);
    let bottom = (expanded_y + expanded_h).min(sy + sh);
    (left, top, (right - left).max(1), (bottom - top).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_lookup() {
        let a = find_cell_for_key("a").unwrap();
        assert_eq!((a.row, a.column), (2, 0));
        let semicolon = find_cell_for_key(";").unwrap();
        assert_eq!((semicolon.row, semicolon.column), (2, 9));
        assert!(find_cell_for_key("tab").is_none());
    }

    #[test]
    fn full_screen_cell_center() {
        let bounds = (0, 0, 2048, 1152);
        let a = find_cell_for_key("a").unwrap();
        // row 2 of 4, col 0 of 10
        let (cx, cy) = cell_center(bounds, &a);
        assert_eq!(cx, 2048 / 10 / 2);
        assert_eq!(cy, 2 * (1152 / 4) + (1152 / 4) / 2);
    }

    #[test]
    fn chord_classification() {
        let pair: Vec<CellTarget> = ["z", "x"]
            .iter()
            .filter_map(|k| find_cell_for_key(k))
            .collect();
        assert_eq!(classify_chord(&pair), Some("pair"));
        let vertical: Vec<CellTarget> = ["q", "a"]
            .iter()
            .filter_map(|k| find_cell_for_key(k))
            .collect();
        assert_eq!(classify_chord(&vertical), Some("pair"));
        let diagonal: Vec<CellTarget> = ["q", "s"]
            .iter()
            .filter_map(|k| find_cell_for_key(k))
            .collect();
        assert_eq!(classify_chord(&diagonal), Some("pair"));
        let quad: Vec<CellTarget> = ["a", "s", "z", "x"]
            .iter()
            .filter_map(|k| find_cell_for_key(k))
            .collect();
        assert_eq!(classify_chord(&quad), Some("quad"));
        let non: Vec<CellTarget> = ["a", "l"]
            .iter()
            .filter_map(|k| find_cell_for_key(k))
            .collect();
        assert_eq!(classify_chord(&non), None);
    }
}
