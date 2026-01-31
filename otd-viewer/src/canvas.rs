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
) {
    // Render layers from bottom to top
    if layers.sheet {
        render_sheet(painter, schema, transform, canvas_rect);
    }

    if layers.trim && (schema.trim_left > 0.0 || schema.trim_bottom > 0.0) {
        render_trim_zone(painter, schema, transform, canvas_rect);
    }

    if layers.linear_cuts {
        render_linear_cuts(painter, schema, transform, canvas_rect);
    }

    if layers.pieces {
        render_pieces(painter, schema, transform, canvas_rect);
    }

    if layers.shapes {
        render_shapes(painter, schema, transform, canvas_rect);
    }

    if layers.labels {
        render_labels(painter, schema, transform, canvas_rect);
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
fn render_pieces(painter: &Painter, schema: &Schema, transform: &ViewTransform, canvas_rect: Rect) {
    for piece in &schema.pieces {
        render_piece(painter, piece, transform, canvas_rect);
    }
}

/// Render a single piece rectangle.
fn render_piece(painter: &Painter, piece: &Piece, transform: &ViewTransform, canvas_rect: Rect) {
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

    // Fill
    painter.rect_filled(rect, 0.0, theme::PIECE_FILL);

    // Border
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(theme::PIECE_STROKE_WIDTH, theme::PIECE_BORDER),
    );
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
fn render_labels(painter: &Painter, schema: &Schema, transform: &ViewTransform, canvas_rect: Rect) {
    for (i, piece) in schema.pieces.iter().enumerate() {
        let center = transform.sheet_to_screen(
            Pos2::new(
                (piece.x_origin + piece.width / 2.0) as f32,
                (piece.y_origin + piece.height / 2.0) as f32,
            ),
            canvas_rect,
        );

        let label = format!("#{}", i + 1);
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.0),
            theme::LABEL_TEXT,
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
