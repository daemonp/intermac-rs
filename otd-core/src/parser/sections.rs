//! Section-specific parsers for OTD format.

use crate::config::Unit;
use crate::model::{Cut, CutType, Piece, PieceType, Shape};

/// Parse a key=value pair from a line.
pub fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let eq_pos = line.find('=')?;
    let key = line[..eq_pos].trim();
    let value = line[eq_pos + 1..].trim();
    Some((key, value))
}

/// Parse a string value.
pub fn parse_string_value(line: &str, key: &str) -> Option<String> {
    let (k, v) = parse_key_value(line)?;
    if k == key {
        Some(v.to_string())
    } else {
        None
    }
}

/// Parse an integer value.
pub fn parse_int_value(line: &str, key: &str) -> Option<i32> {
    let (k, v) = parse_key_value(line)?;
    if k == key {
        v.parse().ok()
    } else {
        None
    }
}

/// Parse a float value.
pub fn parse_float_value(line: &str, key: &str) -> Option<f64> {
    let (k, v) = parse_key_value(line)?;
    if k == key {
        v.parse().ok()
    } else {
        None
    }
}

/// Parse multiple key=value pairs from a single line (space-separated).
pub fn parse_multi_values(line: &str) -> Vec<(&str, &str)> {
    let mut result = Vec::new();
    let mut remaining = line.trim();

    while !remaining.is_empty() {
        // Find the next key=value pair
        if let Some(eq_pos) = remaining.find('=') {
            let key = remaining[..eq_pos].trim();
            let after_eq = &remaining[eq_pos + 1..];

            // Find where the value ends (next key or end of string)
            // A key starts with a letter and is followed by =
            let value_end = after_eq
                .char_indices()
                .skip(1) // Skip at least one char for the value
                .find(|(i, _)| {
                    // Look ahead to see if this could be a key=
                    let rest = &after_eq[*i..];
                    rest.starts_with(char::is_alphabetic)
                        && rest
                            .find('=')
                            .is_some_and(|eq| rest[..eq].chars().all(|c| c.is_alphanumeric()))
                })
                .map(|(i, _)| i)
                .unwrap_or(after_eq.len());

            let value = after_eq[..value_end].trim();
            result.push((key, value));

            remaining = after_eq[value_end..].trim();
        } else {
            break;
        }
    }

    result
}

/// Header section data.
#[derive(Debug, Default)]
pub struct HeaderData {
    pub otd_version: String,
    pub unit: Unit,
    pub date: String,
}

/// Parse [Header] section.
pub fn parse_header(lines: &[&str]) -> HeaderData {
    let mut data = HeaderData::default();

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        if let Some(v) = parse_string_value(line, "OTDCutVersion") {
            data.otd_version = v;
        } else if let Some(v) = parse_string_value(line, "AWCutVersion") {
            data.otd_version = v;
        } else if let Some(v) = parse_string_value(line, "Dimension") {
            data.unit = Unit::from_dimension_str(&v).unwrap_or_default();
        } else if let Some(v) = parse_string_value(line, "Date") {
            data.date = v;
        }
    }

    data
}

/// Signature section data.
#[derive(Debug, Default)]
pub struct SignatureData {
    pub creator: String,
}

/// Parse [Signature] section.
pub fn parse_signature(lines: &[&str]) -> SignatureData {
    let mut data = SignatureData::default();

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        if let Some(v) = parse_string_value(line, "Creator") {
            data.creator = v;
        }
    }

    data
}

/// Pattern section data (before parsing nested coordinates).
#[derive(Debug, Default)]
pub struct PatternData {
    pub machine_name: String,
    pub machine_number: u16,
    pub glass_id: String,
    pub glass_description: String,
    pub thickness: f64,
    pub glass_structured: bool,
    pub glass_coated: bool,
    pub width: f64,
    pub height: f64,
    pub trim_left: f64,
    pub trim_bottom: f64,
    pub quantity: u32,
    pub cutting_order: u8,
    pub linear_advance: f64,
    pub min_angle: f64,
    pub coating_min_angle: f64,
    pub linear_tool: i32,
    pub shaped_tool: i32,
    pub open_shaped_tool: i32,
    pub incision_tool: i32,
    pub optimize_shape_order: bool,
}

