//! CNI file generator for cutting table machines (100-199).

use crate::config::{MachineConfig, DEFAULT_LINEAR_TOOL, DEFAULT_SHAPED_TOOL};
use crate::error::Result;
use crate::model::{CutType, Schema};
use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

use super::gcode::{format_coord, format_tool_code, GcodeWriter};

/// Generate a CNI file from parsed schemas.
pub fn generate_cni(
    schemas: &[Schema],
    input_filename: &str,
    config: &MachineConfig,
) -> Result<String> {
    let mut output = String::new();

    // Generate [COMMENTO] section
    generate_comment_section(&mut output, schemas, input_filename);

    // Generate [CENTRO01] section (empty)
    writeln!(output, "[CENTRO01]").unwrap();
    writeln!(output).unwrap();

    // Generate [PARAMETRI01] section
    generate_parameters_section(&mut output, schemas, config);

    // Generate [UTENSILI01] section
    generate_tools_section(&mut output, schemas);

    // Generate [LAVORAZIONI01] section (empty)
    writeln!(output, "[LAVORAZIONI01]").unwrap();
    writeln!(output, "%").unwrap();
    writeln!(output).unwrap();

    // Generate [CONTORNATURA01] section
    generate_contour_section(&mut output, schemas, config);

    // Generate [*LDIST] sections
    generate_distribution_sections(&mut output, schemas, input_filename);

    Ok(output)
}

/// Generate the [COMMENTO] section.
fn generate_comment_section(output: &mut String, schemas: &[Schema], filename: &str) {
    writeln!(output, "[COMMENTO]").unwrap();
    writeln!(output, "; Project: {}", filename).unwrap();

    if let Some(schema) = schemas.first() {
        writeln!(output, "; Material : {}", schema.glass_id).unwrap();
    }

    writeln!(output, "; Creator: otd-convert-rs").unwrap();
    writeln!(output, "; Version: 0.1.0").unwrap();
    writeln!(output).unwrap();
}

/// Generate the [PARAMETRI01] section.
fn generate_parameters_section(output: &mut String, schemas: &[Schema], config: &MachineConfig) {
    writeln!(output, "[PARAMETRI01]").unwrap();

    if let Some(schema) = schemas.first() {
        let unit_code = schema.unit.gcode();
        writeln!(
            output,
            "N10 {} LX={} LY={} LZ={} P103={}",
            unit_code,
            format_coord(schema.width),
            format_coord(schema.height),
            format_coord(schema.thickness),
            config.machine_number
        )
        .unwrap();
    }

    writeln!(output, "%").unwrap();
    writeln!(output).unwrap();
}

/// Generate the [UTENSILI01] section.
fn generate_tools_section(output: &mut String, schemas: &[Schema]) {
    writeln!(output, "[UTENSILI01]").unwrap();

    // Collect all used tools
    let mut tools: HashSet<u16> = HashSet::new();
    tools.insert(DEFAULT_LINEAR_TOOL);

    for schema in schemas {
        if !schema.shapes.is_empty() {
            tools.insert(DEFAULT_SHAPED_TOOL);
        }
        if schema.linear_tool > 0 {
            tools.insert(schema.linear_tool as u16);
        }
        if schema.shaped_tool > 0 {
            tools.insert(schema.shaped_tool as u16);
        }
    }

    let mut tools: Vec<_> = tools.into_iter().collect();
    tools.sort();

    for tool in tools {
        writeln!(output, "{}", format_tool_code(tool)).unwrap();
    }

    writeln!(output, "%").unwrap();
    writeln!(output).unwrap();
}

