//! Cut/Taglio - Individual cut segment definition.

use serde::{Deserialize, Serialize};

/// Type of cut geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CutType {
    /// Straight line segment.
    #[default]
    Line = 1,
    /// Clockwise arc.
    ArcCW = 2,
    /// Counter-clockwise arc.
    ArcCCW = 3,
}

/// Type of linear cut orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineType {
    /// Vertical cut (parallel to Y axis).
    #[default]
    Vertical = 1,
    /// Horizontal cut (parallel to X axis).
    Horizontal = 2,
    /// Oblique/diagonal cut.
    Oblique = 3,
}

/// Individual cut segment (Taglio in Italian).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cut {
    /// Type of cut (line, arc CW, arc CCW).
    pub cut_type: CutType,
    /// Line orientation for linear cuts.
    pub line_type: LineType,
    /// Start point X coordinate.
    pub xi: f64,
    /// Start point Y coordinate.
    pub yi: f64,
    /// End point X coordinate.
    pub xf: f64,
    /// End point Y coordinate.
    pub yf: f64,
    /// Arc center X coordinate (for arcs).
    pub xc: f64,
    /// Arc center Y coordinate (for arcs).
    pub yc: f64,
    /// Arc radius.
    pub radius: f64,
    /// Cut hierarchy level (Levcut).
    pub level: i32,
    /// Tool rotation angle.
    pub rotation: f64,
    /// Position quota in hierarchy (Qcut).
    pub quota: f64,
    /// Cut length (Lcut).
    pub length: f64,
    /// Cut type flags/bitmask (Tcut).
    pub tcut: i32,
    /// Rest/remainder dimension (Rcut).
    pub rest: f64,
    /// Tool code for this cut.
    pub tool_code: i32,
    /// Ablation width (for LowE coating removal).
    pub ablation_width: f64,
    /// Piece indices affected by this cut.
    pub piece_indices: Vec<i32>,
    /// Cut indices for pieces (which side of piece this cut affects).
    pub cut_indices: Vec<i32>,
    /// Number of pieces this cut affects.
    pub num_pieces: i32,
    /// Whether this cut is a waste/scrap cut.
    pub is_scrap: bool,
    /// Parent shape index (-1 if not part of a shape).
    pub parent_shape: i32,
    /// Whether this cut is active/enabled.
    pub active: bool,
}

impl Cut {
    /// Create a new line cut.
    pub fn new_line(xi: f64, yi: f64, xf: f64, yf: f64) -> Self {
        let mut cut = Self {
            cut_type: CutType::Line,
            xi,
            yi,
            xf,
            yf,
            active: true,
            parent_shape: -1,
            ..Default::default()
        };
        cut.determine_line_type();
        cut
    }

    /// Create a new clockwise arc.
    pub fn new_arc_cw(xi: f64, yi: f64, xf: f64, yf: f64, radius: f64) -> Self {
        let mut cut = Self {
            cut_type: CutType::ArcCW,
            xi,
            yi,
            xf,
            yf,
            radius,
            active: true,
            parent_shape: -1,
            ..Default::default()
        };
        cut.calculate_arc_center();
        cut
    }

    /// Create a new counter-clockwise arc.
    pub fn new_arc_ccw(xi: f64, yi: f64, xf: f64, yf: f64, radius: f64) -> Self {
        let mut cut = Self {
            cut_type: CutType::ArcCCW,
            xi,
            yi,
            xf,
            yf,
            radius,
            active: true,
            parent_shape: -1,
            ..Default::default()
        };
        cut.calculate_arc_center();
        cut
    }

    /// Determine the line type based on coordinates.
    pub fn determine_line_type(&mut self) {
        use crate::config::float_cmp::approx_eq;

        if approx_eq(self.xi, self.xf) {
            self.line_type = LineType::Vertical;
        } else if approx_eq(self.yi, self.yf) {
            self.line_type = LineType::Horizontal;
        } else {
            self.line_type = LineType::Oblique;
        }
    }

