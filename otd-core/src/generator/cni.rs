//! CNI file generator for cutting table machines (100-199).

use crate::config::{
    MachineConfig, DEFAULT_LINEAR_TOOL, DEFAULT_SHAPED_TOOL, EPS_COARSE, TOOL_TYPE_SHAPED,
};
use crate::error::Result;
use crate::model::{CutType, Schema};
use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

use super::dxf::generate_dxf_sections;
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

    // Generate [*PRWB] and [*PRWC] DXF visualization sections
    output.push_str(&generate_dxf_sections(schemas));

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

    for schema in schemas {
        // Add linear tool if there are linear cuts
        if !schema.linear_cuts.is_empty() {
            if schema.linear_tool > 0 {
                tools.insert(schema.linear_tool as u16);
            } else {
                tools.insert(DEFAULT_LINEAR_TOOL);
            }
        }

        // Add shaped tool if shapes actually use it (have cuts with that tool type)
        if schema_uses_tool_type(schema, TOOL_TYPE_SHAPED as usize) {
            if schema.shaped_tool > 0 {
                tools.insert(schema.shaped_tool as u16);
            } else {
                tools.insert(DEFAULT_SHAPED_TOOL);
            }
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

    // Extra blank line before program end
    writer.write_raw("");

    // Program end
    writer.write_label("999999999");
    writer.call_macro("PFOXOUT");
    writer.write_terminator();

    write!(output, "{}", writer.take_output()).unwrap();
    writeln!(output).unwrap();
}

/// Check if schema has shapes that use a specific tool type.
fn schema_uses_tool_type(schema: &Schema, tool_type: usize) -> bool {
    schema.shapes.iter().any(|shape| shape.uses_tool(tool_type))
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

    // Schema parameters - P012/P013/P014 are area calculations
    // P012 = total sheet area / 1,000,000 (in square mm or equivalent)
    // P013 = waste area / 1,000,000
    // P014 = waste percentage
    let total_area = schema.height * schema.width;
    let piece_area = calculate_piece_area(schema);
    let waste_area = total_area - piece_area;

    let p012 = total_area / 1_000_000.0;
    let p013 = waste_area / 1_000_000.0;
    let p014 = if total_area > 0.0 {
        (waste_area / total_area) * 100.0
    } else {
        0.0
    };

    writer.set_param_float(12, p012);
    writer.set_param_float(13, p013);
    writer.set_param_float(14, p014);
    writer.set_param(941, schema.n_layout_sync);

    // Calculate rest dimensions (PXRS/PYRS)
    let (pxrs, pyrs) = calculate_rest_dimensions(schema);
    writer.write_line(&format!("PXRS={}", format_coord(pxrs)));
    writer.write_line(&format!("PYRS={}", format_coord(pyrs)));
    writer.call_macro("PTMREP_B");

    // Tool selection jumps
    writer.write_raw(";parte relativa al Taglio --------");
    let linear_label = format!("01{:04}", schema_num);
    let shaped_label = format!("02{:04}", schema_num);

    // Check if this schema has shapes using the shaped tool
    let has_shaped_cuts = schema_uses_tool_type(schema, TOOL_TYPE_SHAPED as usize);

    // Linear tool jump (always present if there are linear cuts)
    if !schema.linear_cuts.is_empty() {
        writer.write_line(&format!(
            "JM((P260=2)~(P007={:04})):{}",
            DEFAULT_LINEAR_TOOL, linear_label
        ));
    }

    // Shaped tool jump (only if shapes use this tool type)
    if has_shaped_cuts {
        writer.write_line(&format!(
            "JM((P260=2)~(P007={:04})):{}",
            DEFAULT_SHAPED_TOOL, shaped_label
        ));
    }

    writer.write_line("JM(P260=2):999999999");
    writer.write_raw("");

    // Linear cuts section (only if there are linear cuts)
    if !schema.linear_cuts.is_empty() {
        writer.write_comment("parte geometrica lineare ----------");
        writer.write_label(&linear_label);
        generate_linear_cuts(writer, schema);
        writer.write_raw("");
        writer.write_line("JM(P260=2):999999999");
        writer.write_raw("");
    }

    // Shape cuts section (only if shapes use the shaped tool)
    if has_shaped_cuts {
        writer.write_comment("parte geometrica sagomata ----------");
        writer.write_label(&shaped_label);
        generate_shape_cuts(writer, schema, schema_num);
        writer.write_raw("");
        writer.write_line("JM(P260=2):999999999");
        writer.write_raw("");
    }

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

    // Get ordered piece indices using nearest-neighbor algorithm
    let ordered_indices = order_pieces_nearest_neighbor(schema, TOOL_TYPE_SHAPED as usize);

    for piece_idx in ordered_indices {
        let piece = &schema.pieces[piece_idx];
        let shape_idx = match piece.shape_index {
            Some(idx) => idx,
            None => continue,
        };

        let _shape = &schema.shapes[shape_idx];

        // Generate macro call
        // Label format: 1000000000 + toolType * 10000000 + schemaNum * 1000 + (2 * shapeIndex + 1)
        // This produces odd-numbered shape labels (001, 003, 005, 007...)
        let macro_label =
            calculate_shape_macro_label(TOOL_TYPE_SHAPED, schema_num as u32, shape_idx);

        writer.write_raw(";----");
        writer.set_work_offset();
        writer.set_xo(piece.x_origin);
        writer.set_yo(piece.y_origin);
        writer.call_label(&macro_label.to_string());
    }
}

/// Order pieces using nearest-neighbor algorithm for optimal tool path.
///
/// Algorithm:
/// 1. For each piece with a shape using the specified tool, get start/end points from first cut
/// 2. Starting from (0,0), repeatedly find the nearest unvisited piece
/// 3. After visiting a piece, move "current position" to that piece's first cut end point
///
/// Note: Uses the first cut's Xi/Yi as start and first cut's Xf/Yf as end.
fn order_pieces_nearest_neighbor(schema: &Schema, tool_type: usize) -> Vec<usize> {
    let num_pieces = schema.pieces.len();

    // Collect piece info: (start_x, start_y, end_x, end_y, has_valid_shape)
    let mut piece_info: Vec<(f64, f64, f64, f64, bool)> = Vec::with_capacity(num_pieces);

    for piece in schema.pieces.iter() {
        if let Some(shape_idx) = piece.shape_index {
            let shape = &schema.shapes[shape_idx];

            // Check if this shape uses the specified tool type
            if shape.uses_tool(tool_type) {
                // Find first active cut with this tool type
                if let Some(first_cut) = shape
                    .cuts
                    .iter()
                    .find(|c| c.active && c.tool_code == tool_type as i32)
                {
                    // Start point = piece origin + first cut start (Xi, Yi)
                    let start_x = piece.x_origin + first_cut.xi;
                    let start_y = piece.y_origin + first_cut.yi;
                    // End point = piece origin + first cut end (Xf, Yf)
                    let end_x = piece.x_origin + first_cut.xf;
                    let end_y = piece.y_origin + first_cut.yf;

                    piece_info.push((start_x, start_y, end_x, end_y, true));
                    continue;
                }
            }
        }
        // Piece doesn't have a valid shape for this tool
        piece_info.push((0.0, 0.0, 0.0, 0.0, false));
    }

    // If no optimization needed, return sequential order
    if !schema.optimize_shape_order {
        return (0..num_pieces).collect();
    }

    // Nearest-neighbor ordering
    let mut used = vec![false; num_pieces];
    let mut order = Vec::with_capacity(num_pieces);
    let mut current_x = 0.0;
    let mut current_y = 0.0;

    for _ in 0..num_pieces {
        let mut best_distance = f64::MAX;
        let mut best_idx = None;

        for (idx, info) in piece_info.iter().enumerate() {
            if !used[idx] && info.4 {
                // info.4 is has_valid_shape
                let dx = info.0 - current_x; // info.0 is start_x
                let dy = info.1 - current_y; // info.1 is start_y
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < best_distance {
                    best_distance = distance;
                    best_idx = Some(idx);
                }
            }
        }

        if let Some(idx) = best_idx {
            order.push(idx);
            used[idx] = true;
            current_x = piece_info[idx].2; // end_x
            current_y = piece_info[idx].3; // end_y
        } else {
            // No more pieces with valid shapes, break
            break;
        }
    }

    // If order is empty, return sequential order of all pieces
    if order.is_empty() {
        return (0..num_pieces).collect();
    }

    order
}

/// Calculate the shape macro label.
/// Formula: 1000000000 + toolType * 10000000 + schemaNum * 1000 + (2 * shapeIndex + 1)
fn calculate_shape_macro_label(tool_type: u32, schema_num: u32, shape_idx: usize) -> u64 {
    1_000_000_000
        + (tool_type as u64) * 10_000_000
        + (schema_num as u64) * 1000
        + (2 * shape_idx as u64 + 1)
}

/// Generate shape macro definitions.
fn generate_shape_macros(writer: &mut GcodeWriter, schemas: &[Schema]) {
    use crate::config::D_MIN_CONT;

    // Check if any schema has shapes that use the shaped tool
    let has_any_shapes = schemas
        .iter()
        .any(|s| schema_uses_tool_type(s, TOOL_TYPE_SHAPED as usize));

    if !has_any_shapes {
        return; // No shapes to generate macros for
    }

    writer.write_comment("macro delle icone ----------");

    // Default minimum angle for path continuity (in degrees)
    const DEFAULT_MIN_ANGLE: f64 = 15.0;

    for (schema_idx, schema) in schemas.iter().enumerate() {
        let schema_num = (schema_idx + 1) as u32;
        let min_angle = schema.min_angle.max(DEFAULT_MIN_ANGLE);

        for (shape_idx, shape) in schema.shapes.iter().enumerate() {
            // Skip shapes that don't use the shaped tool type
            if !shape.uses_tool(TOOL_TYPE_SHAPED as usize) {
                continue;
            }

            // Use the same label formula as in generate_shape_cuts
            let macro_label = calculate_shape_macro_label(TOOL_TYPE_SHAPED, schema_num, shape_idx);
            writer.write_label(&macro_label.to_string());

            generate_shape_macro_content(writer, shape, D_MIN_CONT, min_angle);

            writer.write_terminator();
            writer.write_raw("");
        }
    }
}

/// Generate the content of a single shape macro, handling discontinuities.
fn generate_shape_macro_content(
    writer: &mut GcodeWriter,
    shape: &crate::model::Shape,
    d_min_cont: f64,
    min_angle: f64,
) {
    use crate::config::MIN_VAL_C_POSITIVE;

    let active_cuts: Vec<_> = shape.cuts.iter().filter(|c| c.active).collect();
    if active_cuts.is_empty() {
        writer.tool_up();
        return;
    }

    let mut is_first_segment = true;
    let mut last_end_x = 0.0_f64;
    let mut last_end_y = 0.0_f64;
    let mut last_final_angle = 0.0_f64;
    let mut cut_index = 0;

    while cut_index < active_cuts.len() {
        let cut = active_cuts[cut_index];

        // Check if we need to lift and reposition
        let needs_reposition = if is_first_segment {
            true // Always position for first segment
        } else {
            let distance = ((cut.xi - last_end_x).powi(2) + (cut.yi - last_end_y).powi(2)).sqrt();
            let angle_diff = angle_min_degrees(cut.initial_angle_degrees(), last_final_angle);

            distance > d_min_cont || (distance <= d_min_cont && angle_diff >= min_angle)
        };

        if needs_reposition {
            // Close previous segment if not first
            if !is_first_segment {
                writer.tangent_mode_off();
            }

            writer.tool_up();

            // Calculate rotation angle
            let rotation = cut.initial_angle_degrees();
            let rotation = if rotation < MIN_VAL_C_POSITIVE {
                MIN_VAL_C_POSITIVE
            } else {
                rotation
            };

            writer.set_rotation_shape(rotation);
            writer.apply_rotation();

            // Rapid move to start position
            writer.write_line(&format!(
                "G00 X={} Y={} C=P540 AR=P540",
                format_coord(cut.xi),
                format_coord(cut.yi)
            ));

            // Calculate path length for this segment
            let segment_length =
                calculate_segment_length(&active_cuts, cut_index, d_min_cont, min_angle);
            writer.set_shape_params(segment_length, 1);

            writer.tool_down();
            writer.tangent_mode_on();

            is_first_segment = false;
        }

        // Output the cut movement
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

        // Track last position and angle
        last_end_x = cut.xf;
        last_end_y = cut.yf;
        last_final_angle = cut.final_angle_degrees();

        cut_index += 1;
    }

    // Close final segment
    writer.tangent_mode_off();
    writer.tool_up();
}

/// Calculate the length of a continuous segment starting at the given index.
fn calculate_segment_length(
    cuts: &[&crate::model::Cut],
    start_index: usize,
    d_min_cont: f64,
    min_angle: f64,
) -> f64 {
    let mut total_length = 0.0;

    for i in start_index..cuts.len() {
        let cut = cuts[i];
        total_length += cut.calculate_length();

        // Check if next cut is a discontinuity
        if i < cuts.len() - 1 {
            let next_cut = cuts[i + 1];
            let distance = ((next_cut.xi - cut.xf).powi(2) + (next_cut.yi - cut.yf).powi(2)).sqrt();
            let angle_diff =
                angle_min_degrees(next_cut.initial_angle_degrees(), cut.final_angle_degrees());

            if distance > d_min_cont || (distance <= d_min_cont && angle_diff >= min_angle) {
                break;
            }
        }
    }

    total_length
}

/// Calculate minimum angle difference between two angles (in degrees).
fn angle_min_degrees(angle1: f64, angle2: f64) -> f64 {
    let diff = (angle1 - angle2).abs();
    if diff > 180.0 {
        360.0 - diff
    } else {
        diff
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

        // Piece information - group by piece code, output dimensions from first matching piece
        let mut seen_codes: std::collections::HashSet<i32> = std::collections::HashSet::new();

        for piece in &schema.pieces {
            if let Some(info_id) = piece.info_id {
                // Find the piece_type for this piece
                if let Some(piece_type) = schema.piece_types.iter().find(|pt| pt.id == info_id) {
                    let code = piece_type.piece_code;

                    // Skip if we've already output this piece code, or if it's waste/invalid
                    if code <= 0 || piece_type.waste || seen_codes.contains(&code) {
                        continue;
                    }
                    seen_codes.insert(code);

                    // Shape name
                    if let Some(shape_idx) = piece.shape_index {
                        let shape_name = &schema.shapes[shape_idx].name;
                        writeln!(output, ";NomeSagoma={}", shape_name).unwrap();
                    } else {
                        writeln!(output, ";NomeSagoma=").unwrap();
                    }

                    writeln!(output, ";CodPz={}", code).unwrap();
                    // Use the PIECE's dimensions, not the piece_type's
                    writeln!(output, ";DimXPz={}", format_coord(piece.width)).unwrap();
                    writeln!(output, ";DimYPz={}", format_coord(piece.height)).unwrap();

                    // Count pieces of this type
                    let count = schema
                        .pieces
                        .iter()
                        .filter(|p| {
                            p.info_id
                                .map(|id| {
                                    schema
                                        .piece_types
                                        .iter()
                                        .find(|pt| pt.id == id)
                                        .map(|pt| pt.piece_code == code)
                                        .unwrap_or(false)
                                })
                                .unwrap_or(false)
                        })
                        .count();
                    writeln!(output, ";QtaPz={}", count).unwrap();

                    writeln!(output, ";ClientePz={}", piece_type.customer).unwrap();
                    writeln!(output, ";OrdinePz={}", piece_type.order_no).unwrap();
                }
            }
        }

        writeln!(output, "%").unwrap();
    }
    // Extra blank line after last LDIST section
    writeln!(output).unwrap();
}

/// Calculate total area of all pieces in the schema.
fn calculate_piece_area(schema: &Schema) -> f64 {
    let mut area = 0.0;

    for piece in &schema.pieces {
        // Only count pieces with valid info (non-scrap pieces)
        if piece.info_id.is_some() {
            // Check if the associated piece_type is not scrap
            if let Some(info_id) = piece.info_id {
                let is_waste = schema
                    .piece_types
                    .iter()
                    .find(|pt| pt.id == info_id)
                    .map(|pt| pt.waste)
                    .unwrap_or(false);

                if !is_waste {
                    area += piece.width * piece.height;
                }
            }
        }
    }

    area
}

/// Calculate rest dimensions (PXRS, PYRS) for the schema.
///
/// This calculates the dimensions of the largest usable rest area.
fn calculate_rest_dimensions(schema: &Schema) -> (f64, f64) {
    // Use EPS_COARSE for coarser comparisons in rest dimension calculations
    let anticipo = schema.linear_advance; // anticipoLineareCopia

    if schema.pieces.is_empty() {
        return (
            schema.width - schema.trim_left,
            schema.height - schema.trim_bottom,
        );
    }

    // Find the extent of all pieces
    let mut trim_left = schema.trim_left;
    let mut min_y = schema.height; // Minimum piece Y origin (num6)
    let mut max_x = 0.0_f64; // Maximum piece right edge (num7)
    let mut max_y = 0.0_f64; // Maximum piece top edge (num9)

    for piece in &schema.pieces {
        if piece.x_origin < trim_left {
            trim_left = piece.x_origin;
        }
        if piece.y_origin < min_y {
            min_y = piece.y_origin;
        }
        let piece_right = piece.x_origin + piece.width;
        let piece_top = piece.y_origin + piece.height;
        if piece_right > max_x {
            max_x = piece_right;
        }
        if piece_top > max_y {
            max_y = piece_top;
        }
    }

    // Determine if rest is on right (flag=true) or top (flag=false)
    let mut flag = false;
    for cut in &schema.linear_cuts {
        let cut_len = cut.calculate_length();
        // Vertical cut at max_x that spans from min_y to top
        // Use EPS_COARSE tolerance for the >= comparison to handle floating point precision
        if (cut.xi - max_x).abs() < EPS_COARSE
            && cut_len + 2.0 * anticipo + min_y >= schema.height - EPS_COARSE
        {
            flag = true;
        }
        // Horizontal cut at max_y that spans from trim_left to right
        if (cut.yi - max_y).abs() < EPS_COARSE
            && cut_len + 2.0 * anticipo + trim_left >= schema.width - EPS_COARSE
        {
            flag = false;
        }
    }

    if flag {
        // Rest is on the right side
        let dim_x = schema.width - max_x;
        if dim_x > 0.0 {
            let dim_y = schema.height - min_y;

            // Find secondary rest area (between vertical cuts)
            let mut max_cut_x = 0.0_f64;
            for cut in &schema.linear_cuts {
                let cut_len = cut.calculate_length();
                // Vertical cuts that span full height
                if (cut.xi - cut.xf).abs() < EPS_COARSE
                    && cut_len + min_y + 2.0 * anticipo > schema.height
                    && cut.xi < max_x
                    && cut.xi > max_cut_x
                {
                    max_cut_x = cut.xi;
                }
            }

            // Find max piece top in the secondary area
            let mut max_piece_top = 0.0_f64;
            for piece in &schema.pieces {
                if piece.x_origin >= max_cut_x
                    && piece.x_origin + piece.width <= max_x
                    && piece.y_origin + piece.height > max_piece_top
                {
                    max_piece_top = piece.y_origin + piece.height;
                }
            }

            let secondary_dim_x = max_x - max_cut_x;
            let secondary_dim_y = schema.height - max_piece_top;
            let area1 = dim_x * dim_y;
            let area2 = secondary_dim_x * secondary_dim_y;

            if area1 >= area2 {
                (dim_x, dim_y)
            } else {
                (secondary_dim_x, secondary_dim_y)
            }
        } else {
            (0.0, 0.0)
        }
    } else {
        // Rest is on top
        let dim_y = schema.height - max_y;
        let dim_x = schema.width - trim_left;
        (dim_x, dim_y)
    }
}
