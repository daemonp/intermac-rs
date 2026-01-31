//! DXF (Drawing Exchange Format) generator for piece visualization.
//!
//! Generates PRWB (bottom view) and PRWC (C-side/mirrored view) sections
//! containing DXF drawings of the cutting patterns.

use crate::config::angle::normalize_degrees;
use crate::model::{CutType, Piece, Schema, Shape};
use std::fmt::Write;

/// View mode for DXF generation.
/// Controls coordinate transformation for normal vs mirrored views.
#[derive(Debug, Clone, Copy)]
pub enum ViewMode {
    /// Normal view (PRWB) - no coordinate transformation.
    Normal,
    /// Mirrored view (PRWC) - X coordinates are mirrored around sheet center.
    Mirrored { sheet_width: f64 },
}

impl ViewMode {
    /// Transform an X coordinate according to the view mode.
    #[inline]
    fn transform_x(&self, x: f64) -> f64 {
        match self {
            ViewMode::Normal => x,
            ViewMode::Mirrored { sheet_width } => sheet_width - x,
        }
    }

    /// Check if this is a mirrored view.
    #[inline]
    fn is_mirrored(&self) -> bool {
        matches!(self, ViewMode::Mirrored { .. })
    }
}

/// DXF layer colors (AutoCAD color indices)
pub struct DxfColors {
    pub exterior: i32,   // Sheet boundary
    pub cuts: i32,       // Linear cuts
    pub shape_cuts: i32, // Shape cuts
    pub piece_type: i32, // Piece type text
    pub customer: i32,   // Customer text
    pub order: i32,      // Order number text
    pub dimensions: i32, // Dimension text
    pub shape_name: i32, // Shape name text
    pub piece_fill: i32, // Piece fill color
    pub scrap_fill: i32, // Scrap fill color
}

impl Default for DxfColors {
    fn default() -> Self {
        // Standard DXF color indices for visualization
        Self {
            exterior: 5,     // Blue
            cuts: 140,       // Light red/pink
            shape_cuts: 6,   // Magenta
            piece_type: 5,   // Blue
            customer: 5,     // Blue
            order: 5,        // Blue
            dimensions: 5,   // Blue
            shape_name: 5,   // Blue
            piece_fill: 131, // Light green
            scrap_fill: 254, // Gray
        }
    }
}

/// DXF writer for generating AutoCAD-compatible drawings.
pub struct DxfWriter {
    output: String,
    #[allow(dead_code)]
    colors: DxfColors,
}