    /// Calculate arc center from endpoints and radius.
    pub fn calculate_arc_center(&mut self) {
        if self.radius <= 0.0 {
            return;
        }

        // Midpoint
        let mx = (self.xi + self.xf) / 2.0;
        let my = (self.yi + self.yf) / 2.0;

        // Distance between points
        let dx = self.xf - self.xi;
        let dy = self.yf - self.yi;
        let d = (dx * dx + dy * dy).sqrt();

        if d > 2.0 * self.radius {
            // Points too far apart for this radius
            self.xc = mx;
            self.yc = my;
            return;
        }

        // Height of center from chord
        let h = (self.radius * self.radius - (d / 2.0) * (d / 2.0)).sqrt();

        // Perpendicular unit vector
        let px = -dy / d;
        let py = dx / d;

        // For CW arc, center is on the right of the chord direction
        // For CCW arc, center is on the left
        let sign = match self.cut_type {
            CutType::ArcCW => 1.0,
            CutType::ArcCCW => -1.0,
            CutType::Line => 0.0,
        };

        self.xc = mx + sign * h * px;
        self.yc = my + sign * h * py;
    }

    /// Get the length of this cut.
    pub fn calculate_length(&self) -> f64 {
        match self.cut_type {
            CutType::Line => {
                let dx = self.xf - self.xi;
                let dy = self.yf - self.yi;
                (dx * dx + dy * dy).sqrt()
            }
            CutType::ArcCW | CutType::ArcCCW => {
                // Arc length = radius * angle
                let angle = self.arc_angle();
                self.radius * angle.abs()
            }
        }
    }

    /// Calculate the arc angle in radians.
    pub fn arc_angle(&self) -> f64 {
        if self.cut_type == CutType::Line {
            return 0.0;
        }

        let start_angle = (self.yi - self.yc).atan2(self.xi - self.xc);
        let end_angle = (self.yf - self.yc).atan2(self.xf - self.xc);

        let mut angle = end_angle - start_angle;

        // Normalize based on arc direction
        match self.cut_type {
            CutType::ArcCW => {
                if angle > 0.0 {
                    angle -= 2.0 * std::f64::consts::PI;
                }
            }
            CutType::ArcCCW => {
                if angle < 0.0 {
                    angle += 2.0 * std::f64::consts::PI;
                }
            }
            CutType::Line => {}
        }

        angle
    }

    /// Get the start angle for G-code output (in degrees).
    pub fn start_angle_degrees(&self) -> f64 {
        if self.cut_type == CutType::Line {
            // For lines, calculate angle from direction
            let dx = self.xf - self.xi;
            let dy = self.yf - self.yi;
            dy.atan2(dx).to_degrees()
        } else {
            let angle = (self.yi - self.yc).atan2(self.xi - self.xc);
            angle.to_degrees()
        }
    }

    /// Initialize piece indices array.
    pub fn init_piece_indices(&mut self, capacity: usize) {
        self.piece_indices = vec![-1; capacity];
        self.cut_indices = vec![-1; capacity];
        self.num_pieces = 0;
    }

    /// Add a piece index.
    pub fn add_piece_index(&mut self, piece_idx: i32, cut_idx: i32) {
        if (self.num_pieces as usize) < self.piece_indices.len() {
            self.piece_indices[self.num_pieces as usize] = piece_idx;
            self.cut_indices[self.num_pieces as usize] = cut_idx;
            self.num_pieces += 1;
        }
    }

    /// Check if this is a vertical cut.
    pub fn is_vertical(&self) -> bool {
        self.line_type == LineType::Vertical
    }

    /// Check if this is a horizontal cut.
    pub fn is_horizontal(&self) -> bool {
        self.line_type == LineType::Horizontal
    }

    /// Check if this is a straight line (not an arc).
    pub fn is_line(&self) -> bool {
        self.cut_type == CutType::Line
    }

    /// Check if this is an arc.
    pub fn is_arc(&self) -> bool {
        matches!(self.cut_type, CutType::ArcCW | CutType::ArcCCW)
    }
}
