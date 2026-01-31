//! Color palette and styling constants for the viewer.

#![allow(dead_code)] // Many constants reserved for future features

use egui::Color32;

// Background colors
pub const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 30);
pub const CANVAS_BG: Color32 = Color32::from_rgb(40, 40, 40);

// Sheet colors
pub const SHEET_FILL: Color32 = Color32::from_rgb(45, 74, 62);
pub const SHEET_BORDER: Color32 = Color32::from_rgb(74, 124, 106);
pub const TRIM_ZONE: Color32 = Color32::from_rgba_premultiplied(61, 61, 61, 128);

// Cut colors
pub const LINEAR_CUT: Color32 = Color32::from_rgb(231, 76, 60);
pub const LINEAR_CUT_HOVER: Color32 = Color32::from_rgb(255, 120, 100);

// Piece colors
pub const PIECE_BORDER: Color32 = Color32::from_rgb(52, 152, 219);
pub const PIECE_FILL: Color32 = Color32::from_rgba_premultiplied(41, 128, 185, 64);
pub const PIECE_HOVER: Color32 = Color32::from_rgba_premultiplied(52, 152, 219, 128);

// Shape colors
pub const SHAPE_STROKE: Color32 = Color32::from_rgb(26, 188, 156);
pub const SHAPE_FILL: Color32 = Color32::from_rgba_premultiplied(22, 160, 133, 96);

// Interaction colors
pub const SELECTION: Color32 = Color32::from_rgb(241, 196, 15);
pub const HOVER_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 60);

// Text colors
pub const LABEL_TEXT: Color32 = Color32::from_rgb(236, 240, 241);
pub const DIM_TEXT: Color32 = Color32::from_rgb(150, 150, 150);

// Grid
pub const GRID_LINES: Color32 = Color32::from_rgba_premultiplied(80, 80, 80, 128);

// Stroke widths
pub const SHEET_STROKE_WIDTH: f32 = 2.0;
pub const CUT_STROKE_WIDTH: f32 = 1.5;
pub const PIECE_STROKE_WIDTH: f32 = 1.0;
pub const SHAPE_STROKE_WIDTH: f32 = 2.0;
pub const SELECTION_STROKE_WIDTH: f32 = 3.0;
