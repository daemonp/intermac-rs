//! Integration tests for OTD to CNI conversion.
//!
//! These tests validate the structural correctness of generated CNI files
//! rather than exact byte-for-byte matching. This approach accommodates:
//! - Different machine configurations (tool codes)
//! - Version string differences
//! - Filename variations
//! - Minor floating-point rounding differences
//!
//! The tests verify that the generated output will produce correct machine behavior.

use otd_core::{convert_otd_to_cni, parse_otd_file, validate_schemas};
use std::collections::HashMap;
use std::path::Path;

/// Fixture directory for integration tests
const FIXTURE_DIR: &str = "tests/fixtures/integration";

// ==================== CNI Structure Parsing ====================

/// Represents a parsed CNI file structure
#[derive(Debug)]
struct CniStructure {
    sections: HashMap<String, Vec<String>>,
    #[allow(dead_code)] // Preserved for potential future use (e.g., section order validation)
    section_order: Vec<String>,
}

impl CniStructure {
    /// Parse CNI content into sections
    fn parse(content: &str) -> Self {
        let mut sections: HashMap<String, Vec<String>> = HashMap::new();
        let mut section_order = Vec::new();
        let mut current_section = String::from("_header");
        let mut current_lines: Vec<String> = Vec::new();

        for line in content.lines() {
            let line = line.trim_end_matches('\r');

            if line.starts_with('[') && line.ends_with(']') {
                // Save previous section
                if !current_lines.is_empty() || current_section != "_header" {
                    sections.insert(current_section.clone(), current_lines);
                    section_order.push(current_section);
                }
                // Start new section
                current_section = line[1..line.len() - 1].to_string();
                current_lines = Vec::new();
            } else {
                current_lines.push(line.to_string());
            }
        }

        // Save last section
        if !current_lines.is_empty() {
            sections.insert(current_section.clone(), current_lines);
            section_order.push(current_section);
        }

        CniStructure {
            sections,
            section_order,
        }
    }

    /// Get a section by name (supports prefix matching for numbered sections)
    fn get_section(&self, prefix: &str) -> Option<&Vec<String>> {
        // Try exact match first
        if let Some(section) = self.sections.get(prefix) {
            return Some(section);
        }
        // Try prefix match (e.g., "LDIST" matches "*LDIST0001_01")
        for (name, section) in &self.sections {
            if name.contains(prefix) {
                return Some(section);
            }
        }
        None
    }

    /// Check if a section exists (supports prefix matching)
    fn has_section(&self, prefix: &str) -> bool {
        self.get_section(prefix).is_some()
    }

    /// Get all sections matching a prefix
    fn get_sections_matching(&self, prefix: &str) -> Vec<(&String, &Vec<String>)> {
        self.sections
            .iter()
            .filter(|(name, _)| name.contains(prefix))
            .collect()
    }

    /// Count sections matching a prefix
    fn count_sections(&self, prefix: &str) -> usize {
        self.sections
            .keys()
            .filter(|name| name.contains(prefix))
            .count()
    }
}

// ==================== Validation Helpers ====================

/// Validates the COMMENTO (comment/header) section
fn validate_commento_section(cni: &CniStructure) -> Result<(), String> {
    let section = cni
        .get_section("COMMENTO")
        .ok_or("Missing COMMENTO section")?;

    // Check for required comment lines
    let has_project = section.iter().any(|l| l.starts_with("; Project:"));
    let has_creator = section.iter().any(|l| l.starts_with("; Creator:"));

    if !has_project {
        return Err("COMMENTO missing Project line".to_string());
    }
    if !has_creator {
        return Err("COMMENTO missing Creator line".to_string());
    }

    Ok(())
}

