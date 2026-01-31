//! Linear cut processing transformations.

use crate::config::{float_cmp, EPS};
use crate::model::{Cut, LineType, Schema};

/// Merge overlapping linear cuts on the same line.
pub fn merge_linear_cuts(schema: &mut Schema) {
    if schema.linear_cuts.len() < 2 {
        return;
    }

    // Collect vertical cut indices
    let vertical_indices: Vec<usize> = get_cut_indices(&schema.linear_cuts, true);
    merge_overlapping_by_indices(&mut schema.linear_cuts, &vertical_indices, true);

    // Collect horizontal cut indices (re-collect as some may have been deactivated)
    let horizontal_indices: Vec<usize> = get_cut_indices(&schema.linear_cuts, false);
    merge_overlapping_by_indices(&mut schema.linear_cuts, &horizontal_indices, false);
}

/// Get indices of cuts by orientation.
fn get_cut_indices(cuts: &[Cut], is_vertical: bool) -> Vec<usize> {
    cuts.iter()
        .enumerate()
        .filter(|(_, c)| {
            c.active
                && if is_vertical {
                    c.is_vertical()
                } else {
                    c.is_horizontal()
                }
        })
        .map(|(i, _)| i)
        .collect()
}

/// Merge overlapping cuts by indices.
#[allow(clippy::needless_range_loop)] // We need indices to mutate cuts array
fn merge_overlapping_by_indices(cuts: &mut [Cut], indices: &[usize], is_vertical: bool) {
    if indices.len() < 2 {
        return;
    }

    // Sort indices by position
    let mut sorted_indices: Vec<usize> = indices.to_vec();
    sorted_indices.sort_by(|&a, &b| {
        if is_vertical {
            cuts[a].xi.partial_cmp(&cuts[b].xi).unwrap().then(
                cuts[a]
                    .yi
                    .min(cuts[a].yf)
                    .partial_cmp(&cuts[b].yi.min(cuts[b].yf))
                    .unwrap(),
            )
        } else {
            cuts[a].yi.partial_cmp(&cuts[b].yi).unwrap().then(
                cuts[a]
                    .xi
                    .min(cuts[a].xf)
                    .partial_cmp(&cuts[b].xi.min(cuts[b].xf))
                    .unwrap(),
            )
        }
    });

    for i in 0..sorted_indices.len() - 1 {
        let idx_i = sorted_indices[i];
        if !cuts[idx_i].active {
            continue;
        }

        for j in i + 1..sorted_indices.len() {
            let idx_j = sorted_indices[j];
            if !cuts[idx_j].active {
                continue;
            }

            let same_line = if is_vertical {
                float_cmp::approx_eq(cuts[idx_i].xi, cuts[idx_j].xi)
            } else {
                float_cmp::approx_eq(cuts[idx_i].yi, cuts[idx_j].yi)
            };

            if !same_line {
                break;
            }

            let (start1, end1, start2, end2) = if is_vertical {
                (
                    cuts[idx_i].yi.min(cuts[idx_i].yf),
                    cuts[idx_i].yi.max(cuts[idx_i].yf),
                    cuts[idx_j].yi.min(cuts[idx_j].yf),
                    cuts[idx_j].yi.max(cuts[idx_j].yf),
                )
            } else {
                (
                    cuts[idx_i].xi.min(cuts[idx_i].xf),
                    cuts[idx_i].xi.max(cuts[idx_i].xf),
                    cuts[idx_j].xi.min(cuts[idx_j].xf),
                    cuts[idx_j].xi.max(cuts[idx_j].xf),
                )
            };

            if start2 <= end1 + EPS {
                let new_end = end1.max(end2);
                if is_vertical {
                    cuts[idx_i].yf = new_end;
                    cuts[idx_i].yi = start1;
                } else {
                    cuts[idx_i].xf = new_end;
                    cuts[idx_i].xi = start1;
                }
                cuts[idx_j].active = false;
            }
        }
    }
}

/// Remove cuts that are at the sheet edges.
///
/// Edge filtering logic:
/// - Vertical cuts: remove if X < min_border OR X close to width (within min_border)
/// - Horizontal cuts: remove if Y < min_border OR Y close to height (within min_border)
///
/// min_border is 2.0mm converted to the file's units.
pub fn remove_edge_cuts(schema: &mut Schema) {
    let width = schema.width;
    let height = schema.height;

    // dMinBordo = 2.0mm, converted to current units
    let d_min_bordo = 2.0 / schema.unit.to_mm_factor();

    for cut in &mut schema.linear_cuts {
        if !cut.active {
            continue;
        }

        let should_remove = match cut.line_type {
            LineType::Vertical => {
                // Remove if X is too close to 0 or to width
                cut.xi < d_min_bordo || (width - cut.xi).abs() < d_min_bordo
            }
            LineType::Horizontal => {
                // Remove if Y is too close to 0 or to height
                cut.yi < d_min_bordo || (height - cut.yi).abs() < d_min_bordo
            }
            LineType::Oblique => false,
        };

        if should_remove {
            cut.active = false;
        }
    }
}

