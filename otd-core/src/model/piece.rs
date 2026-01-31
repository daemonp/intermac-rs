//! Piece definition representing a single glass workpiece in the layout.

use serde::{Deserialize, Serialize};

/// A single glass workpiece positioned on the cutting sheet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Piece {
    /// X origin position on sheet.
    pub x_origin: f64,
    /// Y origin position on sheet.
    pub y_origin: f64,
    /// Piece width.
    pub width: f64,
    /// Piece height.
    pub height: f64,
    /// Reference to Info section ID (if any).
    pub info_id: Option<i32>,
    /// Reference to Shape section ID (if any).
    pub shape_id: Option<i32>,
    /// Unique piece index in layout.
    pub piece_index: i32,
    /// Index into piece_types array (resolved).
    pub piece_type_index: Option<usize>,
    /// Index into shapes array (resolved).
    pub shape_index: Option<usize>,
    /// IndPiece value from OTD (for Cuttings section).
    pub ind_piece: Option<i32>,
    /// Which sides touch sheet edges (bitfield: 1=left, 2=bottom, 4=right, 8=top).
    pub edge_sides: u8,
}

impl Piece {
    /// Create a new piece.
    pub fn new(x_origin: f64, y_origin: f64, width: f64, height: f64) -> Self {
        Self {
            x_origin,
            y_origin,
            width,
            height,
            piece_index: -1,
            ..Default::default()
        }
    }

    /// Set origin coordinates.
    pub fn set_origin(&mut self, x: f64, y: f64) {
        self.x_origin = x;
        self.y_origin = y;
    }

    /// Set dimensions.
    pub fn set_dimensions(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    /// Set Info and Shape references.
    pub fn set_info_shape(&mut self, info_id: Option<i32>, shape_id: Option<i32>) {
        self.info_id = info_id;
        self.shape_id = shape_id;
    }

    /// Get the right edge X coordinate.
    pub fn x_max(&self) -> f64 {
        self.x_origin + self.width
    }

    /// Get the top edge Y coordinate.
    pub fn y_max(&self) -> f64 {
        self.y_origin + self.height
    }

    /// Check if piece touches left edge of sheet.
    pub fn touches_left(&self) -> bool {
        (self.edge_sides & 1) != 0
    }

    /// Check if piece touches bottom edge of sheet.
    pub fn touches_bottom(&self) -> bool {
        (self.edge_sides & 2) != 0
    }

    /// Check if piece touches right edge of sheet.
    pub fn touches_right(&self) -> bool {
        (self.edge_sides & 4) != 0
    }

    /// Check if piece touches top edge of sheet.
    pub fn touches_top(&self) -> bool {
        (self.edge_sides & 8) != 0
    }

    /// Set which edges this piece touches.
    pub fn set_edge_sides(&mut self, left: bool, bottom: bool, right: bool, top: bool) {
        self.edge_sides = 0;
        if left {
            self.edge_sides |= 1;
        }
        if bottom {
            self.edge_sides |= 2;
        }
        if right {
            self.edge_sides |= 4;
        }
        if top {
            self.edge_sides |= 8;
        }
    }

    /// Check if this piece has a custom shape.
    pub fn has_shape(&self) -> bool {
        self.shape_id.is_some()
    }

    /// Check if this piece has order info.
    pub fn has_info(&self) -> bool {
        self.info_id.is_some()
    }

    /// Get the center X coordinate.
    pub fn center_x(&self) -> f64 {
        self.x_origin + self.width / 2.0
    }

    /// Get the center Y coordinate.
    pub fn center_y(&self) -> f64 {
        self.y_origin + self.height / 2.0
    }
}