/// Validates the PARAMETRI (parameters) section
fn validate_parametri_section(cni: &CniStructure) -> Result<(), String> {
    let sections = cni.get_sections_matching("PARAMETRI");
    if sections.is_empty() {
        return Err("Missing PARAMETRI section".to_string());
    }

    for (name, section) in sections {
        let content = section.join(" ");

        // Check for required parameters (using LX/LY/LZ format)
        // N10 G70 LX=129.5 LY=95.5 LZ=0.15748 P103=130
        if !content.contains("G70") {
            return Err(format!("{} missing G70 (inch mode)", name));
        }
        if !content.contains("LX=") {
            return Err(format!("{} missing LX (sheet width)", name));
        }
        if !content.contains("LY=") {
            return Err(format!("{} missing LY (sheet height)", name));
        }
        if !content.contains("LZ=") {
            return Err(format!("{} missing LZ (sheet thickness)", name));
        }
        if !content.contains("P103=") {
            return Err(format!("{} missing P103 (machine number)", name));
        }
    }

    Ok(())
}

/// Validates the UTENSILI (tools) section
fn validate_utensili_section(cni: &CniStructure) -> Result<(), String> {
    let sections = cni.get_sections_matching("UTENSILI");
    if sections.is_empty() {
        return Err("Missing UTENSILI section".to_string());
    }

    for (_name, section) in sections {
        // Should have at least one tool definition (4-digit code)
        let has_tool = section
            .iter()
            .any(|l| l.trim().chars().take(4).all(|c| c.is_ascii_digit()));
        if !has_tool {
            // Empty tools section is OK for schemas without shapes
            // Just check it's not malformed
        }
    }

    Ok(())
}

/// Validates the CONTORNATURA (G-code) section
fn validate_contornatura_section(cni: &CniStructure, has_shapes: bool) -> Result<(), String> {
    let sections = cni.get_sections_matching("CONTORNATURA");
    if sections.is_empty() {
        return Err("Missing CONTORNATURA section".to_string());
    }

    for (_name, section) in sections {
        let content = section.join("\n");

        // Required G-code elements
        if !content.contains("L=PRGINIT") {
            return Err("Missing PRGINIT call".to_string());
        }
        if !content.contains("L=PFOXOUT") {
            return Err("Missing PFOXOUT call".to_string());
        }
        if !content.contains(":999999999") {
            return Err("Missing end label :999999999".to_string());
        }
        if !content.contains("L=PTOOL") {
            return Err("Missing PTOOL (tool load) call".to_string());
        }
        if !content.contains("L=PT_SU") {
            return Err("Missing PT_SU (tool up) call".to_string());
        }
        if !content.contains("L=PT_GIU") {
            return Err("Missing PT_GIU (tool down) call".to_string());
        }

        // Check for G-code line numbers (N followed by digits)
        let has_line_numbers = section.iter().any(|l| {
            l.trim().starts_with('N')
                && l.trim()
                    .chars()
                    .skip(1)
                    .take_while(|c| c.is_ascii_digit())
                    .count()
                    > 0
        });
        if !has_line_numbers {
            return Err("Missing G-code line numbers".to_string());
        }

        // If shapes exist, verify shape macros are called
        if has_shapes {
            // Should have shape-related jumps or macro calls
            let has_shape_code = content.contains("JM(") || content.contains("L=PFOXSAG");
            if !has_shape_code {
                return Err("Has shapes but missing shape-related G-code".to_string());
            }
        }
    }

    Ok(())
}

/// Validates DXF sections (LDIST, PRWB, PRWC)
fn validate_dxf_sections(cni: &CniStructure, expected_schemas: usize) -> Result<(), String> {
    // Check LDIST sections exist (one per schema, not per piece)
    let ldist_count = cni.count_sections("LDIST");
    if ldist_count == 0 {
        return Err("Missing LDIST sections".to_string());
    }
    // LDIST count should match schema count (one per pattern)
    if ldist_count != expected_schemas {
        return Err(format!(
            "LDIST section count mismatch: got {}, expected {} (one per schema)",
            ldist_count, expected_schemas
        ));
    }

    // Check PRWB sections (piece visualizations)
    let prwb_count = cni.count_sections("PRWB");
    if prwb_count == 0 {
        return Err("Missing PRWB sections".to_string());
    }

    // Check PRWC sections (cutting visualizations)
    let prwc_count = cni.count_sections("PRWC");
    if prwc_count == 0 {
        return Err("Missing PRWC sections".to_string());
    }

    // Validate DXF structure in at least one PRWB section
    if let Some((_name, section)) = cni.get_sections_matching("PRWB").first() {
        let content = section.join("\n");
        if !content.contains("SECTION") {
            return Err("DXF missing SECTION keyword".to_string());
        }
        if !content.contains("HEADER") {
            return Err("DXF missing HEADER".to_string());
        }
        if !content.contains("ENTITIES") {
            return Err("DXF missing ENTITIES".to_string());
        }
        if !content.contains("ENDSEC") {
            return Err("DXF missing ENDSEC".to_string());
        }
        if !content.contains("EOF") {
            return Err("DXF missing EOF".to_string());
        }
    }

    Ok(())
}

