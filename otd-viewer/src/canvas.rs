//! Canvas rendering for the OTD viewer.

#![allow(dead_code)] // Some functions reserved for future features

use egui::{Painter, Pos2, Rect, Stroke};
use otd_core::{Cut, CutType, Piece, Schema, Shape};

use crate::layers::LayerVisibility;
use crate::theme;
use crate::transform::ViewTransform;

/// Render the complete schema on the canvas.
pub fn render_schema(
    painter: &Painter,
    schema: &Schema,
    transform: &ViewTransform,
    canvas_rect: Rect,
    layers: &LayerVisibility,
    hovered_piece: Option<usize>,
    selected_piece: Option<usize>,
) {
    // Render layers from bottom to top
    if layers.sheet {
        render_sheet(painter, schema, transform, canvas_rect);
    }

    // Render grid (on top of sheet, below everything else)
    if layers.grid {
        render_grid(painter, schema, transform, canvas_rect);
    }

    if layers.trim && (schema.trim_left > 0.0 || schema.trim_bottom > 0.0) {
        render_trim_zone(painter, schema, transform, canvas_rect);
    }

    // Render waste regions (areas of the sheet not covered by pieces)
    if layers.waste {
        render_waste_regions(painter, schema, transform, canvas_rect);
    }

    if layers.linear_cuts {
        render_linear_cuts(painter, schema, transform, canvas_rect);
    }

    if layers.pieces {
        render_pieces(
            painter,
            schema,
            transform,
            canvas_rect,
            hovered_piece,
            selected_piece,
        );
    }

    if layers.shapes {
        render_shapes(painter, schema, transform, canvas_rect);
    }

    if layers.labels {
        render_labels(
            painter,
            schema,
            transform,
            canvas_rect,
            hovered_piece,
            selected_piece,
        );
    }
}

/// Render coordinate grid overlay.
fn render_grid(painter: &Painter, schema: &Schema, transform: &ViewTransform, canvas_rect: Rect) {
    // Determine grid spacing based on zoom level
    // At low zoom, use larger grid; at high zoom, use smaller grid
    let base_spacing = 10.0; // 10 inches base
    let zoom = transform.zoom as f64;

    // Adjust spacing so grid lines are roughly 50-100 pixels apart
    let target_screen_spacing = 80.0;
    let mut spacing = base_spacing;

    // Scale up if lines would be too close
    while spacing * zoom < target_screen_spacing / 2.0 && spacing < 100.0 {
        spacing *= 2.0;
    }
    // Scale down if lines would be too far apart
    while spacing * zoom > target_screen_spacing * 2.0 && spacing > 1.0 {
        spacing /= 2.0;
    }

    let minor_stroke = Stroke::new(1.0, theme::GRID_LINES);
    let major_stroke = Stroke::new(1.5, theme::GRID_MAJOR);

    // Draw vertical lines
    let mut x = 0.0;
    while x <= schema.width {
        let is_major = (x / spacing).round() % 5.0 == 0.0 || x == 0.0;
        let p1 = transform.sheet_to_screen(Pos2::new(x as f32, 0.0), canvas_rect);
        let p2 = transform.sheet_to_screen(Pos2::new(x as f32, schema.height as f32), canvas_rect);
        painter.line_segment([p1, p2], if is_major { major_stroke } else { minor_stroke });
        x += spacing;
    }

    // Draw horizontal lines
    let mut y = 0.0;
    while y <= schema.height {
        let is_major = (y / spacing).round() % 5.0 == 0.0 || y == 0.0;
        let p1 = transform.sheet_to_screen(Pos2::new(0.0, y as f32), canvas_rect);
        let p2 = transform.sheet_to_screen(Pos2::new(schema.width as f32, y as f32), canvas_rect);
        painter.line_segment([p1, p2], if is_major { major_stroke } else { minor_stroke });
        y += spacing;
    }
}

/// Render the glass sheet rectangle.
fn render_sheet(painter: &Painter, schema: &Schema, transform: &ViewTransform, canvas_rect: Rect) {
    let min = transform.sheet_to_screen(Pos2::new(0.0, 0.0), canvas_rect);
    let max = transform.sheet_to_screen(
        Pos2::new(schema.width as f32, schema.height as f32),
        canvas_rect,
    );

    let rect = Rect::from_two_pos(min, max);

    // Fill
    painter.rect_filled(rect, 0.0, theme::SHEET_FILL);

    // Border
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(theme::SHEET_STROKE_WIDTH, theme::SHEET_BORDER),
    );
}