impl DxfWriter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            colors: DxfColors::default(),
        }
    }

    /// Get the generated DXF content.
    pub fn into_string(self) -> String {
        self.output
    }

    /// Write a raw line.
    #[allow(dead_code)]
    fn write_line(&mut self, line: &str) {
        writeln!(self.output, "{}", line).unwrap();
    }

    /// Write a DXF group code and value (with spacing for header/tables).
    fn write_group(&mut self, code: i32, value: &str) {
        // DXF group codes are right-aligned:
        // Single digit codes: 2 spaces prefix (e.g., "  0", "  1")
        // Double digit codes: 1 space prefix (e.g., " 10", " 20")
        // Triple digit codes: no prefix (e.g., "100")
        if code < 10 {
            writeln!(self.output, "  {}", code).unwrap();
        } else if code < 100 {
            writeln!(self.output, " {}", code).unwrap();
        } else {
            writeln!(self.output, "{}", code).unwrap();
        }
        writeln!(self.output, "{}", value).unwrap();
    }

    /// Write a DXF group code and value (without spacing for entities).
    fn write_entity_group(&mut self, code: i32, value: &str) {
        writeln!(self.output, "{}", code).unwrap();
        writeln!(self.output, "{}", value).unwrap();
    }

    /// Write a DXF group code with integer value (right-aligned in 6 chars).
    fn write_group_int(&mut self, code: i32, value: i32) {
        if code < 10 {
            writeln!(self.output, "  {}", code).unwrap();
        } else if code < 100 {
            writeln!(self.output, " {}", code).unwrap();
        } else {
            writeln!(self.output, "{}", code).unwrap();
        }
        writeln!(self.output, "{:>6}", value).unwrap();
    }

    /// Write a coordinate value with 3 decimal places.
    /// Uses "round half away from zero" rounding.
    fn format_coord(value: f64) -> String {
        // Multiply by 1000, round away from zero, then divide by 1000
        let scaled = value * 1000.0;
        let rounded = if scaled >= 0.0 {
            (scaled + 0.5).floor()
        } else {
            (scaled - 0.5).ceil()
        };
        format!("{:.3}", rounded / 1000.0)
    }

    /// Write the DXF header section.
    pub fn write_header(&mut self) {
        self.write_group(0, "SECTION");
        self.write_group(2, "HEADER");

        // AutoCAD version
        self.write_group(9, "$ACADVER");
        self.write_group(1, "AC1009");

        // UCS settings
        self.write_group(9, "$UCSNAME");
        // Note: "2" written without leading space for DXF compatibility
        writeln!(self.output, "2").unwrap();
        writeln!(self.output).unwrap();

        self.write_group(9, "$UCSORG");
        self.write_group(10, "0.0");
        self.write_group(20, "0.0");
        self.write_group(30, "0.0");

        self.write_group(9, "$UCSXDIR");
        self.write_group(10, "1.0");
        self.write_group(20, "0.0");
        self.write_group(30, "0.0");

        self.write_group(9, "$UCSYDIR");
        self.write_group(10, "0.0");
        self.write_group(20, "-1.0");
        self.write_group(30, "0.0");

        // PUCS settings
        self.write_group(9, "$PUCSNAME");
        self.write_group(2, "");

        self.write_group(9, "$PUCSORG");
        self.write_group(10, "0.0");
        self.write_group(20, "0.0");
        self.write_group(30, "0.0");

        self.write_group(9, "$PUCSXDIR");
        self.write_group(10, "1.0");
        self.write_group(20, "0.0");
        self.write_group(30, "0.0");

        self.write_group(9, "$PUCSYDIR");
        self.write_group(10, "0.0");
        self.write_group(20, "1.0");
        self.write_group(30, "0.0");

        self.write_group(0, "ENDSEC");
    }

    /// Write the tables section with layer definitions.
    pub fn write_tables(&mut self, layers: &[(&str, i32)]) {
        self.write_group(0, "SECTION");
        self.write_group(2, "TABLES");

        // Line type table
        self.write_group(0, "TABLE");
        self.write_group(2, "LTYPE");
        self.write_group_int(70, 7);

        // Continuous line type
        self.write_group(0, "LTYPE");
        self.write_group(2, "CONTINUOUS");
        self.write_group_int(70, 64);
        self.write_group(3, "Solid line");
        self.write_group_int(72, 65);
        self.write_group_int(73, 0);
        self.write_group(40, "0.0");

        self.write_group(0, "ENDTAB");

        // Layer table
        self.write_group(0, "TABLE");
        self.write_group(2, "LAYER");
        self.write_group_int(70, 6);

        for (name, color) in layers {
            self.write_layer(name, *color);
        }

        self.write_group(0, "ENDTAB");

        // Style table (empty)
        self.write_group(0, "TABLE");
        self.write_group(2, "STYLE");
        self.write_group_int(70, 2);
        self.write_group(0, "ENDTAB");

        // UCS table (empty)
        self.write_group(0, "TABLE");
        self.write_group(2, "UCS");
        self.write_group_int(70, 0);
        self.write_group(0, "ENDTAB");

        self.write_group(0, "ENDSEC");

        // Blocks section (empty)
        self.write_group(0, "SECTION");
        self.write_group(2, "BLOCKS");
        self.write_group(0, "ENDSEC");
    }

    /// Write a layer definition.
    fn write_layer(&mut self, name: &str, color: i32) {
        self.write_group(0, "LAYER");
        self.write_group(2, name);
        self.write_group_int(70, 64);
        self.write_group(62, &color.to_string());
        self.write_group(6, "CONTINUOUS");
    }

    /// Begin the entities section.
    pub fn begin_entities(&mut self) {
        self.write_group(0, "SECTION");
        self.write_group(2, "ENTITIES");
    }

    /// End the entities section and write EOF.
    pub fn end_entities(&mut self) {
        self.write_group(0, "ENDSEC");
        self.write_group(0, "EOF");
        writeln!(self.output, "%%").unwrap();
    }

    /// Write a LINE entity.
    pub fn write_line_entity(
        &mut self,
        layer: &str,
        color: i32,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) {
        self.write_entity_group(0, "LINE");
        self.write_entity_group(8, layer);
        self.write_entity_group(62, &color.to_string());
        self.write_entity_group(10, &Self::format_coord(x1));
        self.write_entity_group(20, &Self::format_coord(y1));
        self.write_entity_group(30, "0.000");
        self.write_entity_group(11, &Self::format_coord(x2));
        self.write_entity_group(21, &Self::format_coord(y2));
        self.write_entity_group(31, "0.000");
    }

    /// Write an ARC entity.
    /// Note: DXF arcs use center, radius, and start/end angles in degrees (0-360).
    #[allow(clippy::too_many_arguments)] // DXF arc requires all these parameters
    pub fn write_arc_entity(
        &mut self,
        layer: &str,
        color: i32,
        cx: f64,
        cy: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    ) {
        self.write_entity_group(0, "ARC");
        self.write_entity_group(8, layer);
        self.write_entity_group(62, &color.to_string());
        self.write_entity_group(10, &Self::format_coord(cx));
        self.write_entity_group(20, &Self::format_coord(cy));
        self.write_entity_group(30, "0.000");
        self.write_entity_group(40, &Self::format_coord(radius));
        self.write_entity_group(50, &Self::format_coord(normalize_degrees(start_angle)));
        self.write_entity_group(51, &Self::format_coord(normalize_degrees(end_angle)));
    }

    /// Format a color value (right-aligned in 4 chars with leading space).
    fn format_color(color: i32) -> String {
        format!("{:>4}", color)
    }

    /// Write a SOLID entity (filled rectangle).
    /// Note: SOLID uses different formatting (with spacing) vs LINE/ARC/TEXT (no spacing).
    pub fn write_solid_entity(
        &mut self,
        layer: &str,
        color: i32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.write_group(0, "SOLID");
        self.write_group(8, layer);
        self.write_group(62, &Self::format_color(color));
        self.write_group(10, &Self::format_coord(x));
        self.write_group(20, &Self::format_coord(y));
        self.write_group(30, "0.000");
        self.write_group(11, &Self::format_coord(x + width));
        self.write_group(21, &Self::format_coord(y));
        self.write_group(31, "0.000");
        self.write_group(12, &Self::format_coord(x));
        self.write_group(22, &Self::format_coord(y + height));
        self.write_group(32, "0.000");
        self.write_group(13, &Self::format_coord(x + width));
        self.write_group(23, &Self::format_coord(y + height));
        self.write_group(33, "0.000");
    }

    /// Write a TEXT entity.
    /// Note: TEXT uses spacing like SOLID (different from LINE/ARC).
    pub fn write_text_entity(&mut self, layer: &str, color: i32, x: f64, y: f64, text: &str) {
        self.write_group(0, "TEXT");
        self.write_group(8, layer);
        self.write_group(62, &color.to_string());
        self.write_group(10, &Self::format_coord(x));
        self.write_group(20, &Self::format_coord(y));
        self.write_group(30, "0.000");
        self.write_group(1, text);
    }
}

