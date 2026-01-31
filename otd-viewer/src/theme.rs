//! Color palette and styling constants for the viewer.
//!
//! Design philosophy:
//! - Industrial/professional look suitable for manufacturing environment
//! - High contrast for easy visibility
//! - Glass sheet is neutral (grey/slate) - it's the "paper"
//! - Pieces are the focus - use warm colors that stand out
//! - Waste areas are clearly marked as "bad" (hatched/red tint)
//! - Cuts are high-visibility (bright lines)

#![allow(dead_code)] // Many constants reserved for future features

use egui::Color32;

// =============================================================================
// BACKGROUND
// =============================================================================
pub const BACKGROUND: Color32 = Color32::from_rgb(25, 25, 28);
pub const CANVAS_BG: Color32 = Color32::from_rgb(32, 34, 37);

// =============================================================================
// GLASS SHEET - Neutral slate/grey, like actual glass on a dark table
// =============================================================================
pub const SHEET_FILL: Color32 = Color32::from_rgb(55, 65, 75);
pub const SHEET_BORDER: Color32 = Color32::from_rgb(100, 115, 130);
pub const TRIM_ZONE: Color32 = Color32::from_rgba_premultiplied(40, 45, 50, 200);

// =============================================================================
// CUTS - Bright red/orange for high visibility
// =============================================================================
pub const LINEAR_CUT: Color32 = Color32::from_rgb(255, 85, 50);
pub const LINEAR_CUT_HOVER: Color32 = Color32::from_rgb(255, 150, 100);

// =============================================================================
// PIECES - Warm amber/gold tones that pop against the cool grey sheet
// =============================================================================
pub const PIECE_BORDER: Color32 = Color32::from_rgb(255, 200, 100);
pub const PIECE_FILL: Color32 = Color32::from_rgba_premultiplied(255, 180, 80, 50);
pub const PIECE_HOVER_FILL: Color32 = Color32::from_rgba_premultiplied(255, 200, 100, 100);
pub const PIECE_HOVER_BORDER: Color32 = Color32::from_rgb(255, 230, 150);

// =============================================================================
// SHAPES - Teal/cyan for custom contours (distinct from pieces)
// =============================================================================
pub const SHAPE_STROKE: Color32 = Color32::from_rgb(0, 220, 180);
pub const SHAPE_FILL: Color32 = Color32::from_rgba_premultiplied(0, 200, 160, 60);
pub const SHAPE_HOVER_STROKE: Color32 = Color32::from_rgb(100, 255, 220);

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