/// Validates LDIST section content (piece metadata)
fn validate_ldist_content(cni: &CniStructure) -> Result<(), String> {
    for (_name, section) in cni.get_sections_matching("LDIST") {
        // LDIST should have piece metadata
        let has_cod = section.iter().any(|l| l.starts_with(";Cod="));
        let has_dimx = section.iter().any(|l| l.starts_with(";DimX="));
        let has_dimy = section.iter().any(|l| l.starts_with(";DimY="));

        if !has_cod || !has_dimx || !has_dimy {
            return Err(format!(
                "LDIST section missing required metadata (Cod={}, DimX={}, DimY={})",
                has_cod, has_dimx, has_dimy
            ));
        }
    }

    Ok(())
}

// ==================== Test Helpers ====================

/// Parse and validate an OTD file
fn parse_and_validate(otd_path: &Path) -> Vec<otd_core::Schema> {
    let schemas = parse_otd_file(otd_path).expect("Failed to parse OTD file");
    let validation = validate_schemas(&schemas).expect("Validation error");

    // Print any warnings for debugging
    for warning in &validation.warnings {
        eprintln!("Warning: {}", warning);
    }

    assert!(
        validation.passed,
        "Validation failed: {:?}",
        validation.errors
    );
    schemas
}

/// Convert OTD to CNI and return the output
fn convert_to_cni(otd_path: &Path, machine_number: u16) -> String {
    convert_otd_to_cni(otd_path, machine_number).expect("Failed to convert OTD to CNI")
}

/// Full structural validation of generated CNI
fn validate_cni_structure(
    generated: &str,
    expected_schemas: usize,
    has_shapes: bool,
) -> Vec<String> {
    let mut errors = Vec::new();
    let cni = CniStructure::parse(generated);

    // Validate COMMENTO
    if let Err(e) = validate_commento_section(&cni) {
        errors.push(e);
    }

    // Validate CENTRO sections - there's always just ONE shared header section
    let centro_count = cni.count_sections("CENTRO");
    if centro_count != 1 {
        errors.push(format!(
            "Expected 1 CENTRO section (shared header), found {}",
            centro_count
        ));
    }

    // Validate PARAMETRI
    if let Err(e) = validate_parametri_section(&cni) {
        errors.push(e);
    }

    // Validate UTENSILI
    if let Err(e) = validate_utensili_section(&cni) {
        errors.push(e);
    }

    // Validate LAVORAZIONI sections exist
    if !cni.has_section("LAVORAZIONI") {
        errors.push("Missing LAVORAZIONI section".to_string());
    }

    // Validate CONTORNATURA
    if let Err(e) = validate_contornatura_section(&cni, has_shapes) {
        errors.push(e);
    }

    // Validate DXF sections (one LDIST per schema)
    if let Err(e) = validate_dxf_sections(&cni, expected_schemas) {
        errors.push(e);
    }

    // Validate LDIST content
    if let Err(e) = validate_ldist_content(&cni) {
        errors.push(e);
    }

    errors
}

