//! Linear cut processing transformations.

use crate::config::{float_cmp, D_MIN_BORDO, EPS};
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

/// Remove cuts that are too close to sheet edges.
pub fn remove_edge_cuts(schema: &mut Schema) {
    let width = schema.width;
    let height = schema.height;

    for cut in &mut schema.linear_cuts {
        if !cut.active {
            continue;
        }

        let should_remove = match cut.line_type {
            LineType::Vertical => {
                // Remove if X is too close to left or right edge
                cut.xi < D_MIN_BORDO || cut.xi > width - D_MIN_BORDO
            }
            LineType::Horizontal => {
                // Remove if Y is too close to top or bottom edge
                cut.yi < D_MIN_BORDO || cut.yi > height - D_MIN_BORDO
            }
            LineType::Oblique => false,
        };

        if should_remove {
            cut.active = false;
        }
    }
}

/// Optimize cut order to minimize tool travel distance.
pub fn optimize_cut_order(schema: &mut Schema) {
    if schema.linear_cuts.len() < 2 {
        return;
    }

    let mut ordered: Vec<Cut> = Vec::with_capacity(schema.linear_cuts.len());
    let mut remaining: Vec<Cut> = schema
        .linear_cuts
        .iter()
        .filter(|c| c.active)
        .cloned()
        .collect();

    if remaining.is_empty() {
        return;
    }

    // Start from the cut closest to origin
    remaining.sort_by(|a, b| {
        let dist_a = a.xi * a.xi + a.yi * a.yi;
        let dist_b = b.xi * b.xi + b.yi * b.yi;
        dist_a.partial_cmp(&dist_b).unwrap()
    });

    let first = remaining.remove(0);
    let mut current_x = first.xf;
    let mut current_y = first.yf;
    ordered.push(first);

    // Greedy nearest-neighbor
    while !remaining.is_empty() {
        let mut best_idx = 0;
        let mut best_dist = f64::MAX;

        for (i, cut) in remaining.iter().enumerate() {
            // Distance to start of cut
            let dist_start = (cut.xi - current_x).powi(2) + (cut.yi - current_y).powi(2);
            // Distance to end of cut (could traverse in reverse)
            let dist_end = (cut.xf - current_x).powi(2) + (cut.yf - current_y).powi(2);

            let dist = dist_start.min(dist_end);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }

        let mut next = remaining.remove(best_idx);

        // Check if we should reverse the cut direction
        let dist_start = (next.xi - current_x).powi(2) + (next.yi - current_y).powi(2);
        let dist_end = (next.xf - current_x).powi(2) + (next.yf - current_y).powi(2);

        if dist_end < dist_start {
            // Reverse the cut
            std::mem::swap(&mut next.xi, &mut next.xf);
            std::mem::swap(&mut next.yi, &mut next.yf);
        }

        current_x = next.xf;
        current_y = next.yf;
        ordered.push(next);
    }

    // Replace with ordered cuts, keeping inactive ones at the end
    let inactive: Vec<Cut> = schema
        .linear_cuts
        .iter()
        .filter(|c| !c.active)
        .cloned()
        .collect();

    schema.linear_cuts = ordered;
    schema.linear_cuts.extend(inactive);
}

/// Apply linear advance offset to cuts.
pub fn apply_linear_advance(schema: &mut Schema, advance: f64) {
    if advance <= 0.0 {
        return;
    }

    for cut in &mut schema.linear_cuts {
        if !cut.active || !cut.is_line() {
            continue;
        }

        match cut.line_type {
            LineType::Vertical => {
                // Extend in Y direction
                if cut.yf > cut.yi {
                    cut.yi -= advance;
                    cut.yf += advance;
                } else {
                    cut.yi += advance;
                    cut.yf -= advance;
                }
            }
            LineType::Horizontal => {
                // Extend in X direction
                if cut.xf > cut.xi {
                    cut.xi -= advance;
                    cut.xf += advance;
                } else {
                    cut.xi += advance;
                    cut.xf -= advance;
                }
            }
            LineType::Oblique => {
                // Extend along the line direction
                let dx = cut.xf - cut.xi;
                let dy = cut.yf - cut.yi;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    let ux = dx / len;
                    let uy = dy / len;
                    cut.xi -= ux * advance;
                    cut.yi -= uy * advance;
                    cut.xf += ux * advance;
                    cut.yf += uy * advance;
                }
            }
        }

        // Clamp to sheet bounds
        cut.xi = cut.xi.max(0.0).min(schema.width);
        cut.yi = cut.yi.max(0.0).min(schema.height);
        cut.xf = cut.xf.max(0.0).min(schema.width);
        cut.yf = cut.yf.max(0.0).min(schema.height);
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
    if schema.linear_advance > 0.0 {
        apply_linear_advance(schema, schema.linear_advance);
    }

    schema.linear_cuts_optimized = true;
}
