//! Validation logic for OTD to CNI conversion.

use crate::error::{ConvertError, Result};
use crate::model::Schema;

/// Validation result with warnings.
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub passed: bool,
    /// Warning messages.
    pub warnings: Vec<String>,
    /// Error messages.
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a passing result.
    pub fn ok() -> Self {
        Self {
            passed: true,
            ..Default::default()
        }
    }

    /// Create a failing result with an error.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            passed: false,
            errors: vec![message.into()],
            ..Default::default()
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    /// Add an error.
    pub fn add_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
        self.passed = false;
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: ValidationResult) {
        self.warnings.extend(other.warnings);
        self.errors.extend(other.errors);
        if !other.passed {
            self.passed = false;
        }
    }
}

/// Validate all schemas.
pub fn validate_schemas(schemas: &[Schema]) -> Result<ValidationResult> {
    let mut result = ValidationResult::ok();

    if schemas.is_empty() {
        return Err(ConvertError::NoPatternSection);
    }

    for (idx, schema) in schemas.iter().enumerate() {
        let schema_result = validate_schema(schema, idx + 1);
        result.merge(schema_result);
    }

    Ok(result)
}

/// Validate a single schema.
pub fn validate_schema(schema: &Schema, schema_num: usize) -> ValidationResult {
    let mut result = ValidationResult::ok();

    // Check sheet dimensions
    if schema.width <= 0.0 || schema.height <= 0.0 {
        result.add_error(format!(
            "Schema {}: Invalid sheet dimensions ({}x{})",
            schema_num, schema.width, schema.height
        ));
    }

    // Check thickness
    if schema.thickness <= 0.0 {
        result.add_warning(format!("Schema {}: Missing or zero thickness", schema_num));
    }

    // Validate pieces
    for (piece_idx, piece) in schema.pieces.iter().enumerate() {
        // Check piece dimensions
        if piece.width <= 0.0 || piece.height <= 0.0 {
            result.add_error(format!(
                "Schema {}, Piece {}: Invalid dimensions ({}x{})",
                schema_num,
                piece_idx + 1,
                piece.width,
                piece.height
            ));
        }

        // Check piece is within sheet bounds
        if piece.x_origin < 0.0
            || piece.y_origin < 0.0
            || piece.x_max() > schema.width + crate::config::EPS
            || piece.y_max() > schema.height + crate::config::EPS
        {
            result.add_warning(format!(
                "Schema {}, Piece {}: Extends beyond sheet bounds",
                schema_num,
                piece_idx + 1
            ));
        }

        // Check shape reference
        if let Some(shape_id) = piece.shape_id {
            if schema.find_shape(shape_id).is_none() {
                result.add_error(format!(
                    "Schema {}, Piece {}: Shape {} not found",
                    schema_num,
                    piece_idx + 1,
                    shape_id
                ));
            }
        }

        // Check info reference
        if let Some(info_id) = piece.info_id {
            if schema.find_piece_type(info_id).is_none() {
                result.add_error(format!(
                    "Schema {}, Piece {}: Info {} not found",
                    schema_num,
                    piece_idx + 1,
                    info_id
                ));
            }
        }
    }

    // Validate shapes
    for shape in &schema.shapes {
        // Check for empty shapes
        if shape.cuts.is_empty() {
            result.add_warning(format!(
                "Schema {}, Shape {}: No cuts defined",
                schema_num, shape.id
            ));
        }

        // Check shape closure
        if !shape.is_open && !shape.is_closed() {
            result.add_warning(format!(
                "Schema {}, Shape {}: Shape is not closed",
                schema_num, shape.id
            ));
        }

        // Validate arc radii
        for (cut_idx, cut) in shape.cuts.iter().enumerate() {
            if cut.is_arc() {
                let chord_len = ((cut.xf - cut.xi).powi(2) + (cut.yf - cut.yi).powi(2)).sqrt();
                if cut.radius < chord_len / 2.0 - crate::config::EPS {
                    result.add_error(format!(
                        "Schema {}, Shape {}, Cut {}: Arc radius {} is too small for chord length {}",
                        schema_num, shape.id, cut_idx + 1, cut.radius, chord_len
                    ));
                }
            }
        }
    }

    // Check for shape/piece size mismatch
    if let Err(mismatched) = crate::transform::check_shape_piece_sizes(schema) {
        for shape_id in mismatched {
            result.add_error(format!(
                "Schema {}: Shape {} is used on pieces of different sizes",
                schema_num, shape_id
            ));
        }
    }

    // Check for cuts
    if schema.linear_cuts.is_empty() && schema.shapes.is_empty() {
        result.add_warning(format!("Schema {}: No cuts or shapes defined", schema_num));
    }

    result
}