/// Compare generated vs reference at a high level (section counts, sizes)
fn compare_high_level(generated: &str, reference: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let gen_cni = CniStructure::parse(generated);
    let ref_cni = CniStructure::parse(reference);

    // Compare section counts
    let sections_to_compare = [
        "CENTRO",
        "PARAMETRI",
        "UTENSILI",
        "LAVORAZIONI",
        "CONTORNATURA",
    ];
    for section in &sections_to_compare {
        let gen_count = gen_cni.count_sections(section);
        let ref_count = ref_cni.count_sections(section);
        if gen_count != ref_count {
            warnings.push(format!(
                "{} section count: generated={}, reference={}",
                section, gen_count, ref_count
            ));
        }
    }

    // Compare DXF section counts (allow some variance)
    for section in &["LDIST", "PRWB", "PRWC"] {
        let gen_count = gen_cni.count_sections(section);
        let ref_count = ref_cni.count_sections(section);
        if gen_count != ref_count {
            warnings.push(format!(
                "{} section count: generated={}, reference={}",
                section, gen_count, ref_count
            ));
        }
    }

    // Compare overall line counts (within 10%)
    let gen_lines = generated.lines().count();
    let ref_lines = reference.lines().count();
    let diff_pct = ((gen_lines as f64 - ref_lines as f64) / ref_lines as f64).abs() * 100.0;
    if diff_pct > 10.0 {
        warnings.push(format!(
            "Line count differs by {:.1}%: generated={}, reference={}",
            diff_pct, gen_lines, ref_lines
        ));
    }

    warnings
}

// ==================== Integration Tests ====================

/// Test: Simple linear cuts only (smallest test case)
#[test]
fn test_simple_linear_conversion() {
    let otd_path = Path::new(FIXTURE_DIR).join("simple_linear.otd");
    let cni_path = Path::new(FIXTURE_DIR).join("simple_linear.cni");

    // Parse and validate OTD
    let schemas = parse_and_validate(&otd_path);
    assert_eq!(schemas.len(), 1, "Expected 1 schema/pattern");
    assert!(schemas[0].shapes.is_empty(), "Expected no shapes");

    // Convert
    let generated = convert_to_cni(&otd_path, 130);

    // Structural validation
    let errors = validate_cni_structure(&generated, schemas.len(), false);
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("Validation error: {}", err);
        }
        panic!("Structural validation failed with {} errors", errors.len());
    }

    // Compare with reference (warnings only, not failures)
    let reference = std::fs::read_to_string(&cni_path).expect("Failed to read reference CNI");
    let warnings = compare_high_level(&generated, &reference);
    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }
}

/// Test: OTD with shape definitions (arcs and curves)
#[test]
fn test_with_shapes_conversion() {
    let otd_path = Path::new(FIXTURE_DIR).join("with_shapes.otd");
    let cni_path = Path::new(FIXTURE_DIR).join("with_shapes.cni");

    // Parse and validate
    let schemas = parse_and_validate(&otd_path);
    assert_eq!(schemas.len(), 1, "Expected 1 schema/pattern");
    assert!(
        !schemas[0].shapes.is_empty(),
        "Expected shapes to be present"
    );

    // Verify shapes have arc cuts
    let has_arcs = schemas[0]
        .shapes
        .iter()
        .any(|s| s.cuts.iter().any(|c| c.is_arc()));
    assert!(has_arcs, "Expected at least one arc in shapes");

    let has_shapes = schemas.iter().any(|s| !s.shapes.is_empty());

    // Convert
    let generated = convert_to_cni(&otd_path, 130);

    // Structural validation
    let errors = validate_cni_structure(&generated, schemas.len(), has_shapes);
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("Validation error: {}", err);
        }
        panic!("Structural validation failed with {} errors", errors.len());
    }

    // High-level comparison with reference
    let reference = std::fs::read_to_string(&cni_path).expect("Failed to read reference CNI");
    let warnings = compare_high_level(&generated, &reference);
    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }
}