/// Optimize cut order to minimize tool travel distance.
///
/// Uses nearest-neighbor algorithm starting from origin (0, 0).
pub fn optimize_cut_order(schema: &mut Schema) {
    if schema.linear_cuts.len() < 2 {
        return;
    }

    let mut remaining: Vec<(usize, bool)> = schema
        .linear_cuts
        .iter()
        .enumerate()
        .filter(|(_, c)| c.active)
        .map(|(i, _)| (i, false)) // (original index, reversed flag)
        .collect();

    if remaining.is_empty() {
        return;
    }

    // Start from origin (0, 0)
    let mut current_x = 0.0;
    let mut current_y = 0.0;

    // Order array: maps original index -> new position
    let mut order: Vec<(usize, bool, i32)> = Vec::with_capacity(remaining.len());

    // Greedy nearest-neighbor from origin
    for new_pos in 0..remaining.len() {
        let mut best_idx = 0;
        let mut best_dist = f64::MAX;
        let mut best_reversed = false;

        for (i, &(orig_idx, _)) in remaining.iter().enumerate() {
            let cut = &schema.linear_cuts[orig_idx];

            // Distance to start of cut
            let dx_start = current_x - cut.xi;
            let dy_start = current_y - cut.yi;
            let dist_start = (dx_start * dx_start + dy_start * dy_start).sqrt();

            // Distance to end of cut
            let dx_end = current_x - cut.xf;
            let dy_end = current_y - cut.yf;
            let dist_end = (dx_end * dx_end + dy_end * dy_end).sqrt();

            if dist_start < best_dist {
                best_dist = dist_start;
                best_idx = i;
                best_reversed = false;
            }
            if dist_end < best_dist {
                best_dist = dist_end;
                best_idx = i;
                best_reversed = true;
            }
        }

        let (orig_idx, _) = remaining.remove(best_idx);

        // Update current position to end of this cut
        let cut = &schema.linear_cuts[orig_idx];
        if best_reversed {
            current_x = cut.xi;
            current_y = cut.yi;
        } else {
            current_x = cut.xf;
            current_y = cut.yf;
        }

        order.push((orig_idx, best_reversed, new_pos as i32));
    }

    // Sort cuts by the new order
    order.sort_by_key(|(_, _, new_pos)| *new_pos);

    // Apply the ordering and reversals
    let mut new_cuts: Vec<Cut> = Vec::with_capacity(order.len());
    for (orig_idx, reversed, _) in order {
        let mut cut = schema.linear_cuts[orig_idx].clone();
        if reversed {
            std::mem::swap(&mut cut.xi, &mut cut.xf);
            std::mem::swap(&mut cut.yi, &mut cut.yf);
        }
        new_cuts.push(cut);
    }

    // Keep inactive cuts at the end
    let inactive: Vec<Cut> = schema
        .linear_cuts
        .iter()
        .filter(|c| !c.active)
        .cloned()
        .collect();

    schema.linear_cuts = new_cuts;
    schema.linear_cuts.extend(inactive);
}

/// Apply linear advance offset to cuts.
///
/// The linear advance shrinks each cut by `advance` on both ends,
/// allowing the tool to start inside the material rather than at the edge.
pub fn apply_linear_advance(schema: &mut Schema, advance: f64) {
    if advance <= 0.0 {
        return;
    }

    for cut in &mut schema.linear_cuts {
        if !cut.active || !cut.is_line() {
            continue;
        }

        // Calculate current cut length
        let length = cut.calculate_length();

        // Cut would be too short after applying advance - deactivate it
        let min_length = advance * 2.0 + EPS;
        if length < min_length {
            cut.active = false;
            continue;
        }

        match cut.line_type {
            LineType::Vertical => {
                // Shrink in Y direction
                if cut.yf > cut.yi {
                    cut.yi += advance;
                    cut.yf -= advance;
                } else {
                    cut.yi -= advance;
                    cut.yf += advance;
                }
            }
            LineType::Horizontal => {
                // Shrink in X direction
                if cut.xf > cut.xi {
                    cut.xi += advance;
                    cut.xf -= advance;
                } else {
                    cut.xi -= advance;
                    cut.xf += advance;
                }
            }
            LineType::Oblique => {
                // Shrink along the line direction
                let dx = cut.xf - cut.xi;
                let dy = cut.yf - cut.yi;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    let ux = dx / len;
                    let uy = dy / len;
                    cut.xi += ux * advance;
                    cut.yi += uy * advance;
                    cut.xf -= ux * advance;
                    cut.yf -= uy * advance;
                }
            }
        }
    }
}

/// Process all linear cut transformations.
pub fn process_linear_cuts(schema: &mut Schema) {
    if schema.linear_cuts_optimized {
        // Cuts were already optimized in the OTD file
        return;
    }

    // Step 1: Merge overlapping cuts
    merge_linear_cuts(schema);

    // Step 2: Remove edge cuts
    remove_edge_cuts(schema);

    // Step 3: Optimize order
    optimize_cut_order(schema);

    // Step 4: Apply linear advance
    let advance = schema.linear_advance;
    if advance > 0.0 {
        apply_linear_advance(schema, advance);
    }

    schema.linear_cuts_optimized = true;
}