/// Generate the [CONTORNATURA01] section.
fn generate_contour_section(output: &mut String, schemas: &[Schema], config: &MachineConfig) {
    writeln!(output, "[CONTORNATURA01]").unwrap();

    let mut writer = GcodeWriter::with_start(20);

    // Program initialization
    writer.set_param(15, 1);
    writer.call_macro("PRGINIT");
    writer.write_line("JM:(P262)");
    writer.write_raw("");

    // Generate each pattern/schema
    for (schema_idx, schema) in schemas.iter().enumerate() {
        let schema_num = schema_idx + 1;
        generate_schema_code(&mut writer, schema, schema_num, config);
    }

    // Generate shape macros
    generate_shape_macros(&mut writer, schemas);

    // Program end
    writer.write_label("999999999");
    writer.call_macro("PFOXOUT");
    writer.write_terminator();

    write!(output, "{}", writer.take_output()).unwrap();
    writeln!(output).unwrap();
}

/// Generate G-code for a single schema/pattern.
fn generate_schema_code(
    writer: &mut GcodeWriter,
    schema: &Schema,
    schema_num: usize,
    _config: &MachineConfig,
) {
    let label = format!("{:04}", schema_num);

    writer.write_comment(&format!(
        "--- inizio Schema={} Lastre={}",
        schema_num, schema.quantity
    ));
    writer.write_label(&label);

    // Schema parameters
    // P012, P013, P014 are calculated values (simplified here)
    let p012 = schema.thickness * 0.098; // Simplified calculation
    let p013 = schema.thickness * schema.thickness * 0.032;
    let p014 = schema.width * schema.height * 0.001;

    writer.set_param_float(12, p012);
    writer.set_param_float(13, p013);
    writer.set_param_float(14, p014);
    writer.set_param(941, 0);

    // Calculate rest dimensions
    let pxrs = schema.width - schema.trim_left;
    let pyrs = schema.trim_bottom;
    writer.write_line(&format!("PXRS={}", format_coord(pxrs)));
    writer.write_line(&format!("PYRS={}", format_coord(pyrs)));
    writer.call_macro("PTMREP_B");

    // Tool selection jumps
    writer.write_comment("parte relativa al Taglio --------");
    let linear_label = format!("01{:04}", schema_num);
    let shaped_label = format!("02{:04}", schema_num);

    writer.write_line(&format!(
        "JM((P260=2)~(P007={:04})):{}",
        DEFAULT_LINEAR_TOOL, linear_label
    ));
    writer.write_line(&format!(
        "JM((P260=2)~(P007={:04})):{}",
        DEFAULT_SHAPED_TOOL, shaped_label
    ));
    writer.write_line("JM(P260=2):999999999");
    writer.write_raw("");

    // Linear cuts section
    writer.write_comment("parte geometrica lineare ----------");
    writer.write_label(&linear_label);
    generate_linear_cuts(writer, schema);
    writer.write_raw("");
    writer.write_line("JM(P260=2):999999999");
    writer.write_raw("");

    // Shape cuts section
    writer.write_comment("parte geometrica sagomata ----------");
    writer.write_label(&shaped_label);
    generate_shape_cuts(writer, schema, schema_num);
    writer.write_raw("");
    writer.write_line("JM(P260=2):999999999");
    writer.write_raw("");

    writer.jump("999999999");
    writer.write_raw("");
}

/// Generate linear cut G-code.
fn generate_linear_cuts(writer: &mut GcodeWriter, schema: &Schema) {
    writer.set_tool(DEFAULT_LINEAR_TOOL);
    writer.load_tool();
    writer.tool_up();

    for cut in &schema.linear_cuts {
        if !cut.active {
            continue;
        }

        // Set rotation based on cut direction
        let rotation = if cut.is_vertical() { 90.0 } else { 0.0 };
        writer.set_rotation(rotation);
        writer.apply_rotation();

        // Rapid move to start
        writer.rapid_move(cut.xi, cut.yi, Some("P540"));

        // Direction code
        writer.direction_code(cut.is_vertical());

        // Tool down
        writer.tool_down();

        // Linear cut
        writer.linear_move(cut.xf, cut.yf, Some("P540"));

        // Tool up
        writer.tool_up();
    }
}