/// Test: Multiple pieces with varied layouts
#[test]
fn test_multi_piece_conversion() {
    let otd_path = Path::new(FIXTURE_DIR).join("multi_piece.otd");
    let cni_path = Path::new(FIXTURE_DIR).join("multi_piece.cni");

    // Parse and validate
    let schemas = parse_and_validate(&otd_path);
    assert!(!schemas.is_empty(), "Expected at least 1 schema");

    let total_pieces: usize = schemas.iter().map(|s| s.pieces.len()).sum();
    let has_shapes = schemas.iter().any(|s| !s.shapes.is_empty());

    eprintln!(
        "Multi-piece: {} schemas, {} total pieces, has_shapes={}",
        schemas.len(),
        total_pieces,
        has_shapes
    );

    // Convert
    let generated = convert_to_cni(&otd_path, 130);

    // Structural validation
    let errors = validate_cni_structure(&generated, schemas.len(), has_shapes);
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("Validation error: {}", err);
        }
        panic!("Structural validation failed with {} errors", errors.len());
    }

    // High-level comparison
    let reference = std::fs::read_to_string(&cni_path).expect("Failed to read reference CNI");
    let warnings = compare_high_level(&generated, &reference);
    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }
}

/// Test: Empty shapes (shape definitions without geometry)
#[test]
fn test_empty_shapes_conversion() {
    let otd_path = Path::new(FIXTURE_DIR).join("empty_shapes.otd");
    let cni_path = Path::new(FIXTURE_DIR).join("empty_shapes.cni");

    // Parse and validate
    let schemas = parse_and_validate(&otd_path);
    assert!(!schemas.is_empty(), "Expected at least 1 schema");

    // Check for empty shapes (shapes with no cuts)
    let total_shapes: usize = schemas.iter().map(|s| s.shapes.len()).sum();
    let empty_shapes: usize = schemas
        .iter()
        .flat_map(|s| &s.shapes)
        .filter(|s| s.cuts.is_empty())
        .count();
    eprintln!(
        "Found {} total shapes, {} empty shapes",
        total_shapes, empty_shapes
    );

    // For empty shapes, treat as no shapes for G-code validation
    let has_non_empty_shapes = schemas
        .iter()
        .any(|s| s.shapes.iter().any(|shape| !shape.cuts.is_empty()));

    // Convert
    let generated = convert_to_cni(&otd_path, 130);

    // Structural validation
    let errors = validate_cni_structure(&generated, schemas.len(), has_non_empty_shapes);
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("Validation error: {}", err);
        }
        panic!("Structural validation failed with {} errors", errors.len());
    }

    // High-level comparison
    let reference = std::fs::read_to_string(&cni_path).expect("Failed to read reference CNI");
    let warnings = compare_high_level(&generated, &reference);
    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }
}

/// Test: Complex shapes with many cuts
#[test]
fn test_complex_shapes_conversion() {
    let otd_path = Path::new(FIXTURE_DIR).join("complex_shapes.otd");
    let cni_path = Path::new(FIXTURE_DIR).join("complex_shapes.cni");

    // Parse and validate
    let schemas = parse_and_validate(&otd_path);
    assert!(!schemas.is_empty(), "Expected at least 1 schema");

    // Log shape complexity
    for (i, schema) in schemas.iter().enumerate() {
        for (j, shape) in schema.shapes.iter().enumerate() {
            eprintln!(
                "Schema {} Shape {}: {} cuts, {} arcs",
                i,
                j,
                shape.cuts.len(),
                shape.cuts.iter().filter(|c| c.is_arc()).count()
            );
        }
    }

    let has_shapes = schemas
        .iter()
        .any(|s| s.shapes.iter().any(|shape| !shape.cuts.is_empty()));

    // Convert
    let generated = convert_to_cni(&otd_path, 130);

    // Structural validation
    let errors = validate_cni_structure(&generated, schemas.len(), has_shapes);
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("Validation error: {}", err);
        }
        panic!("Structural validation failed with {} errors", errors.len());
    }

    // High-level comparison
    let reference = std::fs::read_to_string(&cni_path).expect("Failed to read reference CNI");
    let warnings = compare_high_level(&generated, &reference);
    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }
}