/// Format a dimension value with 3 decimal places using "round half away from zero".
fn format_dimension(value: f64) -> String {
    let scaled = value * 1000.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        (scaled - 0.5).ceil()
    };
    format!("{:.3}", rounded / 1000.0)
}

/// Generate text labels for a piece.
/// Uses view mode to handle coordinate transformation for normal vs mirrored views.
fn generate_piece_texts(
    dxf: &mut DxfWriter,
    schema: &Schema,
    piece: &Piece,
    colors: &DxfColors,
    mode: &ViewMode,
) {
    // Get piece type info if available
    let piece_type = piece
        .piece_type_index
        .and_then(|idx| schema.piece_types.get(idx));

    let is_scrap = piece_type.map(|pt| pt.waste).unwrap_or(false);
    if is_scrap {
        return; // Don't add labels to scrap pieces
    }

    // Calculate the base X coordinate for text positioning
    // For mirrored view, we need the mirrored piece origin
    let base_x = match mode {
        ViewMode::Normal => piece.x_origin,
        ViewMode::Mirrored { sheet_width } => sheet_width - piece.x_origin - piece.width,
    };

    // Dimensions text - positioned at center X, 3/4 height Y
    let dim_x = base_x + piece.width / 2.0;
    let dim_y = piece.y_origin + piece.height * 3.0 / 4.0;
    let dim_text = format!(
        "{} x {}",
        format_dimension(piece.width),
        format_dimension(piece.height)
    );
    dxf.write_text_entity("Dimensioni", colors.dimensions, dim_x, dim_y, &dim_text);

    // Shape name text - positioned at center, only if piece has shape
    if piece.shape_index.is_some() {
        let name_x = base_x + piece.width / 2.0;
        let name_y = piece.y_origin + piece.height / 2.0;
        // Note: For PRWC (mirrored view), output empty string without "?" fallback
        let shape_name = piece
            .shape_index
            .and_then(|idx| schema.shapes.get(idx))
            .map(|s| {
                if mode.is_mirrored() {
                    s.name.clone()
                } else if s.name.is_empty() {
                    "?".to_string()
                } else {
                    s.name.clone()
                }
            })
            .unwrap_or_else(|| {
                if mode.is_mirrored() {
                    String::new()
                } else {
                    "?".to_string()
                }
            });
        dxf.write_text_entity("NomeSagoma", colors.shape_name, name_x, name_y, &shape_name);
    }

    // Piece type code - positioned at 9/10 x, 9/10 y
    if let Some(pt) = piece_type {
        let type_x = base_x + piece.width * 9.0 / 10.0;
        let type_y = piece.y_origin + piece.height * 9.0 / 10.0;
        dxf.write_text_entity(
            "TipoP",
            colors.piece_type,
            type_x,
            type_y,
            &pt.piece_code.to_string(),
        );

        // Customer text - positioned at 3/4 x, 1/4 y
        if !pt.customer.is_empty() {
            let cust_x = base_x + piece.width * 3.0 / 4.0;
            let cust_y = piece.y_origin + piece.height * 1.0 / 4.0;
            dxf.write_text_entity("Cliente", colors.customer, cust_x, cust_y, &pt.customer);
        }

        // Order number text - positioned at 3/4 x, 1/8 y
        if !pt.order_no.is_empty() {
            let order_x = base_x + piece.width * 3.0 / 4.0;
            let order_y = piece.y_origin + piece.height * 1.0 / 8.0;
            dxf.write_text_entity("Ordine", colors.order, order_x, order_y, &pt.order_no);
        }
    }
}

