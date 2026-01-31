//! G-code generation utilities.

use std::fmt::Write;

/// G-code writer with line numbering.
pub struct GcodeWriter {
    /// Current line number.
    line_number: u32,
    /// Line number increment.
    increment: u32,
    /// Output buffer.
    buffer: String,
}

impl GcodeWriter {
    /// Create a new G-code writer.
    pub fn new() -> Self {
        Self {
            line_number: 10,
            increment: 10,
            buffer: String::new(),
        }
    }

    /// Create a new G-code writer starting at a specific line number.
    pub fn with_start(start: u32) -> Self {
        Self {
            line_number: start,
            increment: 10,
            buffer: String::new(),
        }
    }

    /// Get the current line number.
    pub fn current_line(&self) -> u32 {
        self.line_number
    }

    /// Get the generated G-code.
    pub fn output(&self) -> &str {
        &self.buffer
    }

    /// Take the generated G-code.
    pub fn take_output(self) -> String {
        self.buffer
    }

    /// Write a numbered line.
    pub fn write_line(&mut self, content: &str) {
        writeln!(self.buffer, "N{} {}", self.line_number, content).unwrap();
        self.line_number += self.increment;
    }

    /// Write a line without numbering.
    pub fn write_raw(&mut self, content: &str) {
        writeln!(self.buffer, "{}", content).unwrap();
    }

    /// Write a comment line.
    pub fn write_comment(&mut self, comment: &str) {
        writeln!(self.buffer, "; {}", comment).unwrap();
    }

    /// Write a label.
    pub fn write_label(&mut self, label: &str) {
        writeln!(self.buffer, ":{}", label).unwrap();
    }

    /// Write a section terminator.
    pub fn write_terminator(&mut self) {
        writeln!(self.buffer, "%").unwrap();
    }

    // === Parameter commands ===

    /// Set a parameter value.
    pub fn set_param(&mut self, param: u16, value: impl std::fmt::Display) {
        self.write_line(&format!("P{:03}={}", param, value));
    }

    /// Set a parameter with a formatted float.
    pub fn set_param_float(&mut self, param: u16, value: f64) {
        self.write_line(&format!("P{:03}={}", param, format_coord(value)));
    }

    /// Set the tool parameter (P007).
    pub fn set_tool(&mut self, tool_code: u16) {
        self.write_line(&format!("P007={:04}", tool_code));
    }

    /// Set the rotation angle (P539).
    ///
    /// Uses integer format for angles like 90, 0 for horizontal.
    pub fn set_rotation(&mut self, angle: f64) {
        let angle_str = if (angle - angle.round()).abs() < 0.0001 {
            // Use integer format for whole number angles (0, 90, 180, etc.)
            format!("{}", angle.round() as i32)
        } else {
            format_coord(angle)
        };
        self.write_line(&format!("P539={}", angle_str));
    }

    /// Set the rotation angle for shape cuts (P539).
    ///
    /// Uses MIN_VAL_C_POSITIVE for small angles (tangent mode requires non-zero).
    pub fn set_rotation_shape(&mut self, angle: f64) {
        use crate::config::MIN_VAL_C_POSITIVE;

        let angle_str = if angle.abs() < MIN_VAL_C_POSITIVE {
            // Use minimum positive value for tangent mode
            format!("{}", MIN_VAL_C_POSITIVE)
        } else if (angle - angle.round()).abs() < 0.0001 {
            // Use integer format for whole number angles
            format!("{}", angle.round() as i32)
        } else {
            format_coord(angle)
        };
        self.write_line(&format!("P539={}", angle_str));
    }

    // === Movement commands ===

    /// Rapid move (G00).
    pub fn rapid_move(&mut self, x: f64, y: f64, c: Option<&str>) {
        let c_str = c.map(|s| format!(" C={}", s)).unwrap_or_default();
        self.write_line(&format!(
            "G00 X={} Y={}{}",
            format_coord(x),
            format_coord(y),
            c_str
        ));
    }

    /// Linear interpolation (G01).
    pub fn linear_move(&mut self, x: f64, y: f64, c: Option<&str>) {
        let c_str = c.map(|s| format!(" C={}", s)).unwrap_or_default();
        self.write_line(&format!(
            "G01 X={} Y={}{}",
            format_coord(x),
            format_coord(y),
            c_str
        ));
    }

    /// Clockwise arc (G02).
    pub fn arc_cw(&mut self, x: f64, y: f64, i: f64, j: f64) {
        self.write_line(&format!(
            "G02 X={} Y={} I={} J={}",
            format_coord(x),
            format_coord(y),
            format_coord(i),
            format_coord(j)
        ));
    }

    /// Counter-clockwise arc (G03).
    pub fn arc_ccw(&mut self, x: f64, y: f64, i: f64, j: f64) {
        self.write_line(&format!(
            "G03 X={} Y={} I={} J={}",
            format_coord(x),
            format_coord(y),
            format_coord(i),
            format_coord(j)
        ));
    }

