//! Schema - Complete cutting layout for one glass sheet.

use super::{Cut, Piece, PieceType, Shape};
use crate::config::Unit;
use serde::{Deserialize, Serialize};

/// Complete cutting layout for one glass sheet (Schema in Italian).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    // === Header information ===
    /// OTD file version string.
    pub otd_version: String,
    /// Unit of measurement.
    pub unit: Unit,
    /// Creation date string.
    pub date: String,
    /// Creator software name.
    pub creator: String,

    // === Machine information ===
    /// Target machine name.
    pub machine_name: String,
    /// Machine type number (100-199 for cutting tables).
    pub machine_number: u16,

    // === Glass/material information ===
    /// Glass/material type identifier.
    pub glass_id: String,
    /// Glass/material description.
    pub glass_description: String,
    /// Glass thickness.
    pub thickness: f64,
    /// Whether glass has structured/textured surface.
    pub glass_structured: bool,
    /// Whether glass has Low-E coating.
    pub glass_coated: bool,

    // === Sheet dimensions ===
    /// Sheet width.
    pub width: f64,
    /// Sheet height.
    pub height: f64,
    /// Left edge trim amount.
    pub trim_left: f64,
    /// Bottom edge trim amount.
    pub trim_bottom: f64,

    // === Cutting parameters ===
    /// Number of sheets to cut.
    pub quantity: u32,
    /// Cutting order: 0=linear first, 1=shapes first.
    pub cutting_order: u8,
    /// Linear advance/offset for cuts.
    pub linear_advance: f64,
    /// Minimum cutting angle for glass.
    pub min_angle: f64,
    /// Minimum cutting angle for coating.
    pub coating_min_angle: f64,
    /// Whether linear cuts have been optimized.
    pub linear_cuts_optimized: bool,
    /// Whether to optimize shape order.
    pub optimize_shape_order: bool,
    /// Layout sync number.
    pub n_layout_sync: i32,

    // === Tool codes ===
    /// Linear tool code.
    pub linear_tool: i32,
    /// Shaped tool code.
    pub shaped_tool: i32,
    /// Open shaped tool code.
    pub open_shaped_tool: i32,
    /// Incision/scoring tool code.
    pub incision_tool: i32,

    // === Cuts and pieces ===
    /// Linear cut segments.
    pub linear_cuts: Vec<Cut>,
    /// Low-E ablation cuts.
    pub lowe_cuts: Vec<Cut>,
    /// Workpieces.
    pub pieces: Vec<Piece>,
    /// Low-E workpieces.
    pub lowe_pieces: Vec<Piece>,
    /// Piece type definitions.
    pub piece_types: Vec<PieceType>,
    /// Shape definitions.
    pub shapes: Vec<Shape>,

    // === Flags ===
    /// Whether IndPiece was present in OTD.
    pub has_ind_piece: bool,
    /// Whether this schema uses multiple shapes (laminated mode).
    pub multiple_shapes: bool,
}

impl Schema {
    /// Create a new empty schema.
    pub fn new() -> Self {
        Self {
            quantity: 1,
            min_angle: 5.0,
            coating_min_angle: 5.0,
            optimize_shape_order: true,
            ..Default::default()
        }
    }

    /// Initialize linear cuts array.
    pub fn init_linear_cuts(&mut self, capacity: usize) {
        self.linear_cuts = Vec::with_capacity(capacity);
    }

    /// Initialize pieces array.
    pub fn init_pieces(&mut self, capacity: usize) {
        self.pieces = Vec::with_capacity(capacity);
    }

    /// Initialize piece types array.
    pub fn init_piece_types(&mut self, capacity: usize) {
        self.piece_types = Vec::with_capacity(capacity);
    }

    /// Initialize shapes array.
    pub fn init_shapes(&mut self, capacity: usize) {
        self.shapes = Vec::with_capacity(capacity);
    }

    /// Add a linear cut.
    pub fn add_linear_cut(&mut self, cut: Cut) {
        self.linear_cuts.push(cut);
    }

    /// Add a piece.
    pub fn add_piece(&mut self, piece: Piece) {
        self.pieces.push(piece);
    }

    /// Add a piece type.
    pub fn add_piece_type(&mut self, piece_type: PieceType) {
        self.piece_types.push(piece_type);
    }

    /// Add a shape.
    pub fn add_shape(&mut self, shape: Shape) {
        self.shapes.push(shape);
    }

    /// Find piece type by ID.
    pub fn find_piece_type(&self, id: i32) -> Option<usize> {
        self.piece_types.iter().position(|pt| pt.id == id)
    }

    /// Find shape by ID.
    pub fn find_shape(&self, id: i32) -> Option<usize> {
        self.shapes.iter().position(|s| s.id == id)
    }

    /// Get total number of linear cuts.
    pub fn num_linear_cuts(&self) -> usize {
        self.linear_cuts.len()
    }

    /// Get total number of pieces.
    pub fn num_pieces(&self) -> usize {
        self.pieces.len()
    }

    /// Get total number of shapes.
    pub fn num_shapes(&self) -> usize {
        self.shapes.len()
    }

    /// Calculate usable sheet dimensions (after trim).
    pub fn usable_width(&self) -> f64 {
        self.width - self.trim_left
    }

    /// Calculate usable sheet height (after trim).
    pub fn usable_height(&self) -> f64 {
        self.height - self.trim_bottom
    }

    /// Resolve piece indices to piece types and shapes.
    pub fn resolve_piece_references(&mut self) {
        for piece in &mut self.pieces {
            if let Some(info_id) = piece.info_id {
                piece.piece_type_index = self.piece_types.iter().position(|pt| pt.id == info_id);
            }
            if let Some(shape_id) = piece.shape_id {
                piece.shape_index = self.shapes.iter().position(|s| s.id == shape_id);
            }
        }
    }

    /// Set edge sides for all pieces based on sheet dimensions.
    pub fn calculate_piece_edges(&mut self) {
        use crate::config::float_cmp::approx_eq;

        for piece in &mut self.pieces {
            let left = approx_eq(piece.x_origin, self.trim_left) || approx_eq(piece.x_origin, 0.0);
            let bottom =
                approx_eq(piece.y_origin, self.trim_bottom) || approx_eq(piece.y_origin, 0.0);
            let right = approx_eq(piece.x_max(), self.width);
            let top = approx_eq(piece.y_max(), self.height);

            piece.set_edge_sides(left, bottom, right, top);
        }
    }

    /// Get distribution of pieces by piece type.
    pub fn piece_distribution(&self) -> Vec<(i32, usize)> {
        let mut counts: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();

        for piece in &self.pieces {
            if let Some(pt_idx) = piece.piece_type_index {
                let pt_id = self.piece_types[pt_idx].id;
                *counts.entry(pt_id).or_insert(0) += 1;
            }
        }

        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by_key(|(id, _)| *id);
        result
    }
}