/// Draw shape cuts for a piece.
fn draw_shape_cuts(
    dxf: &mut DxfWriter,
    piece: &Piece,
    shape: &Shape,
    colors: &DxfColors,
    mode: &ViewMode,
) {
    let ox = piece.x_origin;
    let oy = piece.y_origin;

    for cut in &shape.cuts {
        if !cut.active {
            continue;
        }

        match cut.cut_type {
            CutType::Line => {
                let xi = mode.transform_x(ox + cut.xi);
                let xf = mode.transform_x(ox + cut.xf);
                dxf.write_line_entity(
                    "TagliSag",
                    colors.shape_cuts,
                    xi,
                    oy + cut.yi,
                    xf,
                    oy + cut.yf,
                );
            }
            CutType::ArcCW | CutType::ArcCCW => {
                let cx = mode.transform_x(ox + cut.xc);
                let xi = mode.transform_x(ox + cut.xi);
                let xf = mode.transform_x(ox + cut.xf);

                // Calculate angles based on (potentially mirrored) coordinates
                let start_angle = (cut.yi - cut.yc).atan2(xi - cx).to_degrees();
                let end_angle = (cut.yf - cut.yc).atan2(xf - cx).to_degrees();

                // Arc direction logic:
                // - Normal view: CW swaps angles, CCW keeps them
                // - Mirrored view: CCW swaps angles, CW keeps them (mirroring reverses direction)
                let (a1, a2) = match (cut.cut_type, mode.is_mirrored()) {
                    (CutType::ArcCW, false) | (CutType::ArcCCW, true) => (end_angle, start_angle),
                    (CutType::ArcCCW, false) | (CutType::ArcCW, true) => (start_angle, end_angle),
                    _ => unreachable!(),
                };

                dxf.write_arc_entity(
                    "TagliSag",
                    colors.shape_cuts,
                    cx,
                    oy + cut.yc,
                    cut.radius,
                    a1,
                    a2,
                );
            }
        }
    }
}