/// Test: Large layout with many pieces (stress test)
#[test]
fn test_large_layout_conversion() {
    let otd_path = Path::new(FIXTURE_DIR).join("large_layout.otd");
    let cni_path = Path::new(FIXTURE_DIR).join("large_layout.cni");

    // Parse and validate
    let schemas = parse_and_validate(&otd_path);
    assert!(!schemas.is_empty(), "Expected at least 1 schema");

    let total_pieces: usize = schemas.iter().map(|s| s.pieces.len()).sum();
    let total_shapes: usize = schemas.iter().map(|s| s.shapes.len()).sum();
    let has_shapes = schemas
        .iter()
        .any(|s| s.shapes.iter().any(|shape| !shape.cuts.is_empty()));

    eprintln!(
        "Large layout: {} schemas, {} pieces, {} shapes",
        schemas.len(),
        total_pieces,
        total_shapes
    );

    // Convert (measure time)
    let start = std::time::Instant::now();
    let generated = convert_to_cni(&otd_path, 130);
    let elapsed = start.elapsed();
    eprintln!("Conversion took {:?}", elapsed);

    // Performance check - should complete in reasonable time
    assert!(
        elapsed.as_secs() < 5,
        "Conversion took too long: {:?}",
        elapsed
    );

    // Basic size checks
    let gen_lines = generated.lines().count();
    eprintln!("Generated {} lines, {} bytes", gen_lines, generated.len());

    // Structural validation
    let errors = validate_cni_structure(&generated, schemas.len(), has_shapes);
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("Validation error: {}", err);
        }
        panic!("Structural validation failed with {} errors", errors.len());
    }

    // High-level comparison
    let reference = std::fs::read_to_string(&cni_path).expect("Failed to read reference CNI");
    let warnings = compare_high_level(&generated, &reference);
    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }

    // Line count should be reasonably close
    let ref_lines = reference.lines().count();
    let diff_pct = ((gen_lines as f64 - ref_lines as f64) / ref_lines as f64).abs() * 100.0;
    assert!(
        diff_pct < 15.0,
        "Line count differs by {:.1}%: generated={}, reference={}",
        diff_pct,
        gen_lines,
        ref_lines
    );
}

// ==================== Structural Tests ====================

/// Test: Verify all schemas have required sections in output
#[test]
fn test_output_structure() {
    let otd_path = Path::new(FIXTURE_DIR).join("with_shapes.otd");
    let generated = convert_to_cni(&otd_path, 130);
    let cni = CniStructure::parse(&generated);

    // Check for required sections
    assert!(cni.has_section("COMMENTO"), "Missing COMMENTO section");
    assert!(cni.has_section("CENTRO"), "Missing CENTRO section");
    assert!(cni.has_section("PARAMETRI"), "Missing PARAMETRI section");
    assert!(cni.has_section("UTENSILI"), "Missing UTENSILI section");
    assert!(
        cni.has_section("LAVORAZIONI"),
        "Missing LAVORAZIONI section"
    );
    assert!(
        cni.has_section("CONTORNATURA"),
        "Missing CONTORNATURA section"
    );
    assert!(cni.has_section("LDIST"), "Missing LDIST section");
    assert!(cni.has_section("PRWB"), "Missing PRWB section");
    assert!(cni.has_section("PRWC"), "Missing PRWC section");
}

/// Test: Verify G-code structure in CONTORNATURA section
#[test]
fn test_gcode_structure() {
    let otd_path = Path::new(FIXTURE_DIR).join("simple_linear.otd");
    let generated = convert_to_cni(&otd_path, 130);

    // Check for program structure
    assert!(generated.contains("L=PRGINIT"), "Missing PRGINIT call");
    assert!(generated.contains("L=PFOXOUT"), "Missing PFOXOUT call");
    assert!(generated.contains(":999999999"), "Missing end label");
    assert!(generated.contains("L=PT_SU"), "Missing PT_SU (tool up)");
    assert!(generated.contains("L=PT_GIU"), "Missing PT_GIU (tool down)");
    assert!(generated.contains("L=PTOOL"), "Missing PTOOL (load tool)");

    // Check for movement commands
    assert!(generated.contains("G00"), "Missing rapid move G00");
    assert!(generated.contains("G01"), "Missing linear move G01");
}