/// Validate that there are active linear cuts.
pub fn validate_has_cuts(schema: &Schema) -> bool {
    schema.linear_cuts.iter().any(|c| c.active) || schema.shapes.iter().any(|s| !s.cuts.is_empty())
}

/// Validate piece layout (no overlaps).
pub fn validate_piece_layout(schema: &Schema) -> Vec<(usize, usize)> {
    let mut overlaps = Vec::new();

    for i in 0..schema.pieces.len() {
        for j in i + 1..schema.pieces.len() {
            let a = &schema.pieces[i];
            let b = &schema.pieces[j];

            // Check for overlap
            let x_overlap = a.x_origin < b.x_max() && b.x_origin < a.x_max();
            let y_overlap = a.y_origin < b.y_max() && b.y_origin < a.y_max();

            if x_overlap && y_overlap {
                overlaps.push((i, j));
            }
        }
    }

    overlaps
}

/// Quick validation check for command-line --validate flag.
pub fn quick_validate(schemas: &[Schema]) -> Result<()> {
    let result = validate_schemas(schemas)?;

    if !result.passed {
        let error_msg = result.errors.join("; ");
        return Err(ConvertError::ParseError {
            line: 0,
            message: error_msg,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Cut, CutType, Piece, Shape};

    fn create_basic_schema() -> Schema {
        Schema {
            width: 1000.0,
            height: 500.0,
            thickness: 4.0,
            ..Default::default()
        }
    }

    // ==================== ValidationResult tests ====================

    #[test]
    fn test_validation_result_ok() {
        let result = ValidationResult::ok();
        assert!(result.passed);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validation_result_error() {
        let result = ValidationResult::error("Something went wrong");
        assert!(!result.passed);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Something went wrong");
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::ok();
        result.add_warning("This is a warning");
        assert!(result.passed); // Warnings don't fail validation
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::ok();
        result.add_error("This is an error");
        assert!(!result.passed);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::ok();
        result1.add_warning("Warning 1");

        let mut result2 = ValidationResult::ok();
        result2.add_error("Error 1");
        result2.add_warning("Warning 2");

        result1.merge(result2);
        assert!(!result1.passed);
        assert_eq!(result1.warnings.len(), 2);
        assert_eq!(result1.errors.len(), 1);
    }

    // ==================== validate_schemas tests ====================

    #[test]
    fn test_validate_schemas_empty() {
        let schemas: Vec<Schema> = vec![];
        let result = validate_schemas(&schemas);
        assert!(result.is_err());
        match result.unwrap_err() {
            ConvertError::NoPatternSection => {}
            _ => panic!("Expected NoPatternSection error"),
        }
    }

    #[test]
    fn test_validate_schemas_valid() {
        let schema = create_basic_schema();
        let result = validate_schemas(&[schema]).expect("Should succeed");
        assert!(result.passed);
    }

    // ==================== validate_schema tests ====================

    #[test]
    fn test_validate_schema_invalid_dimensions() {
        let mut schema = create_basic_schema();
        schema.width = 0.0;
        let result = validate_schema(&schema, 1);
        assert!(!result.passed);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Invalid sheet dimensions")));
    }

    #[test]
    fn test_validate_schema_zero_thickness_warning() {
        let mut schema = create_basic_schema();
        schema.thickness = 0.0;
        let result = validate_schema(&schema, 1);
        assert!(result.passed); // Zero thickness is only a warning
        assert!(result.warnings.iter().any(|w| w.contains("thickness")));
    }

    #[test]
    fn test_validate_schema_piece_invalid_dimensions() {
        let mut schema = create_basic_schema();
        schema.pieces.push(Piece {
            width: -100.0,
            height: 50.0,
            ..Default::default()
        });
        let result = validate_schema(&schema, 1);
        assert!(!result.passed);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Invalid dimensions")));
    }

    #[test]
    fn test_validate_schema_piece_out_of_bounds() {
        let mut schema = create_basic_schema();
        schema.pieces.push(Piece {
            x_origin: 900.0,
            y_origin: 0.0,
            width: 200.0, // Extends to 1100, beyond 1000
            height: 100.0,
            ..Default::default()
        });
        let result = validate_schema(&schema, 1);
        assert!(result.passed); // Out of bounds is just a warning
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("beyond sheet bounds")));
    }

    #[test]
    fn test_validate_schema_shape_not_found() {
        let mut schema = create_basic_schema();
        schema.pieces.push(Piece {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 100.0,
            height: 100.0,
            shape_id: Some(999), // Non-existent shape
            ..Default::default()
        });
        let result = validate_schema(&schema, 1);
        assert!(!result.passed);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Shape 999 not found")));
    }

    #[test]
    fn test_validate_schema_info_not_found() {
        let mut schema = create_basic_schema();
        schema.pieces.push(Piece {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 100.0,
            height: 100.0,
            info_id: Some(999), // Non-existent info
            ..Default::default()
        });
        let result = validate_schema(&schema, 1);
        assert!(!result.passed);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Info 999 not found")));
    }

    #[test]
    fn test_validate_schema_empty_shape_warning() {
        let mut schema = create_basic_schema();
        schema.shapes.push(Shape {
            id: 1,
            cuts: vec![], // Empty shape
            ..Default::default()
        });
        let result = validate_schema(&schema, 1);
        assert!(result.passed); // Empty shape is just a warning
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("No cuts defined")));
    }

    #[test]
    fn test_validate_schema_arc_radius_too_small() {
        let mut schema = create_basic_schema();
        let mut shape = Shape::new(1);
        // Arc with endpoints at (0,0) to (100, 0) but radius only 10
        // Chord length is 100, so minimum radius is 50
        let mut cut = Cut {
            cut_type: CutType::ArcCW,
            xi: 0.0,
            yi: 0.0,
            xf: 100.0,
            yf: 0.0,
            radius: 10.0, // Too small
            active: true,
            ..Default::default()
        };
        cut.calculate_arc_center();
        shape.cuts.push(cut);
        schema.shapes.push(shape);

        let result = validate_schema(&schema, 1);
        assert!(!result.passed);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Arc radius") && e.contains("too small")));
    }

    #[test]
    fn test_validate_schema_no_cuts_warning() {
        let schema = create_basic_schema();
        // No linear cuts or shapes
        let result = validate_schema(&schema, 1);
        assert!(result.passed);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("No cuts or shapes")));
    }

    // ==================== validate_has_cuts tests ====================

    #[test]
    fn test_validate_has_cuts_no_cuts() {
        let schema = create_basic_schema();
        assert!(!validate_has_cuts(&schema));
    }

    #[test]
    fn test_validate_has_cuts_with_linear_cuts() {
        let mut schema = create_basic_schema();
        schema.linear_cuts.push(Cut {
            active: true,
            ..Default::default()
        });
        assert!(validate_has_cuts(&schema));
    }

    #[test]
    fn test_validate_has_cuts_with_inactive_cuts() {
        let mut schema = create_basic_schema();
        schema.linear_cuts.push(Cut {
            active: false,
            ..Default::default()
        });
        assert!(!validate_has_cuts(&schema));
    }

    #[test]
    fn test_validate_has_cuts_with_shapes() {
        let mut schema = create_basic_schema();
        let mut shape = Shape::new(1);
        shape.cuts.push(Cut::new_line(0.0, 0.0, 100.0, 0.0));
        schema.shapes.push(shape);
        assert!(validate_has_cuts(&schema));
    }

    // ==================== validate_piece_layout tests ====================

    #[test]
    fn test_validate_piece_layout_no_overlap() {
        let mut schema = create_basic_schema();
        schema.pieces.push(Piece {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 100.0,
            height: 100.0,
            ..Default::default()
        });
        schema.pieces.push(Piece {
            x_origin: 200.0, // No overlap
            y_origin: 0.0,
            width: 100.0,
            height: 100.0,
            ..Default::default()
        });
        let overlaps = validate_piece_layout(&schema);
        assert!(overlaps.is_empty());
    }

    #[test]
    fn test_validate_piece_layout_with_overlap() {
        let mut schema = create_basic_schema();
        schema.pieces.push(Piece {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 100.0,
            height: 100.0,
            ..Default::default()
        });
        schema.pieces.push(Piece {
            x_origin: 50.0, // Overlaps with first piece
            y_origin: 50.0,
            width: 100.0,
            height: 100.0,
            ..Default::default()
        });
        let overlaps = validate_piece_layout(&schema);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0], (0, 1));
    }

    #[test]
    fn test_validate_piece_layout_adjacent_no_overlap() {
        let mut schema = create_basic_schema();
        schema.pieces.push(Piece {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 100.0,
            height: 100.0,
            ..Default::default()
        });
        schema.pieces.push(Piece {
            x_origin: 100.0, // Exactly adjacent, no overlap
            y_origin: 0.0,
            width: 100.0,
            height: 100.0,
            ..Default::default()
        });
        let overlaps = validate_piece_layout(&schema);
        assert!(overlaps.is_empty());
    }

    // ==================== quick_validate tests ====================

    #[test]
    fn test_quick_validate_success() {
        let schema = create_basic_schema();
        let result = quick_validate(&[schema]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quick_validate_failure() {
        let mut schema = create_basic_schema();
        schema.width = -100.0; // Invalid
        let result = quick_validate(&[schema]);
        assert!(result.is_err());
    }
}