/// Parse [Pattern] section header fields (not the nested coordinates).
pub fn parse_pattern_header(lines: &[&str]) -> PatternData {
    let mut data = PatternData {
        quantity: 1,
        min_angle: 5.0,
        coating_min_angle: 5.0,
        optimize_shape_order: true,
        ..Default::default()
    };

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        // Stop when we hit nested coordinates
        if line.starts_with("X=") || line.starts_with("Y=") || line.starts_with("Z=") {
            break;
        }

        if let Some(v) = parse_string_value(line, "MachineName") {
            data.machine_name = v;
        } else if let Some(v) = parse_int_value(line, "MachineNumber") {
            data.machine_number = v as u16;
        } else if let Some(v) = parse_string_value(line, "GlassID") {
            data.glass_id = v;
        } else if let Some(v) = parse_string_value(line, "GlassDescription") {
            data.glass_description = v;
        } else if let Some(v) = parse_float_value(line, "GlassThickness") {
            data.thickness = v;
        } else if let Some(v) = parse_int_value(line, "GlassStructured") {
            data.glass_structured = v == 1;
        } else if let Some(v) = parse_int_value(line, "GlassCoated") {
            data.glass_coated = v == 1;
        } else if let Some(v) = parse_float_value(line, "Width") {
            data.width = v;
        } else if let Some(v) = parse_float_value(line, "Height") {
            data.height = v;
        } else if let Some(v) = parse_float_value(line, "TrimLeft") {
            data.trim_left = v;
        } else if let Some(v) = parse_float_value(line, "TrimBottom") {
            data.trim_bottom = v;
        } else if let Some(v) = parse_int_value(line, "Pieces") {
            data.quantity = v.max(1) as u32;
        } else if let Some(v) = parse_int_value(line, "CuttingOrder") {
            data.cutting_order = v as u8;
        } else if let Some(v) = parse_float_value(line, "LinearAdvance") {
            data.linear_advance = v;
        } else if let Some(v) = parse_float_value(line, "MinAngle") {
            data.min_angle = v;
        } else if let Some(v) = parse_float_value(line, "CoatingMinAngle") {
            data.coating_min_angle = v;
        } else if let Some(v) = parse_int_value(line, "LinearToolCode") {
            data.linear_tool = v;
        } else if let Some(v) = parse_int_value(line, "ToolCode1") {
            data.shaped_tool = v;
        } else if let Some(v) = parse_int_value(line, "ToolCode2") {
            data.incision_tool = v;
        } else if let Some(v) = parse_int_value(line, "ToolCode6") {
            data.open_shaped_tool = v;
        } else if let Some(v) = parse_int_value(line, "ShapeOptimization") {
            data.optimize_shape_order = v == 1;
        }
    }

    data
}

/// Nested coordinate entry from Pattern section.
#[derive(Debug, Clone)]
pub struct CoordEntry {
    /// Coordinate variable (X, Y, Z, W, V, A, B, C, D, E).
    pub var: char,
    /// Level (0=X, 1=Y, 2=Z, etc.).
    pub level: i32,
    /// Coordinate value.
    pub value: f64,
    /// Shape reference if present.
    pub shape_id: Option<i32>,
    /// Info reference if present.
    pub info_id: Option<i32>,
    /// Rotation if present.
    pub rotation: Option<f64>,
    /// Tcut if present.
    pub tcut: Option<i32>,
}