    // === Macro calls ===

    /// Call a macro/subroutine.
    pub fn call_macro(&mut self, name: &str) {
        self.write_line(&format!("L={}", name));
    }

    /// Call a labeled subroutine.
    pub fn call_label(&mut self, label: &str) {
        self.write_line(&format!("L:{}", label));
    }

    /// Conditional jump.
    pub fn jump_if(&mut self, condition: &str, label: &str) {
        self.write_line(&format!("JM({}):{}", condition, label));
    }

    /// Unconditional jump.
    pub fn jump(&mut self, label: &str) {
        self.write_line(&format!("JM:{}", label));
    }

    /// Complex conditional jump.
    pub fn jump_complex(&mut self, conditions: &str, label: &str) {
        self.write_line(&format!("JM({}):{}", conditions, label));
    }

    // === Tool control ===

    /// Tool up.
    pub fn tool_up(&mut self) {
        self.call_macro("PT_SU");
    }

    /// Tool down.
    pub fn tool_down(&mut self) {
        self.call_macro("PT_GIU");
    }

    /// Load tool.
    pub fn load_tool(&mut self) {
        self.call_macro("PTOOL");
    }

    /// Apply rotation.
    pub fn apply_rotation(&mut self) {
        self.call_macro("PROT_B");
    }

    // === Miscellaneous ===

    /// Direction code for cut (M=532 or M=533).
    pub fn direction_code(&mut self, is_vertical: bool) {
        let code = if is_vertical { 533 } else { 532 };
        self.write_line(&format!("M={}", code));
    }

    /// Set work offset (G58).
    pub fn set_work_offset(&mut self) {
        self.write_line("G58");
    }

    /// Set XO offset.
    pub fn set_xo(&mut self, value: f64) {
        self.write_line(&format!("XO={}", format_coord(value)));
    }

    /// Set YO offset.
    pub fn set_yo(&mut self, value: f64) {
        self.write_line(&format!("YO={}", format_coord(value)));
    }

    /// Enable tangent mode (G28).
    pub fn tangent_mode_on(&mut self) {
        self.write_line("G28");
    }

    /// Disable tangent mode (G46).
    pub fn tangent_mode_off(&mut self) {
        self.write_line("G01 G46");
    }

    /// Set shape parameters.
    pub fn set_shape_params(&mut self, perimeter: f64, tool: i32) {
        self.call_macro("PSETSAG");
        self.set_param_float(203, perimeter);
        self.set_param(204, tool);
    }
}

impl Default for GcodeWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a coordinate value for G-code output.
///
/// Uses "G15" format (15 significant digits) for precise coordinate output.
pub fn format_coord(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }

    // Check for whole numbers first
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }

    let abs_val = value.abs();

    // G15 format: 15 significant digits total
    // For numbers >= 1: subtract integer digit count from 15 for decimal places
    // For numbers < 1: add leading zero count to 15 for decimal places
    const SIG_DIGITS: i32 = 15;
    let decimal_places = if abs_val >= 1.0 {
        let int_digits = abs_val.log10().floor() as i32 + 1;
        (SIG_DIGITS - int_digits).max(0) as usize
    } else {
        // For numbers like 0.0393..., leading zeros don't count toward sig digits
        let leading_zeros = (-abs_val.log10().floor()) as i32 - 1;
        (SIG_DIGITS + leading_zeros.max(0)) as usize
    };

    let formatted = format!("{:.prec$}", value, prec = decimal_places);

    // Trim trailing zeros and possible trailing decimal point
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');

    if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Format a tool code as 4-digit string.
pub fn format_tool_code(code: u16) -> String {
    format!("{:04}", code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_coord() {
        // Basic values
        assert_eq!(format_coord(0.0), "0");
        assert_eq!(format_coord(1.0), "1"); // Integers have no decimal point
        assert_eq!(format_coord(1.5), "1.5");
        assert_eq!(format_coord(1.500000), "1.5");
        assert_eq!(format_coord(129.5), "129.5");
        assert_eq!(format_coord(129.0), "129");
        assert_eq!(format_coord(0.125984), "0.125984");
        assert_eq!(format_coord(90.0), "90");

        // G15 format compatibility tests (15 significant digits)
        assert_eq!(format_coord(1.0 / 25.4), "0.0393700787401575");
        assert_eq!(format_coord(129.5 - 1.0 / 25.4), "129.46062992126");
        assert_eq!(format_coord(49.6875 - 1.0 / 25.4), "49.6481299212598");
        assert_eq!(format_coord(0.5 / 25.4), "0.0196850393700787");
    }

    #[test]
    fn test_gcode_writer() {
        let mut writer = GcodeWriter::new();
        writer.write_line("G70 LX=100 LY=100");
        writer.write_terminator();

        let output = writer.output();
        assert!(output.contains("N10 G70"));
        assert!(output.contains("%"));
    }
}