/// Generate shape cut G-code.
fn generate_shape_cuts(writer: &mut GcodeWriter, schema: &Schema, schema_num: usize) {
    writer.set_tool(DEFAULT_SHAPED_TOOL);
    writer.load_tool();

    for piece in &schema.pieces {
        let shape_idx = match piece.shape_index {
            Some(idx) => idx,
            None => continue,
        };

        let _shape = &schema.shapes[shape_idx];

        // Generate macro call
        writer.write_comment("----");
        writer.set_work_offset();
        writer.set_xo(piece.x_origin);
        writer.set_yo(piece.y_origin);

        let macro_label = format!("101{:04}{:03}", schema_num, shape_idx + 1);
        writer.call_label(&macro_label);
    }
}

/// Generate shape macro definitions.
fn generate_shape_macros(writer: &mut GcodeWriter, schemas: &[Schema]) {
    writer.write_comment("macro delle icone ----------");

    for (schema_idx, schema) in schemas.iter().enumerate() {
        let schema_num = schema_idx + 1;

        for (shape_idx, shape) in schema.shapes.iter().enumerate() {
            let macro_label = format!("101{:04}{:03}", schema_num, shape_idx + 1);
            writer.write_label(&macro_label);

            // Shape macro content
            writer.tool_up();
            writer.set_rotation(shape.rotation);
            writer.apply_rotation();

            if let Some((start_x, start_y)) = shape.start_point() {
                writer.write_line(&format!(
                    "G00 X={} Y={} C=P540 AR=P540",
                    format_coord(start_x),
                    format_coord(start_y)
                ));
            }

            writer.set_shape_params(shape.perimeter, 1);
            writer.tool_down();
            writer.tangent_mode_on();

            // Generate shape segments
            for cut in &shape.cuts {
                if !cut.active {
                    continue;
                }

                match cut.cut_type {
                    CutType::Line => {
                        writer.linear_move(cut.xf, cut.yf, None);
                    }
                    CutType::ArcCW => {
                        writer.arc_cw(cut.xf, cut.yf, cut.xc, cut.yc);
                    }
                    CutType::ArcCCW => {
                        writer.arc_ccw(cut.xf, cut.yf, cut.xc, cut.yc);
                    }
                }
            }

            writer.tangent_mode_off();
            writer.tool_up();
            writer.write_terminator();
            writer.write_raw("");
        }
    }
}

/// Generate [*LDIST] distribution sections.
fn generate_distribution_sections(output: &mut String, schemas: &[Schema], input_filename: &str) {
    let base_name = Path::new(input_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    for (schema_idx, schema) in schemas.iter().enumerate() {
        let section_name = format!("*LDIST{:04}_01", schema_idx + 1);
        writeln!(output, "[{}]", section_name).unwrap();

        // Code identifier
        writeln!(output, ";Cod={}{}", base_name, schema_idx + 1).unwrap();

        // Sheet dimensions
        writeln!(output, ";DimX={}", format_coord(schema.width)).unwrap();
        writeln!(output, ";DimY={}", format_coord(schema.height)).unwrap();
        writeln!(output, ";Spes={}", format_coord(schema.thickness)).unwrap();
        writeln!(output, ";Qta={}", schema.quantity).unwrap();
        writeln!(output, ";TipoVetro={}", schema.glass_id).unwrap();

        // Piece information
        for piece_type in &schema.piece_types {
            writeln!(output, ";NomeSagoma=").unwrap();
            writeln!(output, ";CodPz={}", piece_type.piece_code).unwrap();
            writeln!(output, ";DimXPz={}", format_coord(piece_type.sheet_width)).unwrap();
            writeln!(output, ";DimYPz={}", format_coord(piece_type.sheet_height)).unwrap();

            // Count pieces of this type
            let count = schema
                .pieces
                .iter()
                .filter(|p| p.info_id == Some(piece_type.id))
                .count();
            writeln!(output, ";QtaPz={}", count).unwrap();

            writeln!(output, ";ClientePz={}", piece_type.customer).unwrap();
            writeln!(output, ";OrdinePz={}", piece_type.order_no).unwrap();
        }

        writeln!(output, "%").unwrap();
    }
}
