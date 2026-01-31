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
    for (_shape_idx, shape) in schema.shapes.iter().enumerate() {
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