/// Parse nested coordinate lines from Pattern section.
pub fn parse_pattern_coordinates(lines: &[&str]) -> Vec<CoordEntry> {
    let coord_vars = ['X', 'Y', 'Z', 'W', 'V', 'A', 'B', 'C', 'D', 'E'];
    let mut entries = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        // Check if this is a coordinate line
        let first_char = line.chars().next().unwrap_or(' ');
        if !coord_vars.contains(&first_char) {
            continue;
        }

        // Parse the coordinate value
        let values = parse_multi_values(line);
        if values.is_empty() {
            continue;
        }

        let (var_key, var_value) = values[0];
        if var_key.len() != 1 {
            continue;
        }

        let var = var_key.chars().next().unwrap();
        let level = coord_vars.iter().position(|&c| c == var).unwrap_or(0) as i32;

        let value: f64 = match var_value.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let mut entry = CoordEntry {
            var,
            level,
            value,
            shape_id: None,
            info_id: None,
            rotation: None,
            tcut: None,
        };

        // Parse additional fields
        for (key, val) in &values[1..] {
            match *key {
                "Shape" => entry.shape_id = val.parse().ok(),
                "Info" => entry.info_id = val.parse().ok(),
                "Rot" => entry.rotation = val.parse().ok(),
                "Tcut" => entry.tcut = val.parse().ok(),
                _ => {}
            }
        }

        entries.push(entry);
    }

    entries
}

/// Parse [Info] section into PieceType.
pub fn parse_info(lines: &[&str]) -> Option<PieceType> {
    let mut pt = PieceType::default();
    let mut has_id = false;

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        if let Some(v) = parse_int_value(line, "Id") {
            pt.id = v;
            has_id = true;
        } else if let Some(v) = parse_string_value(line, "OrderNo") {
            pt.order_no = v;
        } else if let Some(v) = parse_string_value(line, "PosNo") {
            pt.position_no = v;
        } else if let Some(v) = parse_string_value(line, "Customer") {
            pt.customer = v;
        } else if let Some(v) = parse_string_value(line, "Commission") {
            pt.commission = v;
        } else if let Some(v) = parse_string_value(line, "SecondGlassReference") {
            pt.second_glass_ref = v;
        } else if let Some(v) = parse_string_value(line, "RackNo") {
            pt.rack_no = v;
        } else if let Some(v) = parse_float_value(line, "SheetWidth") {
            pt.sheet_width = v;
        } else if let Some(v) = parse_float_value(line, "SheetHeight") {
            pt.sheet_height = v;
        } else if let Some(v) = parse_int_value(line, "SheetCode") {
            pt.piece_code = v;
        } else if let Some(v) = parse_int_value(line, "Waste") {
            pt.waste = v == 1;
        }
    }

    if has_id {
        Some(pt)
    } else {
        None
    }
}

/// Parse [Shape] section into Shape.
pub fn parse_shape(lines: &[&str]) -> Option<Shape> {
    let mut shape = Shape::new(0); // Use new() to get proper tool_types initialization
    let mut has_id = false;

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        // Check for Id, Name, Description
        if let Some(v) = parse_int_value(line, "Id") {
            shape.id = v;
            has_id = true;
            continue;
        }
        if let Some(v) = parse_string_value(line, "Name") {
            shape.name = v;
            continue;
        }
        if let Some(v) = parse_string_value(line, "Description") {
            shape.description = v;
            continue;
        }

        // Check for geometry line (starts with x= or X=)
        if line.starts_with("x=") || line.starts_with("X=") {
            if let Some(cut) = parse_geometry_line(line) {
                shape.cuts.push(cut);
            }
        }
    }

    if has_id {
        shape.calculate_perimeter();
        shape.is_open = !shape.is_closed();
        Some(shape)
    } else {
        None
    }
}

