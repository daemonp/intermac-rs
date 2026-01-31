//! PieceType/TipoPezzo - Customer and order metadata.

use serde::{Deserialize, Serialize};

/// Piece type with customer/order information (TipoPezzo in Italian).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PieceType {
    /// Type identifier (matches Info Id=).
    pub id: i32,
    /// Order number.
    pub order_no: String,
    /// Position number within order.
    pub position_no: String,
    /// Customer name.
    pub customer: String,
    /// Commission/project reference.
    pub commission: String,
    /// Secondary glass reference.
    pub second_glass_ref: String,
    /// Rack/sort number.
    pub rack_no: String,
    /// Nominal piece width.
    pub sheet_width: f64,
    /// Nominal piece height.
    pub sheet_height: f64,
    /// Piece code identifier.
    pub piece_code: i32,
    /// Whether this is a waste/scrap piece.
    pub waste: bool,
}

impl PieceType {
    /// Create a new piece type with the given ID.
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    /// Check if this piece type has customer info.
    pub fn has_customer(&self) -> bool {
        !self.customer.is_empty()
    }

    /// Check if this piece type has an order number.
    pub fn has_order(&self) -> bool {
        !self.order_no.is_empty()
    }

    /// Get display dimensions (width x height).
    pub fn dimensions_string(&self) -> String {
        format!("{} x {}", self.sheet_width, self.sheet_height)
    }
}
