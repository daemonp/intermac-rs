//! otd-core - Core library for OTD file parsing and CNI generation.
//!
//! This library provides the core functionality for parsing OTD (Optimized Tool Data) files
//! and generating CNI (CNC ISO) files for Intermac glass cutting tables.
//!
//! # Example
//!
//! ```no_run
//! use otd_core::{parse_otd_file, generate_cni, MachineConfig};
//! use std::path::Path;
//!
//! let schemas = parse_otd_file(Path::new("layout.otd")).unwrap();
//! let config = MachineConfig::new(130);
//! let cni = generate_cni(&schemas, "layout.otd", &config).unwrap();
//! println!("{}", cni);
//! ```

pub mod config;
pub mod error;
pub mod generator;
pub mod model;
pub mod parser;
pub mod transform;
pub mod validation;

// Re-exports for convenience
pub use config::{MachineConfig, Unit};
pub use error::{ConvertError, Result};
pub use generator::generate_cni;
pub use model::{Cut, CutType, LineType, Piece, PieceType, Schema, Shape};
pub use parser::parse_otd_file;
pub use validation::{validate_schemas, ValidationResult};

/// Convert an OTD file to CNI format.
///
/// This is the main high-level function that performs the full conversion pipeline:
/// 1. Parse the OTD file
/// 2. Process/transform the cuts
/// 3. Validate the data
/// 4. Generate the CNI output
///
/// # Arguments
///
/// * `input_path` - Path to the input OTD or OTX file
/// * `machine_number` - Machine type number (100-199 for cutting tables)
///
/// # Returns
///
/// The generated CNI file content as a string.
pub fn convert_otd_to_cni(input_path: &std::path::Path, machine_number: u16) -> Result<String> {
    // Parse the OTD file
    let mut schemas = parse_otd_file(input_path)?;

    // Process each schema
    for schema in &mut schemas {
        // Process linear cuts
        transform::process_linear_cuts(schema);

        // Process shapes
        transform::process_shapes(schema);
    }

    // Validate
    let validation = validate_schemas(&schemas)?;
    if !validation.passed {
        // Log warnings but continue
        for warning in &validation.warnings {
            tracing::warn!("{}", warning);
        }
    }

    // Generate CNI
    let config = MachineConfig::new(machine_number);
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output.otd");

    generate_cni(&schemas, filename, &config)
}