/// Parse a geometry line (x=... y=... X=... Y=... [R=...] [L=...] [C=...]).
pub fn parse_geometry_line(line: &str) -> Option<Cut> {
    let values = parse_multi_values(line);
    if values.is_empty() {
        return None;
    }

    let mut xi = None;
    let mut yi = None;
    let mut xf = None;
    let mut yf = None;
    let mut radius_cw = None;
    let mut radius_ccw = None;
    let mut tool_code = 1;
    let mut ablation_width = 0.0;

    for (key, val) in &values {
        match *key {
            "x" => xi = val.parse().ok(),
            "y" => yi = val.parse().ok(),
            "X" => xf = val.parse().ok(),
            "Y" => yf = val.parse().ok(),
            "R" => radius_cw = val.parse().ok(),
            "L" => radius_ccw = val.parse().ok(),
            "C" => tool_code = val.parse().unwrap_or(1),
            "LA" => ablation_width = val.parse().unwrap_or(0.0),
            _ => {}
        }
    }

    let xi = xi?;
    let yi = yi?;
    let xf = xf?;
    let yf = yf?;

    let mut cut = if let Some(r) = radius_cw {
        Cut::new_arc_cw(xi, yi, xf, yf, r)
    } else if let Some(r) = radius_ccw {
        Cut::new_arc_ccw(xi, yi, xf, yf, r)
    } else {
        Cut::new_line(xi, yi, xf, yf)
    };

    cut.tool_code = tool_code;
    cut.ablation_width = ablation_width;

    Some(cut)
}

/// Parse [Cuttings] section.
pub fn parse_cuttings(lines: &[&str]) -> (Vec<Cut>, Vec<Piece>) {
    let mut cuts = Vec::new();
    let mut pieces = Vec::new();
    let mut current_cut_index: i32 = -1;

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        let values = parse_multi_values(line);
        if values.is_empty() {
            continue;
        }

        let first_key = values[0].0;

        // Check if this is a piece line (starts with XO=)
        if first_key == "XO" {
            let mut piece = Piece::default();
            for (key, val) in &values {
                match *key {
                    "XO" => piece.x_origin = val.parse().unwrap_or(0.0),
                    "YO" => piece.y_origin = val.parse().unwrap_or(0.0),
                    "Width" => piece.width = val.parse().unwrap_or(0.0),
                    "Height" => piece.height = val.parse().unwrap_or(0.0),
                    "Info" => piece.info_id = val.parse().ok(),
                    "Shape" => piece.shape_id = val.parse().ok(),
                    "IndPiece" => piece.ind_piece = val.parse().ok(),
                    _ => {}
                }
            }
            if piece.width > 0.0 && piece.height > 0.0 {
                pieces.push(piece);
            }
            continue;
        }

        // Check if this is an IndPiece reference line
        if first_key == "IndPiece" {
            if current_cut_index >= 0 && (current_cut_index as usize) < cuts.len() {
                let cut: &mut Cut = &mut cuts[current_cut_index as usize];
                for (key, val) in &values {
                    match *key {
                        "IndPiece" => {
                            if let Ok(idx) = val.parse::<i32>() {
                                cut.piece_indices.push(idx);
                            }
                        }
                        "Cut" => {
                            if let Ok(idx) = val.parse::<i32>() {
                                cut.cut_indices.push(idx);
                            }
                        }
                        _ => {}
                    }
                }
            }
            continue;
        }

        // Check if this is a cut line (starts with x= or y=)
        if first_key == "x" || first_key == "y" {
            let mut cut = Cut {
                active: true,
                ..Default::default()
            };

            for (key, val) in &values {
                match *key {
                    "x" => cut.xi = val.parse().unwrap_or(0.0),
                    "y" => cut.yi = val.parse().unwrap_or(0.0),
                    "X" => cut.xf = val.parse().unwrap_or(0.0),
                    "Y" => cut.yf = val.parse().unwrap_or(0.0),
                    "Levcut" => cut.level = val.parse().unwrap_or(0),
                    "Rot" => cut.rotation = val.parse().unwrap_or(0.0),
                    "Qcut" => cut.quota = val.parse().unwrap_or(0.0),
                    "Lcut" => cut.length = val.parse().unwrap_or(0.0),
                    "Tcut" => cut.tcut = val.parse().unwrap_or(0),
                    "Rcut" => cut.rest = val.parse().unwrap_or(-1.0),
                    "Wcut" => {
                        if let Ok(w) = val.parse::<i32>() {
                            cut.is_scrap = w > 0;
                        }
                    }
                    "ParentShape" => cut.parent_shape = val.parse().unwrap_or(-1),
                    _ => {}
                }
            }

            cut.cut_type = CutType::Line;
            cut.determine_line_type();
            cuts.push(cut);
            current_cut_index = cuts.len() as i32 - 1;
        }
    }

    (cuts, pieces)
}

