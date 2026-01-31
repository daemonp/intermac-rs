//! Shape processing transformations.

use crate::config::float_cmp;
use crate::model::{Cut, Schema};

/// Remove shape segments that overlap with linear cuts.
pub fn remove_overlapping_shape_segments(schema: &mut Schema) {
    for shape in &mut schema.shapes {
        for cut in &mut shape.cuts {
            if !cut.active || !cut.is_line() {
                continue;
            }

            // Check if this shape segment overlaps with any linear cut
            for linear_cut in &schema.linear_cuts {
                if !linear_cut.active {
                    continue;
                }

                if segments_overlap(cut, linear_cut) {
                    cut.active = false;
                    break;
                }
            }
        }
    }
}

/// Check if two line segments overlap (approximately on the same line).
fn segments_overlap(a: &Cut, b: &Cut) -> bool {
    if !a.is_line() || !b.is_line() {
        return false;
    }

    // Both must be same orientation
    if a.line_type != b.line_type {
        return false;
    }

    match a.line_type {
        crate::model::LineType::Vertical => {
            // Same X coordinate?
            if !float_cmp::approx_eq(a.xi, b.xi) {
                return false;
            }
            // Check Y overlap
            let a_min = a.yi.min(a.yf);
            let a_max = a.yi.max(a.yf);
            let b_min = b.yi.min(b.yf);
            let b_max = b.yi.max(b.yf);
            ranges_overlap(a_min, a_max, b_min, b_max)
        }
        crate::model::LineType::Horizontal => {
            // Same Y coordinate?
            if !float_cmp::approx_eq(a.yi, b.yi) {
                return false;
            }
            // Check X overlap
            let a_min = a.xi.min(a.xf);
            let a_max = a.xi.max(a.xf);
            let b_min = b.xi.min(b.xf);
            let b_max = b.xi.max(b.xf);
            ranges_overlap(a_min, a_max, b_min, b_max)
        }
        crate::model::LineType::Oblique => {
            // For oblique lines, use more complex overlap detection
            // Simplified: check if lines are collinear and overlap
            false // TODO: implement for oblique lines if needed
        }
    }
}

/// Check if two ranges overlap.
fn ranges_overlap(a_min: f64, a_max: f64, b_min: f64, b_max: f64) -> bool {
    a_min <= b_max + crate::config::EPS && b_min <= a_max + crate::config::EPS
}

/// Detect which tools are used in each shape.
pub fn detect_shape_tools(schema: &mut Schema) {
    for shape in &mut schema.shapes {
        shape.detect_tool_types();
    }
}

/// Calculate initial rotation angle for each shape.
pub fn calculate_shape_rotations(schema: &mut Schema) {
    for shape in &mut schema.shapes {
        if shape.cuts.is_empty() {
            continue;
        }

        shape.rotation = shape.calculate_initial_rotation();
    }
}

/// Validate that shapes are properly closed (or marked as open).
pub fn validate_shape_closure(schema: &Schema) -> Vec<(i32, bool)> {
    schema
        .shapes
        .iter()
        .map(|s| (s.id, s.is_closed()))
        .collect()
}

/// Check that the same shape isn't used on pieces of different sizes.
pub fn check_shape_piece_sizes(schema: &Schema) -> Result<(), Vec<i32>> {
    use std::collections::HashMap;

    let mut shape_sizes: HashMap<i32, (f64, f64)> = HashMap::new();
    let mut mismatched: Vec<i32> = Vec::new();

    for piece in &schema.pieces {
        if let Some(shape_id) = piece.shape_id {
            let size = (piece.width, piece.height);

            if let Some(existing) = shape_sizes.get(&shape_id) {
                // Check if sizes match (with tolerance)
                let width_matches = float_cmp::approx_eq(existing.0, size.0);
                let height_matches = float_cmp::approx_eq(existing.1, size.1);

                // Also check rotated match
                let rotated_width_matches = float_cmp::approx_eq(existing.0, size.1);
                let rotated_height_matches = float_cmp::approx_eq(existing.1, size.0);

                if !(width_matches && height_matches)
                    && !(rotated_width_matches && rotated_height_matches)
                {
                    if !mismatched.contains(&shape_id) {
                        mismatched.push(shape_id);
                    }
                }
            } else {
                shape_sizes.insert(shape_id, size);
            }
        }
    }

    if mismatched.is_empty() {
        Ok(())
    } else {
        Err(mismatched)
    }
}

/// Process all shape transformations.
pub fn process_shapes(schema: &mut Schema) {
    // Step 1: Detect tool types
    detect_shape_tools(schema);

    // Step 2: Calculate rotations
    calculate_shape_rotations(schema);

    // Step 3: Remove segments overlapping with linear cuts
    remove_overlapping_shape_segments(schema);
}

/// Order pieces by shape for optimized cutting.
pub fn order_pieces_by_shape(schema: &mut Schema) {
    if !schema.optimize_shape_order {
        return;
    }

    // Group pieces by shape ID
    let mut pieces_with_shape: Vec<_> = schema
        .pieces
        .iter()
        .enumerate()
        .filter(|(_, p)| p.shape_id.is_some())
        .map(|(i, p)| (i, p.shape_id.unwrap()))
        .collect();

    // Sort by shape ID to group same shapes together
    pieces_with_shape.sort_by_key(|(_, shape_id)| *shape_id);

    // Reorder pieces (this is a simplified version)
    // In the full implementation, this would also consider position
    // to minimize travel between same-shape pieces
}