/// Render the trim zone overlay.
fn render_trim_zone(
    painter: &Painter,
    schema: &Schema,
    transform: &ViewTransform,
    canvas_rect: Rect,
) {
    // Left trim
    if schema.trim_left > 0.0 {
        let min = transform.sheet_to_screen(Pos2::new(0.0, 0.0), canvas_rect);
        let max = transform.sheet_to_screen(
            Pos2::new(schema.trim_left as f32, schema.height as f32),
            canvas_rect,
        );
        let rect = Rect::from_two_pos(min, max);
        painter.rect_filled(rect, 0.0, theme::TRIM_ZONE);
    }

    // Bottom trim
    if schema.trim_bottom > 0.0 {
        let min = transform.sheet_to_screen(Pos2::new(0.0, 0.0), canvas_rect);
        let max = transform.sheet_to_screen(
            Pos2::new(schema.width as f32, schema.trim_bottom as f32),
            canvas_rect,
        );
        let rect = Rect::from_two_pos(min, max);
        painter.rect_filled(rect, 0.0, theme::TRIM_ZONE);
    }
}

/// Render waste regions (areas not covered by pieces).
/// This uses a simple approach: draw the whole sheet as waste, then "punch out" the pieces.
/// We approximate this by drawing waste-colored rectangles in gaps between pieces.
fn render_waste_regions(
    painter: &Painter,
    schema: &Schema,
    transform: &ViewTransform,
    canvas_rect: Rect,
) {
    // Skip if there are no pieces (entire sheet would be waste)
    if schema.pieces.is_empty() {
        return;
    }

    // Create a simple grid-based approach to find waste cells
    // We'll use the linear cuts to define the grid cells, then check which cells have no pieces

    // Collect all unique X and Y coordinates from linear cuts and sheet edges
    let mut x_coords: Vec<f64> = vec![schema.trim_left, schema.width];
    let mut y_coords: Vec<f64> = vec![schema.trim_bottom, schema.height];

    for cut in &schema.linear_cuts {
        if !cut.active {
            continue;
        }
        // Vertical cut
        if (cut.xi - cut.xf).abs() < 0.001 {
            x_coords.push(cut.xi);
        }
        // Horizontal cut
        if (cut.yi - cut.yf).abs() < 0.001 {
            y_coords.push(cut.yi);
        }
    }

    // Sort and deduplicate
    x_coords.sort_by(|a, b| a.partial_cmp(b).unwrap());
    y_coords.sort_by(|a, b| a.partial_cmp(b).unwrap());
    x_coords.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    y_coords.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    // For each grid cell, check if any piece covers it
    for x_idx in 0..x_coords.len().saturating_sub(1) {
        for y_idx in 0..y_coords.len().saturating_sub(1) {
            let cell_x1 = x_coords[x_idx];
            let cell_y1 = y_coords[y_idx];
            let cell_x2 = x_coords[x_idx + 1];
            let cell_y2 = y_coords[y_idx + 1];

            let cell_center_x = (cell_x1 + cell_x2) / 2.0;
            let cell_center_y = (cell_y1 + cell_y2) / 2.0;

            // Check if any piece contains this cell's center
            let is_covered = schema.pieces.iter().any(|piece| {
                cell_center_x >= piece.x_origin
                    && cell_center_x <= piece.x_origin + piece.width
                    && cell_center_y >= piece.y_origin
                    && cell_center_y <= piece.y_origin + piece.height
            });

            if !is_covered {
                // This cell is waste - draw it
                let min = transform
                    .sheet_to_screen(Pos2::new(cell_x1 as f32, cell_y1 as f32), canvas_rect);
                let max = transform
                    .sheet_to_screen(Pos2::new(cell_x2 as f32, cell_y2 as f32), canvas_rect);
                let rect = Rect::from_two_pos(min, max);

                // Fill with waste color
                painter.rect_filled(rect, 0.0, theme::WASTE_FILL);

                // Draw diagonal hatch lines for visual distinction
                draw_hatch_pattern(painter, rect, theme::WASTE_HATCH);
            }
        }
    }
}