/// Parse [LowE] section (same format as Cuttings).
pub fn parse_lowe(lines: &[&str]) -> (Vec<Cut>, Vec<Piece>) {
    parse_cuttings(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== parse_key_value tests ====================

    #[test]
    fn test_parse_key_value_simple() {
        let result = parse_key_value("Key=Value");
        assert_eq!(result, Some(("Key", "Value")));
    }

    #[test]
    fn test_parse_key_value_with_spaces() {
        let result = parse_key_value("  Key  =  Value  ");
        assert_eq!(result, Some(("Key", "Value")));
    }

    #[test]
    fn test_parse_key_value_empty_value() {
        let result = parse_key_value("Key=");
        assert_eq!(result, Some(("Key", "")));
    }

    #[test]
    fn test_parse_key_value_no_equals() {
        let result = parse_key_value("NoEquals");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_key_value_numeric() {
        let result = parse_key_value("Width=123.456");
        assert_eq!(result, Some(("Width", "123.456")));
    }

    // ==================== parse_multi_values tests ====================

    #[test]
    fn test_parse_multi_values_single() {
        let result = parse_multi_values("x=10.5");
        assert_eq!(result, vec![("x", "10.5")]);
    }

    #[test]
    fn test_parse_multi_values_multiple() {
        let result = parse_multi_values("x=10 y=20 X=30 Y=40");
        assert_eq!(
            result,
            vec![("x", "10"), ("y", "20"), ("X", "30"), ("Y", "40")]
        );
    }

    #[test]
    fn test_parse_multi_values_geometry_line() {
        let result = parse_multi_values("x=0 y=0 X=100 Y=0 R=50");
        assert_eq!(result.len(), 5);
        assert_eq!(result[4], ("R", "50"));
    }

    #[test]
    fn test_parse_multi_values_empty() {
        let result = parse_multi_values("");
        assert!(result.is_empty());
    }

    // ==================== parse_header tests ====================

    #[test]
    fn test_parse_header_complete() {
        let lines = vec!["OTDCutVersion=3.0", "Dimension=mm", "Date=2024-01-15"];
        let header = parse_header(&lines);
        assert_eq!(header.otd_version, "3.0");
        assert_eq!(header.unit, Unit::Millimeters);
        assert_eq!(header.date, "2024-01-15");
    }

    #[test]
    fn test_parse_header_with_comments() {
        let lines = vec![
            "; This is a comment",
            "OTDCutVersion=2.5",
            "",
            "Dimension=inch",
        ];
        let header = parse_header(&lines);
        assert_eq!(header.otd_version, "2.5");
        assert_eq!(header.unit, Unit::Inches);
    }

    #[test]
    fn test_parse_header_awcut_version() {
        let lines = vec!["AWCutVersion=1.0"];
        let header = parse_header(&lines);
        assert_eq!(header.otd_version, "1.0");
    }

    // ==================== parse_pattern_header tests ====================

    #[test]
    fn test_parse_pattern_header_basic() {
        let lines = vec![
            "MachineName=CNCMACHINE",
            "MachineNumber=130",
            "Width=3000",
            "Height=2000",
            "GlassThickness=6.5",
        ];
        let pattern = parse_pattern_header(&lines);
        assert_eq!(pattern.machine_name, "CNCMACHINE");
        assert_eq!(pattern.machine_number, 130);
        assert!((pattern.width - 3000.0).abs() < 0.001);
        assert!((pattern.height - 2000.0).abs() < 0.001);
        assert!((pattern.thickness - 6.5).abs() < 0.001);
    }

    #[test]
    fn test_parse_pattern_header_stops_at_coordinates() {
        let lines = vec![
            "Width=3000",
            "X=100",       // Should stop here
            "Height=2000", // Should not be parsed
        ];
        let pattern = parse_pattern_header(&lines);
        assert!((pattern.width - 3000.0).abs() < 0.001);
        assert!((pattern.height - 0.0).abs() < 0.001); // Default value
    }

    #[test]
    fn test_parse_pattern_header_defaults() {
        let lines: Vec<&str> = vec![];
        let pattern = parse_pattern_header(&lines);
        assert_eq!(pattern.quantity, 1);
        assert!((pattern.min_angle - 5.0).abs() < 0.001);
        assert!(pattern.optimize_shape_order);
    }

    // ==================== parse_info tests ====================

    #[test]
    fn test_parse_info_complete() {
        let lines = vec![
            "Id=1",
            "OrderNo=ORD-12345",
            "Customer=ACME Corp",
            "SheetWidth=500",
            "SheetHeight=400",
            "SheetCode=42",
        ];
        let info = parse_info(&lines).expect("Should parse info");
        assert_eq!(info.id, 1);
        assert_eq!(info.order_no, "ORD-12345");
        assert_eq!(info.customer, "ACME Corp");
        assert!((info.sheet_width - 500.0).abs() < 0.001);
        assert!((info.sheet_height - 400.0).abs() < 0.001);
        assert_eq!(info.piece_code, 42);
    }

    #[test]
    fn test_parse_info_no_id() {
        let lines = vec!["OrderNo=ORD-12345", "Customer=ACME Corp"];
        let info = parse_info(&lines);
        assert!(info.is_none());
    }

    #[test]
    fn test_parse_info_waste_flag() {
        let lines = vec!["Id=1", "Waste=1"];
        let info = parse_info(&lines).expect("Should parse info");
        assert!(info.waste);
    }

    // ==================== parse_shape tests ====================

    #[test]
    fn test_parse_shape_basic() {
        let lines = vec!["Id=1", "Name=Rectangle", "Description=A simple rectangle"];
        let shape = parse_shape(&lines).expect("Should parse shape");
        assert_eq!(shape.id, 1);
        assert_eq!(shape.name, "Rectangle");
        assert_eq!(shape.description, "A simple rectangle");
    }

    #[test]
    fn test_parse_shape_with_geometry() {
        let lines = vec![
            "Id=1",
            "Name=Triangle",
            "x=0 y=0 X=100 Y=0",
            "x=100 y=0 X=50 Y=86.6",
            "x=50 y=86.6 X=0 Y=0",
        ];
        let shape = parse_shape(&lines).expect("Should parse shape");
        assert_eq!(shape.cuts.len(), 3);
    }

    #[test]
    fn test_parse_shape_no_id() {
        let lines = vec!["Name=NoID", "x=0 y=0 X=100 Y=0"];
        let shape = parse_shape(&lines);
        assert!(shape.is_none());
    }

    // ==================== parse_geometry_line tests ====================

    #[test]
    fn test_parse_geometry_line_simple_line() {
        let cut = parse_geometry_line("x=0 y=0 X=100 Y=50").expect("Should parse");
        assert!((cut.xi - 0.0).abs() < 0.001);
        assert!((cut.yi - 0.0).abs() < 0.001);
        assert!((cut.xf - 100.0).abs() < 0.001);
        assert!((cut.yf - 50.0).abs() < 0.001);
        assert_eq!(cut.cut_type, CutType::Line);
    }

    #[test]
    fn test_parse_geometry_line_arc_cw() {
        let cut = parse_geometry_line("x=0 y=0 X=100 Y=0 R=50").expect("Should parse");
        assert_eq!(cut.cut_type, CutType::ArcCW);
        assert!((cut.radius - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_geometry_line_arc_ccw() {
        let cut = parse_geometry_line("x=0 y=0 X=100 Y=0 L=50").expect("Should parse");
        assert_eq!(cut.cut_type, CutType::ArcCCW);
        assert!((cut.radius - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_geometry_line_with_tool() {
        let cut = parse_geometry_line("x=0 y=0 X=100 Y=0 C=3").expect("Should parse");
        assert_eq!(cut.tool_code, 3);
    }

    #[test]
    fn test_parse_geometry_line_with_ablation() {
        let cut = parse_geometry_line("x=0 y=0 X=100 Y=0 LA=5.5").expect("Should parse");
        assert!((cut.ablation_width - 5.5).abs() < 0.001);
    }

    #[test]
    fn test_parse_geometry_line_missing_coords() {
        let cut = parse_geometry_line("x=0 y=0 X=100");
        assert!(cut.is_none());
    }

    // ==================== parse_cuttings tests ====================

    #[test]
    fn test_parse_cuttings_basic() {
        let lines = vec![
            "x=0 y=100 X=500 Y=100 Levcut=0",
            "x=500 y=0 X=500 Y=200 Levcut=1",
        ];
        let (cuts, pieces) = parse_cuttings(&lines);
        assert_eq!(cuts.len(), 2);
        assert!(pieces.is_empty());
        assert_eq!(cuts[0].level, 0);
        assert_eq!(cuts[1].level, 1);
    }

    #[test]
    fn test_parse_cuttings_with_pieces() {
        let lines = vec![
            "XO=0 YO=0 Width=100 Height=200 Info=1",
            "XO=100 YO=0 Width=150 Height=200 Info=2 Shape=1",
        ];
        let (cuts, pieces) = parse_cuttings(&lines);
        assert!(cuts.is_empty());
        assert_eq!(pieces.len(), 2);
        assert!((pieces[0].width - 100.0).abs() < 0.001);
        assert_eq!(pieces[0].info_id, Some(1));
        assert_eq!(pieces[1].shape_id, Some(1));
    }

    #[test]
    fn test_parse_cuttings_mixed() {
        let lines = vec![
            "; Comment line",
            "x=0 y=100 X=500 Y=100 Levcut=0 Tcut=1",
            "",
            "XO=0 YO=0 Width=100 Height=100",
        ];
        let (cuts, pieces) = parse_cuttings(&lines);
        assert_eq!(cuts.len(), 1);
        assert_eq!(pieces.len(), 1);
        assert_eq!(cuts[0].tcut, 1);
    }

    #[test]
    fn test_parse_cuttings_with_ind_piece() {
        let lines = vec![
            "x=0 y=100 X=500 Y=100 Levcut=0",
            "IndPiece=0 Cut=1",
            "IndPiece=1 Cut=2",
        ];
        let (cuts, _pieces) = parse_cuttings(&lines);
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].piece_indices, vec![0, 1]);
        assert_eq!(cuts[0].cut_indices, vec![1, 2]);
    }

    // ==================== parse_pattern_coordinates tests ====================

    #[test]
    fn test_parse_pattern_coordinates_basic() {
        let lines = vec!["X=100", "Y=200", "Z=300"];
        let coords = parse_pattern_coordinates(&lines);
        assert_eq!(coords.len(), 3);
        assert_eq!(coords[0].var, 'X');
        assert!((coords[0].value - 100.0).abs() < 0.001);
        assert_eq!(coords[0].level, 0);
        assert_eq!(coords[1].var, 'Y');
        assert_eq!(coords[1].level, 1);
        assert_eq!(coords[2].var, 'Z');
        assert_eq!(coords[2].level, 2);
    }

    #[test]
    fn test_parse_pattern_coordinates_with_shape() {
        let lines = vec!["X=100 Shape=1 Info=2"];
        let coords = parse_pattern_coordinates(&lines);
        assert_eq!(coords.len(), 1);
        assert_eq!(coords[0].shape_id, Some(1));
        assert_eq!(coords[0].info_id, Some(2));
    }

    #[test]
    fn test_parse_pattern_coordinates_with_rotation() {
        let lines = vec!["X=100 Rot=90"];
        let coords = parse_pattern_coordinates(&lines);
        assert_eq!(coords.len(), 1);
        assert_eq!(coords[0].rotation, Some(90.0));
    }
}
