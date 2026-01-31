//! Cut segment definition for linear and shaped cuts.

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

/// Individual cut segment defining a line or arc path.
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
    /// Cut hierarchy level.
    pub level: i32,
    /// Tool rotation angle.
    pub rotation: f64,
    /// Position quota in hierarchy.
    pub quota: f64,
    /// Cut length.
    pub length: f64,
    /// Cut type flags/bitmask.
    pub tcut: i32,
    /// Rest/remainder dimension.
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
    ///
    /// Algorithm:
    /// 1. Calculate angle from start to end point
    /// 2. Calculate chord midpoint
    /// 3. Calculate distance from midpoint to center
    /// 4. Rotate angle by -90° (CW) or +90° (CCW) to find perpendicular direction
    /// 5. Center = midpoint + distance in that direction
    pub fn calculate_arc_center(&mut self) {
        use crate::config::EPS;
        use std::f64::consts::FRAC_PI_2;

        if self.radius <= 0.0 {
            return;
        }

        // Angle from start to end point
        let dx = self.xf - self.xi;
        let dy = self.yf - self.yi;
        let mut angle = dy.atan2(dx);

        // Half the chord length
        let half_chord = (dx * dx + dy * dy).sqrt() / 2.0;

        // Handle case where points are at distance 2*radius (semicircle)
        let half_chord = if (half_chord - self.radius).abs() < 2.0 * EPS {
            self.radius
        } else {
            half_chord
        };

        // Midpoint of chord
        let mx = self.xi + half_chord * angle.cos();
        let my = self.yi + half_chord * angle.sin();

        // Distance from chord midpoint to arc center
        let h_squared = self.radius * self.radius - half_chord * half_chord;
        let h = if h_squared > 0.0 {
            h_squared.sqrt()
        } else {
            0.0
        };

        // Rotate angle perpendicular to chord
        // CW (Type 2): subtract 90°, center is to the right of chord direction
        // CCW (Type 3): add 90°, center is to the left of chord direction
        angle = match self.cut_type {
            CutType::ArcCW => angle - FRAC_PI_2,
            CutType::ArcCCW => angle + FRAC_PI_2,
            CutType::Line => angle,
        };

        // Calculate center
        self.xc = mx + h * angle.cos();
        self.yc = my + h * angle.sin();
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

    /// Get the initial angle in degrees for shape macro output.
    /// The angle is the direction the cut starts moving.
    pub fn initial_angle_degrees(&self) -> f64 {
        match self.cut_type {
            CutType::Line => {
                // For lines: angle of the line direction
                let dx = self.xf - self.xi;
                let dy = self.yf - self.yi;
                let angle = dy.atan2(dx).to_degrees();
                // Normalize to 0-360 range
                if angle < 0.0 {
                    angle + 360.0
                } else {
                    angle
                }
            }
            CutType::ArcCW => {
                // For CW arcs: tangent at start point
                // Radius outward = angle_to_center + 180°
                // CW tangent = 90° clockwise from outward = outward - 90° = angle_to_center + 90°
                let angle_to_center = (self.yc - self.yi).atan2(self.xc - self.xi);
                let tangent = angle_to_center + std::f64::consts::FRAC_PI_2;
                let degrees = tangent.to_degrees();
                if degrees < 0.0 {
                    degrees + 360.0
                } else {
                    degrees
                }
            }
            CutType::ArcCCW => {
                // For CCW arcs: tangent at start point
                // Radius outward = angle_to_center + 180°
                // CCW tangent = 90° counter-clockwise from outward = outward + 90° = angle_to_center + 270° = angle_to_center - 90°
                let angle_to_center = (self.yc - self.yi).atan2(self.xc - self.xi);
                let tangent = angle_to_center - std::f64::consts::FRAC_PI_2;
                let degrees = tangent.to_degrees();
                if degrees < 0.0 {
                    degrees + 360.0
                } else {
                    degrees
                }
            }
        }
    }

    /// Get the final angle in degrees (angle at the end of the cut).
    /// This is the direction the cut is moving when it finishes.
    pub fn final_angle_degrees(&self) -> f64 {
        match self.cut_type {
            CutType::Line => {
                // For lines: same as initial angle (direction doesn't change)
                self.initial_angle_degrees()
            }
            CutType::ArcCW => {
                // For CW arcs: tangent at end point
                // Same logic as initial: tangent = angle_to_center + 90°
                let angle_to_center = (self.yc - self.yf).atan2(self.xc - self.xf);
                let tangent = angle_to_center + std::f64::consts::FRAC_PI_2;
                let degrees = tangent.to_degrees();
                if degrees < 0.0 {
                    degrees + 360.0
                } else {
                    degrees
                }
            }
            CutType::ArcCCW => {
                // For CCW arcs: tangent at end point
                // Same logic as initial: tangent = angle_to_center - 90°
                let angle_to_center = (self.yc - self.yf).atan2(self.xc - self.xf);
                let tangent = angle_to_center - std::f64::consts::FRAC_PI_2;
                let degrees = tangent.to_degrees();
                if degrees < 0.0 {
                    degrees + 360.0
                } else {
                    degrees
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    const EPS: f64 = 0.001;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    // ==================== Line creation tests ====================

    #[test]
    fn test_new_line_horizontal() {
        let cut = Cut::new_line(0.0, 50.0, 100.0, 50.0);
        assert_eq!(cut.cut_type, CutType::Line);
        assert_eq!(cut.line_type, LineType::Horizontal);
        assert!(cut.active);
    }

    #[test]
    fn test_new_line_vertical() {
        let cut = Cut::new_line(50.0, 0.0, 50.0, 100.0);
        assert_eq!(cut.cut_type, CutType::Line);
        assert_eq!(cut.line_type, LineType::Vertical);
    }

    #[test]
    fn test_new_line_oblique() {
        let cut = Cut::new_line(0.0, 0.0, 100.0, 100.0);
        assert_eq!(cut.cut_type, CutType::Line);
        assert_eq!(cut.line_type, LineType::Oblique);
    }

    // ==================== Arc creation tests ====================

    #[test]
    fn test_new_arc_cw() {
        let cut = Cut::new_arc_cw(0.0, 0.0, 100.0, 0.0, 50.0);
        assert_eq!(cut.cut_type, CutType::ArcCW);
        assert!(approx_eq(cut.radius, 50.0));
        // Center should be below the chord (at y = -some_value for CW)
        // For semicircle, center is at midpoint of chord at distance 0
        assert!(approx_eq(cut.xc, 50.0));
    }

    #[test]
    fn test_new_arc_ccw() {
        let cut = Cut::new_arc_ccw(0.0, 0.0, 100.0, 0.0, 50.0);
        assert_eq!(cut.cut_type, CutType::ArcCCW);
        assert!(approx_eq(cut.radius, 50.0));
        // Center should be above the chord (at y = +some_value for CCW)
        assert!(approx_eq(cut.xc, 50.0));
    }

    // ==================== calculate_arc_center tests ====================

    #[test]
    fn test_calculate_arc_center_semicircle_cw() {
        // Semicircle: endpoints are at distance 2*radius
        // From (0,0) to (100,0) with radius 50 - this is exactly a semicircle
        let cut = Cut::new_arc_cw(0.0, 0.0, 100.0, 0.0, 50.0);
        assert!(approx_eq(cut.xc, 50.0));
        // For CW semicircle going left to right, center is below
        assert!(approx_eq(cut.yc, 0.0));
    }

    #[test]
    fn test_calculate_arc_center_semicircle_ccw() {
        let cut = Cut::new_arc_ccw(0.0, 0.0, 100.0, 0.0, 50.0);
        assert!(approx_eq(cut.xc, 50.0));
        // For CCW semicircle going left to right, center is above
        assert!(approx_eq(cut.yc, 0.0));
    }

    #[test]
    fn test_calculate_arc_center_quarter_circle() {
        // Quarter circle with radius 100
        // From (100, 0) to (0, 100), center should be at origin
        let cut = Cut::new_arc_ccw(100.0, 0.0, 0.0, 100.0, 100.0);
        assert!(approx_eq(cut.xc, 0.0));
        assert!(approx_eq(cut.yc, 0.0));
    }

    // ==================== calculate_length tests ====================

    #[test]
    fn test_calculate_length_line() {
        let cut = Cut::new_line(0.0, 0.0, 3.0, 4.0);
        assert!(approx_eq(cut.calculate_length(), 5.0)); // 3-4-5 triangle
    }

    #[test]
    fn test_calculate_length_horizontal_line() {
        let cut = Cut::new_line(0.0, 0.0, 100.0, 0.0);
        assert!(approx_eq(cut.calculate_length(), 100.0));
    }

    #[test]
    fn test_calculate_length_semicircle() {
        // Semicircle with radius 50 has arc length = pi * 50
        let cut = Cut::new_arc_cw(0.0, 0.0, 100.0, 0.0, 50.0);
        let expected = PI * 50.0;
        // Allow slightly larger tolerance for arc calculations
        assert!((cut.calculate_length() - expected).abs() < 0.1);
    }

    // ==================== arc_angle tests ====================

    #[test]
    fn test_arc_angle_line() {
        let cut = Cut::new_line(0.0, 0.0, 100.0, 0.0);
        assert!(approx_eq(cut.arc_angle(), 0.0));
    }

    #[test]
    fn test_arc_angle_semicircle_cw() {
        let cut = Cut::new_arc_cw(0.0, 0.0, 100.0, 0.0, 50.0);
        // Semicircle should have angle of -PI (CW is negative)
        assert!((cut.arc_angle() + PI).abs() < 0.1);
    }

    #[test]
    fn test_arc_angle_semicircle_ccw() {
        let cut = Cut::new_arc_ccw(0.0, 0.0, 100.0, 0.0, 50.0);
        // Semicircle should have angle of +PI (CCW is positive)
        assert!((cut.arc_angle() - PI).abs() < 0.1);
    }

    // ==================== initial_angle_degrees tests ====================

    #[test]
    fn test_initial_angle_degrees_horizontal_line_right() {
        let cut = Cut::new_line(0.0, 0.0, 100.0, 0.0);
        assert!(approx_eq(cut.initial_angle_degrees(), 0.0));
    }

    #[test]
    fn test_initial_angle_degrees_horizontal_line_left() {
        let cut = Cut::new_line(100.0, 0.0, 0.0, 0.0);
        assert!(approx_eq(cut.initial_angle_degrees(), 180.0));
    }

    #[test]
    fn test_initial_angle_degrees_vertical_line_up() {
        let cut = Cut::new_line(0.0, 0.0, 0.0, 100.0);
        assert!(approx_eq(cut.initial_angle_degrees(), 90.0));
    }

    #[test]
    fn test_initial_angle_degrees_vertical_line_down() {
        let cut = Cut::new_line(0.0, 100.0, 0.0, 0.0);
        assert!(approx_eq(cut.initial_angle_degrees(), 270.0));
    }

    #[test]
    fn test_initial_angle_degrees_diagonal_45() {
        let cut = Cut::new_line(0.0, 0.0, 100.0, 100.0);
        assert!(approx_eq(cut.initial_angle_degrees(), 45.0));
    }

    // ==================== final_angle_degrees tests ====================

    #[test]
    fn test_final_angle_degrees_line_same_as_initial() {
        let cut = Cut::new_line(0.0, 0.0, 100.0, 50.0);
        assert!(approx_eq(
            cut.initial_angle_degrees(),
            cut.final_angle_degrees()
        ));
    }

    // ==================== is_* helper methods tests ====================

    #[test]
    fn test_is_vertical() {
        let cut = Cut::new_line(50.0, 0.0, 50.0, 100.0);
        assert!(cut.is_vertical());
        assert!(!cut.is_horizontal());
        assert!(cut.is_line());
        assert!(!cut.is_arc());
    }

    #[test]
    fn test_is_horizontal() {
        let cut = Cut::new_line(0.0, 50.0, 100.0, 50.0);
        assert!(!cut.is_vertical());
        assert!(cut.is_horizontal());
        assert!(cut.is_line());
        assert!(!cut.is_arc());
    }

    #[test]
    fn test_is_arc() {
        let cut_cw = Cut::new_arc_cw(0.0, 0.0, 100.0, 0.0, 50.0);
        let cut_ccw = Cut::new_arc_ccw(0.0, 0.0, 100.0, 0.0, 50.0);

        assert!(cut_cw.is_arc());
        assert!(!cut_cw.is_line());
        assert!(cut_ccw.is_arc());
        assert!(!cut_ccw.is_line());
    }

    // ==================== piece indices tests ====================

    #[test]
    fn test_init_piece_indices() {
        let mut cut = Cut::new_line(0.0, 0.0, 100.0, 0.0);
        cut.init_piece_indices(5);
        assert_eq!(cut.piece_indices.len(), 5);
        assert_eq!(cut.cut_indices.len(), 5);
        assert_eq!(cut.num_pieces, 0);
        assert!(cut.piece_indices.iter().all(|&x| x == -1));
    }

    #[test]
    fn test_add_piece_index() {
        let mut cut = Cut::new_line(0.0, 0.0, 100.0, 0.0);
        cut.init_piece_indices(3);
        cut.add_piece_index(0, 1);
        cut.add_piece_index(1, 2);

        assert_eq!(cut.num_pieces, 2);
        assert_eq!(cut.piece_indices[0], 0);
        assert_eq!(cut.piece_indices[1], 1);
        assert_eq!(cut.cut_indices[0], 1);
        assert_eq!(cut.cut_indices[1], 2);
    }

    #[test]
    fn test_add_piece_index_overflow() {
        let mut cut = Cut::new_line(0.0, 0.0, 100.0, 0.0);
        cut.init_piece_indices(2);
        cut.add_piece_index(0, 1);
        cut.add_piece_index(1, 2);
        cut.add_piece_index(2, 3); // Should be ignored

        assert_eq!(cut.num_pieces, 2);
    }
}