/// Generate a DXF view (PRWB or PRWC) for a schema.
fn generate_dxf_view(schema: &Schema, mode: ViewMode) -> String {
    let mut dxf = DxfWriter::new();
    let colors = DxfColors::default();

    // Define layers
    let layers = vec![
        ("EST", colors.exterior),
        ("Tagli", colors.cuts),
        ("TagliSag", colors.shape_cuts),
        ("TipoP", colors.piece_type),
        ("Cliente", colors.customer),
        ("Ordine", colors.order),
        ("Dimensioni", colors.dimensions),
        ("NomeSagoma", colors.shape_name),
        ("ColPez", colors.piece_fill),
        ("ColSca", colors.scrap_fill),
    ];

    dxf.write_header();
    dxf.write_tables(&layers);
    dxf.begin_entities();

    // Draw scrap fill (background)
    dxf.write_solid_entity(
        "ColSca",
        colors.scrap_fill,
        0.0,
        0.0,
        schema.width,
        schema.height,
    );

    // Draw piece fills
    for piece in &schema.pieces {
        if piece.info_id.is_some() {
            let x = mode.transform_x(piece.x_origin + piece.width);
            // For mirrored view, transform gives us the right edge, so we need to subtract width
            // For normal view, we just use x_origin
            let fill_x = if mode.is_mirrored() {
                x
            } else {
                piece.x_origin
            };
            dxf.write_solid_entity(
                "ColPez",
                colors.piece_fill,
                fill_x,
                piece.y_origin,
                piece.width,
                piece.height,
            );
        }
    }

    // Draw sheet boundary (color 0 = BYLAYER)
    dxf.write_line_entity("EST", 0, 0.0, 0.0, schema.width, 0.0);
    dxf.write_line_entity("EST", 0, schema.width, 0.0, schema.width, schema.height);
    dxf.write_line_entity("EST", 0, schema.width, schema.height, 0.0, schema.height);
    dxf.write_line_entity("EST", 0, 0.0, schema.height, 0.0, 0.0);

    // Draw linear cuts
    for cut in &schema.linear_cuts {
        if cut.active {
            let xi = mode.transform_x(cut.xi);
            let xf = mode.transform_x(cut.xf);
            dxf.write_line_entity("Tagli", colors.cuts, xi, cut.yi, xf, cut.yf);
        }
    }

    // Draw shape cuts
    for piece in &schema.pieces {
        if let Some(shape_idx) = piece.shape_index {
            draw_shape_cuts(&mut dxf, piece, &schema.shapes[shape_idx], &colors, &mode);
        }
    }

    // Add text labels for piece information
    for piece in &schema.pieces {
        if piece.info_id.is_some() {
            generate_piece_texts(&mut dxf, schema, piece, &colors, &mode);
        }
    }

    dxf.end_entities();
    dxf.into_string()
}

/// Generate PRWB section (bottom view) for a schema.
pub fn generate_prwb(schema: &Schema, _schema_num: usize) -> String {
    generate_dxf_view(schema, ViewMode::Normal)
}

/// Generate PRWC section (C-side/mirrored view) for a schema.
pub fn generate_prwc(schema: &Schema, _schema_num: usize) -> String {
    generate_dxf_view(
        schema,
        ViewMode::Mirrored {
            sheet_width: schema.width,
        },
    )
}

/// Generate both PRWB and PRWC sections for all schemas.
pub fn generate_dxf_sections(schemas: &[Schema]) -> String {
    let mut output = String::new();

    for (idx, schema) in schemas.iter().enumerate() {
        let schema_num = idx + 1;

        // PRWB section
        writeln!(output, "[*PRWB{:04}_01]", schema_num).unwrap();
        output.push_str(&generate_prwb(schema, schema_num));

        // PRWC section
        writeln!(output, "[*PRWC{:04}_01]", schema_num).unwrap();
        output.push_str(&generate_prwc(schema, schema_num));
    }

    output
}