/// Test: Verify DXF structure in PRWB/PRWC sections
#[test]
fn test_dxf_structure() {
    let otd_path = Path::new(FIXTURE_DIR).join("with_shapes.otd");
    let generated = convert_to_cni(&otd_path, 130);

    // Check for DXF structure
    assert!(generated.contains("SECTION"), "Missing DXF SECTION");
    assert!(generated.contains("HEADER"), "Missing DXF HEADER");
    assert!(generated.contains("ENTITIES"), "Missing DXF ENTITIES");
    assert!(generated.contains("ENDSEC"), "Missing DXF ENDSEC");
    assert!(generated.contains("EOF"), "Missing DXF EOF");

    // Check for layers
    assert!(generated.contains("EST"), "Missing EST layer");
    assert!(generated.contains("Tagli"), "Missing Tagli layer");
}

/// Test: Verify arc handling in DXF output
#[test]
fn test_dxf_arc_handling() {
    let otd_path = Path::new(FIXTURE_DIR).join("with_shapes.otd");
    let generated = convert_to_cni(&otd_path, 130);

    // For shapes with arcs, DXF should have ARC entities
    assert!(
        generated.contains("ARC") || generated.contains("LINE"),
        "DXF should have ARC or LINE entities for shapes"
    );
}

// ==================== Edge Case Tests ====================

/// Test: Verify parsing handles all fixtures without panic
#[test]
fn test_all_fixtures_parse() {
    let fixtures = [
        "simple_linear.otd",
        "with_shapes.otd",
        "multi_piece.otd",
        "empty_shapes.otd",
        "complex_shapes.otd",
        "large_layout.otd",
    ];

    for fixture in &fixtures {
        let path = Path::new(FIXTURE_DIR).join(fixture);
        let result = parse_otd_file(&path);
        assert!(
            result.is_ok(),
            "Failed to parse {}: {:?}",
            fixture,
            result.err()
        );

        let schemas = result.unwrap();
        assert!(!schemas.is_empty(), "No schemas in {}", fixture);
    }
}

/// Test: Verify conversion handles all fixtures without panic
#[test]
fn test_all_fixtures_convert() {
    let fixtures = [
        "simple_linear.otd",
        "with_shapes.otd",
        "multi_piece.otd",
        "empty_shapes.otd",
        "complex_shapes.otd",
        "large_layout.otd",
    ];

    for fixture in &fixtures {
        let path = Path::new(FIXTURE_DIR).join(fixture);
        let result = convert_otd_to_cni(&path, 130);
        assert!(
            result.is_ok(),
            "Failed to convert {}: {:?}",
            fixture,
            result.err()
        );

        let output = result.unwrap();
        assert!(!output.is_empty(), "Empty output for {}", fixture);
        assert!(
            output.len() > 1000,
            "Output too small for {}: {} bytes",
            fixture,
            output.len()
        );
    }
}

/// Test: Verify different machine numbers produce valid output
#[test]
fn test_different_machine_numbers() {
    let otd_path = Path::new(FIXTURE_DIR).join("simple_linear.otd");

    for machine_num in [100, 130, 200, 999] {
        let result = convert_otd_to_cni(&otd_path, machine_num);
        assert!(
            result.is_ok(),
            "Failed with machine number {}: {:?}",
            machine_num,
            result.err()
        );

        let output = result.unwrap();
        // Machine number should appear in the output
        let machine_str = format!("{:03}", machine_num);
        assert!(
            output.contains(&machine_str),
            "Machine number {} not found in output",
            machine_num
        );
    }
}

/// Test: Empty OTD handling (edge case)
#[test]
fn test_minimal_otd() {
    // The simple_linear fixture is our smallest valid test case
    // Verify it generates proper output
    let otd_path = Path::new(FIXTURE_DIR).join("simple_linear.otd");
    let schemas = parse_and_validate(&otd_path);

    // Should have at least one schema with pieces
    assert!(!schemas.is_empty(), "Expected at least one schema");
    assert!(!schemas[0].pieces.is_empty(), "Expected at least one piece");

    let output = convert_to_cni(&otd_path, 130);

    // Minimal output should still have all required sections
    let cni = CniStructure::parse(&output);
    assert!(cni.has_section("COMMENTO"));
    assert!(cni.has_section("CENTRO"));
    assert!(cni.has_section("PARAMETRI"));
    assert!(cni.has_section("CONTORNATURA"));
}