/// Draw diagonal hatch lines in a rectangle.
fn draw_hatch_pattern(painter: &Painter, rect: Rect, color: egui::Color32) {
    let spacing = 8.0;
    let stroke = Stroke::new(1.0, color);

    let width = rect.width();
    let height = rect.height();
    let max_dim = width + height;

    // Draw lines from bottom-left to top-right
    let mut offset = 0.0;
    while offset < max_dim {
        // Line goes from (x1, y1) to (x2, y2)
        let x1 = rect.min.x + (offset - height).max(0.0);
        let y1 = rect.max.y - (offset).min(height);
        let x2 = rect.min.x + (offset).min(width);
        let y2 = rect.max.y - (offset - width).max(0.0);

        if x1 < rect.max.x && x2 > rect.min.x && y1 > rect.min.y && y2 < rect.max.y {
            painter.line_segment([Pos2::new(x1, y1), Pos2::new(x2, y2)], stroke);
        }

        offset += spacing;
    }
}

/// Render all linear cuts.
fn render_linear_cuts(
    painter: &Painter,
    schema: &Schema,
    transform: &ViewTransform,
    canvas_rect: Rect,
) {
    for cut in &schema.linear_cuts {
        if !cut.active {
            continue;
        }
        render_cut(painter, cut, transform, canvas_rect, theme::LINEAR_CUT);
    }
}

/// Render a single cut segment.
fn render_cut(
    painter: &Painter,
    cut: &Cut,
    transform: &ViewTransform,
    canvas_rect: Rect,
    color: egui::Color32,
) {
    let stroke = Stroke::new(theme::CUT_STROKE_WIDTH, color);

    match cut.cut_type {
        CutType::Line => {
            let p1 =
                transform.sheet_to_screen(Pos2::new(cut.xi as f32, cut.yi as f32), canvas_rect);
            let p2 =
                transform.sheet_to_screen(Pos2::new(cut.xf as f32, cut.yf as f32), canvas_rect);
            painter.line_segment([p1, p2], stroke);
        }
        CutType::ArcCW | CutType::ArcCCW => {
            render_arc(painter, cut, transform, canvas_rect, stroke);
        }
    }
}

/// Render an arc by tessellating it into line segments.
fn render_arc(
    painter: &Painter,
    cut: &Cut,
    transform: &ViewTransform,
    canvas_rect: Rect,
    stroke: Stroke,
) {
    const SEGMENTS: usize = 32;

    let start_angle = (cut.yi - cut.yc).atan2(cut.xi - cut.xc);
    let end_angle = (cut.yf - cut.yc).atan2(cut.xf - cut.xc);

    // Calculate sweep angle based on arc direction
    let mut sweep = end_angle - start_angle;
    match cut.cut_type {
        CutType::ArcCW => {
            if sweep > 0.0 {
                sweep -= std::f64::consts::TAU;
            }
        }
        CutType::ArcCCW => {
            if sweep < 0.0 {
                sweep += std::f64::consts::TAU;
            }
        }
        CutType::Line => return,
    }

    let mut points = Vec::with_capacity(SEGMENTS + 1);
    for i in 0..=SEGMENTS {
        let t = i as f64 / SEGMENTS as f64;
        let angle = start_angle + sweep * t;
        let x = cut.xc + cut.radius * angle.cos();
        let y = cut.yc + cut.radius * angle.sin();
        points.push(transform.sheet_to_screen(Pos2::new(x as f32, y as f32), canvas_rect));
    }

    // Draw as connected line segments
    for window in points.windows(2) {
        painter.line_segment([window[0], window[1]], stroke);
    }
}

/// Render all pieces.
fn render_pieces(
    painter: &Painter,
    schema: &Schema,
    transform: &ViewTransform,
    canvas_rect: Rect,
    hovered_piece: Option<usize>,
    selected_piece: Option<usize>,
) {
    for (i, piece) in schema.pieces.iter().enumerate() {
        let is_hovered = hovered_piece == Some(i);
        let is_selected = selected_piece == Some(i);
        render_piece(
            painter,
            piece,
            transform,
            canvas_rect,
            is_hovered,
            is_selected,
        );
    }
}

/// Render a single piece rectangle.
fn render_piece(
    painter: &Painter,
    piece: &Piece,
    transform: &ViewTransform,
    canvas_rect: Rect,
    is_hovered: bool,
    is_selected: bool,
) {
    let min = transform.sheet_to_screen(
        Pos2::new(piece.x_origin as f32, piece.y_origin as f32),
        canvas_rect,
    );
    let max = transform.sheet_to_screen(
        Pos2::new(
            (piece.x_origin + piece.width) as f32,
            (piece.y_origin + piece.height) as f32,
        ),
        canvas_rect,
    );

    let rect = Rect::from_two_pos(min, max);

    // Choose colors based on selection/hover state
    let (fill_color, border_color, stroke_width) = if is_selected {
        (
            theme::PIECE_SELECTED_FILL,
            theme::PIECE_SELECTED_BORDER,
            theme::SELECTION_STROKE_WIDTH,
        )
    } else if is_hovered {
        (
            theme::PIECE_HOVER_FILL,
            theme::PIECE_HOVER_BORDER,
            theme::PIECE_HOVER_STROKE_WIDTH,
        )
    } else {
        (
            theme::PIECE_FILL,
            theme::PIECE_BORDER,
            theme::PIECE_STROKE_WIDTH,
        )
    };

    // Fill
    painter.rect_filled(rect, 0.0, fill_color);

    // Border
    painter.rect_stroke(rect, 0.0, Stroke::new(stroke_width, border_color));
}

