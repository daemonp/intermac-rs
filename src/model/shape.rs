//! Shape/Sagoma - Custom shape/contour definition.

use super::Cut;
use serde::{Deserialize, Serialize};

/// Custom shape/contour (Sagoma in Italian).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Shape {
    /// Shape identifier (matches Shape Id=).
    pub id: i32,
    /// Shape name.
    pub name: String,
    /// Shape description (e.g., "Form50", "Form51 (Rotated)").
    pub description: String,
    /// Base rotation angle (ROTS).
    pub rotation: f64,
    /// Left border offset.
    pub left_border: f64,
    /// Cut segments that make up the shape.
    pub cuts: Vec<Cut>,
    /// Tool types needed for this shape (indexed by tool code).
    pub tool_types: Vec<bool>,
    /// Whether this shape is open (doesn't close back to start).
    pub is_open: bool,
    /// Calculated perimeter length.
    pub perimeter: f64,
}

impl Shape {
    /// Create a new shape with the given ID.
    pub fn new(id: i32) -> Self {
        Self {
            id,
            tool_types: vec![false; 10], // Room for tool codes 1-9
            ..Default::default()
        }
    }

    /// Initialize cuts array with given capacity.
    pub fn init_cuts(&mut self, capacity: usize) {
        self.cuts = Vec::with_capacity(capacity);
    }

    /// Add a cut segment to the shape.
    pub fn add_cut(&mut self, cut: Cut) {
        self.cuts.push(cut);
    }

    /// Calculate the perimeter of the shape.
    pub fn calculate_perimeter(&mut self) {
        self.perimeter = self.cuts.iter().map(|c| c.calculate_length()).sum();
    }

    /// Check if the shape is closed (last point connects to first).
    pub fn is_closed(&self) -> bool {
        if self.cuts.is_empty() {
            return false;
        }

        use crate::config::float_cmp::approx_eq;

        let first = self.cuts.first().unwrap();
        let last = self.cuts.last().unwrap();

        approx_eq(first.xi, last.xf) && approx_eq(first.yi, last.yf)
    }

    /// Get the bounding box of the shape.
    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        if self.cuts.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for cut in &self.cuts {
            min_x = min_x.min(cut.xi).min(cut.xf);
            min_y = min_y.min(cut.yi).min(cut.yf);
            max_x = max_x.max(cut.xi).max(cut.xf);
            max_y = max_y.max(cut.yi).max(cut.yf);

            // For arcs, also consider the arc extent
            if cut.is_arc() {
                // Simple approximation: include center +/- radius
                min_x = min_x.min(cut.xc - cut.radius);
                min_y = min_y.min(cut.yc - cut.radius);
                max_x = max_x.max(cut.xc + cut.radius);
                max_y = max_y.max(cut.yc + cut.radius);
            }
        }

        (min_x, min_y, max_x, max_y)
    }

    /// Get the width of the shape.
    pub fn width(&self) -> f64 {
        let (min_x, _, max_x, _) = self.bounding_box();
        max_x - min_x
    }

    /// Get the height of the shape.
    pub fn height(&self) -> f64 {
        let (_, min_y, _, max_y) = self.bounding_box();
        max_y - min_y
    }

    /// Find which tool types are used in this shape.
    pub fn detect_tool_types(&mut self) {
        for cut in &self.cuts {
            let tool = cut.tool_code;
            if tool > 0 && (tool as usize) < self.tool_types.len() {
                self.tool_types[tool as usize] = true;
            }
        }
    }

    /// Check if this shape uses a specific tool.
    pub fn uses_tool(&self, tool_code: usize) -> bool {
        tool_code < self.tool_types.len() && self.tool_types[tool_code]
    }

    /// Get the starting point of the shape.
    pub fn start_point(&self) -> Option<(f64, f64)> {
        self.cuts.first().map(|c| (c.xi, c.yi))
    }

    /// Get the ending point of the shape.
    pub fn end_point(&self) -> Option<(f64, f64)> {
        self.cuts.last().map(|c| (c.xf, c.yf))
    }

    /// Calculate the initial rotation angle for the shape.
    pub fn calculate_initial_rotation(&self) -> f64 {
        if let Some(first_cut) = self.cuts.first() {
            first_cut.start_angle_degrees()
        } else {
            0.0
        }
    }
}
