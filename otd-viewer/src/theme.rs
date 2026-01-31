//! Color palette and styling constants for the viewer.
//!
//! Design philosophy:
//! - Calm, professional look suitable for manufacturing environment
//! - Glass sheet has a subtle blue-green tint like actual glass
//! - Pieces are outlined clearly but not loudly
//! - Cuts are subtle (they're just guides)
//! - Selection/hover is where we add emphasis

#![allow(dead_code)] // Many constants reserved for future features

use egui::Color32;

// =============================================================================
// BACKGROUND
// =============================================================================
pub const BACKGROUND: Color32 = Color32::from_rgb(25, 25, 28);
pub const CANVAS_BG: Color32 = Color32::from_rgb(30, 32, 35);

// =============================================================================
// GLASS SHEET - Subtle blue-green tint like actual glass
// =============================================================================
pub const SHEET_FILL: Color32 = Color32::from_rgb(45, 60, 65);
pub const SHEET_BORDER: Color32 = Color32::from_rgb(70, 90, 95);
pub const TRIM_ZONE: Color32 = Color32::from_rgba_premultiplied(35, 40, 45, 180);

// =============================================================================
// CUTS - Subtle, muted lines (not the focus)
// =============================================================================
pub const LINEAR_CUT: Color32 = Color32::from_rgb(120, 90, 80);
pub const LINEAR_CUT_HOVER: Color32 = Color32::from_rgb(180, 140, 120);

// =============================================================================
// PIECES - Clean, understated borders with very subtle fill
// =============================================================================
pub const PIECE_BORDER: Color32 = Color32::from_rgb(180, 190, 200);
pub const PIECE_FILL: Color32 = Color32::from_rgba_premultiplied(100, 120, 130, 30);
pub const PIECE_HOVER_FILL: Color32 = Color32::from_rgba_premultiplied(150, 170, 180, 60);
pub const PIECE_HOVER_BORDER: Color32 = Color32::from_rgb(220, 230, 240);
pub const PIECE_SELECTED_FILL: Color32 = Color32::from_rgba_premultiplied(255, 200, 100, 80);
pub const PIECE_SELECTED_BORDER: Color32 = Color32::from_rgb(255, 210, 120);

// =============================================================================
// SHAPES - Slightly brighter to distinguish custom contours
// =============================================================================
pub const SHAPE_STROKE: Color32 = Color32::from_rgb(100, 180, 160);
pub const SHAPE_FILL: Color32 = Color32::from_rgba_premultiplied(80, 150, 130, 40);
pub const SHAPE_HOVER_STROKE: Color32 = Color32::from_rgb(140, 210, 190);

// =============================================================================
// WASTE REGIONS - Muted red tint to indicate "scrap"
// =============================================================================
pub const WASTE_FILL: Color32 = Color32::from_rgba_premultiplied(120, 50, 50, 80);
pub const WASTE_BORDER: Color32 = Color32::from_rgba_premultiplied(180, 80, 80, 150);
pub const WASTE_HATCH: Color32 = Color32::from_rgba_premultiplied(150, 60, 60, 100);

// =============================================================================
// INTERACTION - Selection and hover
// =============================================================================
pub const SELECTION: Color32 = Color32::from_rgb(255, 220, 50);
pub const SELECTION_GLOW: Color32 = Color32::from_rgba_premultiplied(255, 220, 50, 80);
pub const HOVER_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 40);

// =============================================================================
// TEXT
// =============================================================================
pub const LABEL_TEXT: Color32 = Color32::from_rgb(240, 240, 240);
pub const LABEL_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 180);
pub const DIM_TEXT: Color32 = Color32::from_rgb(140, 145, 150);

// =============================================================================
// GRID
// =============================================================================
pub const GRID_LINES: Color32 = Color32::from_rgba_premultiplied(80, 85, 90, 100);
pub const GRID_MAJOR: Color32 = Color32::from_rgba_premultiplied(100, 105, 110, 150);

// =============================================================================
// UI ELEMENTS
// =============================================================================
pub const NAV_BUTTON_BG: Color32 = Color32::from_rgb(60, 65, 70);
pub const NAV_BUTTON_HOVER: Color32 = Color32::from_rgb(80, 85, 95);
pub const NAV_BUTTON_TEXT: Color32 = Color32::from_rgb(220, 220, 220);

// =============================================================================
// STROKE WIDTHS
// =============================================================================
pub const SHEET_STROKE_WIDTH: f32 = 2.0;
pub const CUT_STROKE_WIDTH: f32 = 2.0;
pub const PIECE_STROKE_WIDTH: f32 = 1.5;
pub const PIECE_HOVER_STROKE_WIDTH: f32 = 2.5;
pub const SHAPE_STROKE_WIDTH: f32 = 2.0;
pub const SELECTION_STROKE_WIDTH: f32 = 3.0;
pub const WASTE_STROKE_WIDTH: f32 = 1.0;