/// Render all shapes (for pieces that have custom contours).
fn render_shapes(painter: &Painter, schema: &Schema, transform: &ViewTransform, canvas_rect: Rect) {
    for piece in &schema.pieces {
        if let Some(shape_idx) = piece.shape_index {
            if let Some(shape) = schema.shapes.get(shape_idx) {
                render_shape(painter, shape, piece, transform, canvas_rect);
            }
        }
    }
}

/// Render a shape contour translated to the piece's position.
fn render_shape(
    painter: &Painter,
    shape: &Shape,
    piece: &Piece,
    transform: &ViewTransform,
    canvas_rect: Rect,
) {
    if shape.cuts.is_empty() {
        return;
    }

    let stroke = Stroke::new(theme::SHAPE_STROKE_WIDTH, theme::SHAPE_STROKE);

    for cut in &shape.cuts {
        // Translate cut coordinates by piece origin
        let translated = Cut {
            xi: cut.xi + piece.x_origin,
            yi: cut.yi + piece.y_origin,
            xf: cut.xf + piece.x_origin,
            yf: cut.yf + piece.y_origin,
            xc: cut.xc + piece.x_origin,
            yc: cut.yc + piece.y_origin,
            ..cut.clone()
        };

        match translated.cut_type {
            CutType::Line => {
                let p1 = transform.sheet_to_screen(
                    Pos2::new(translated.xi as f32, translated.yi as f32),
                    canvas_rect,
                );
                let p2 = transform.sheet_to_screen(
                    Pos2::new(translated.xf as f32, translated.yf as f32),
                    canvas_rect,
                );
                painter.line_segment([p1, p2], stroke);
            }
            CutType::ArcCW | CutType::ArcCCW => {
                render_arc(painter, &translated, transform, canvas_rect, stroke);
            }
        }
    }
}

/// Render piece labels.
fn render_labels(
    painter: &Painter,
    schema: &Schema,
    transform: &ViewTransform,
    canvas_rect: Rect,
    hovered_piece: Option<usize>,
    selected_piece: Option<usize>,
) {
    for (i, piece) in schema.pieces.iter().enumerate() {
        let center = transform.sheet_to_screen(
            Pos2::new(
                (piece.x_origin + piece.width / 2.0) as f32,
                (piece.y_origin + piece.height / 2.0) as f32,
            ),
            canvas_rect,
        );

        let is_hovered = hovered_piece == Some(i);
        let is_selected = selected_piece == Some(i);
        let is_emphasized = is_hovered || is_selected;

        // Build label text - show dimensions for selected piece
        let label = if is_selected {
            format!("#{}\n{:.2}\" × {:.2}\"", i + 1, piece.width, piece.height)
        } else {
            format!("#{}", i + 1)
        };

        let font_size = if is_emphasized { 14.0 } else { 12.0 };

        // Draw shadow for better readability
        painter.text(
            center + egui::Vec2::new(1.0, 1.0),
            egui::Align2::CENTER_CENTER,
            &label,
            egui::FontId::proportional(font_size),
            theme::LABEL_SHADOW,
        );

        // Draw label
        let text_color = if is_selected {
            theme::SELECTION
        } else if is_hovered {
            theme::PIECE_HOVER_BORDER
        } else {
            theme::LABEL_TEXT
        };

        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(font_size),
            text_color,
        );
    }
}

/// Calculate the bounding box of a schema in screen coordinates.
pub fn schema_bounds(schema: &Schema, transform: &ViewTransform, canvas_rect: Rect) -> Rect {
    let min = transform.sheet_to_screen(Pos2::ZERO, canvas_rect);
    let max = transform.sheet_to_screen(
        Pos2::new(schema.width as f32, schema.height as f32),
        canvas_rect,
    );
    Rect::from_two_pos(min, max)
}
