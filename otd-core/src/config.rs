//! Configuration constants and settings for the converter.

/// Floating-point comparison epsilon.
pub const EPS: f64 = 0.0001;

/// Minimum distance from sheet edge for cuts.
pub const D_MIN_BORDO: f64 = 2.0;

/// Minimum distance for continuous path detection.
pub const D_MIN_CONT: f64 = 0.0001;

/// Default linear advance in mm (1mm is standard).
pub const DEFAULT_LINEAR_ADVANCE_MM: f64 = 1.0;

/// Minimum positive value for C axis (tangent mode).
pub const MIN_VAL_C_POSITIVE: f64 = 0.001;

/// Conversion factor: mm to inch.
pub const CONV_MM_INCH: f64 = 25.4;

/// Conversion factor: mm to Tinch (tenths of inch used in some systems).
pub const CONV_MM_TINCH: f64 = 30.303;

/// Minimum piece dimension for score-only cutting.
pub const DIM_MIN_PEZZO_SOLO_INCISIONE: f64 = 100.0;

/// Minimum rest dimension for score-only cutting (normal thickness).
pub const DIM_MIN_RESTO_SOLO_INCISIONE: f64 = 50.0;

/// Minimum rest dimension for score-only cutting (high thickness).
pub const DIM_MIN_RESTO_SOLO_INC_HI_SPESS: f64 = 80.0;

/// Thickness threshold for high-thickness mode.
pub const SOGLIA_SPESSORE_ELEVATO: f64 = 6.0;

/// Default linear tool code.
pub const DEFAULT_LINEAR_TOOL: u16 = 3;

/// Default shaped tool code.
pub const DEFAULT_SHAPED_TOOL: u16 = 31;

/// Tool type constant for shaped cuts.
pub const TOOL_TYPE_SHAPED: u32 = 1;

/// Coarse epsilon for rest dimension calculations.
pub const EPS_COARSE: f64 = 0.001;

use serde::{Deserialize, Serialize};

/// Unit of measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Unit {
    #[default]
    Millimeters,
    Inches,
    TenthsOfInch,
}

impl Unit {
    /// Parse unit from OTD Dimension= value.
    pub fn from_dimension_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "mm" => Some(Unit::Millimeters),
            "inch" => Some(Unit::Inches),
            "tinch" => Some(Unit::TenthsOfInch),
            _ => None,
        }
    }

    /// Get the conversion factor to convert from this unit to millimeters.
    pub fn to_mm_factor(&self) -> f64 {
        match self {
            Unit::Millimeters => 1.0,
            Unit::Inches => CONV_MM_INCH,
            Unit::TenthsOfInch => CONV_MM_TINCH,
        }
    }

    /// Get the G-code for this unit system.
    pub fn gcode(&self) -> &'static str {
        match self {
            Unit::Millimeters => "G71",
            Unit::Inches | Unit::TenthsOfInch => "G70",
        }
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unit::Millimeters => write!(f, "mm"),
            Unit::Inches => write!(f, "inch"),
            Unit::TenthsOfInch => write!(f, "Tinch"),
        }
    }
}

/// Machine configuration.
#[derive(Debug, Clone)]
pub struct MachineConfig {
    /// Machine number (100-199 for cutting tables).
    pub machine_number: u16,
    /// Linear tool code.
    pub linear_tool: u16,
    /// Shaped tool code.
    pub shaped_tool: u16,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            machine_number: 130,
            linear_tool: DEFAULT_LINEAR_TOOL,
            shaped_tool: DEFAULT_SHAPED_TOOL,
        }
    }
}

impl MachineConfig {
    /// Create a new machine configuration.
    pub fn new(machine_number: u16) -> Self {
        Self {
            machine_number,
            ..Default::default()
        }
    }

    /// Check if this is a cutting table machine (100-199).
    pub fn is_cutting_table(&self) -> bool {
        self.machine_number >= 100 && self.machine_number < 200
    }

    /// Check if this is a laminated glass machine (200+).
    pub fn is_laminated(&self) -> bool {
        self.machine_number >= 200
    }
}

/// Utility functions for floating-point comparisons.
pub mod float_cmp {
    use super::EPS;

    /// Check if two floats are approximately equal.
    #[inline]
    pub fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    /// Check if a float is approximately zero.
    #[inline]
    pub fn approx_zero(a: f64) -> bool {
        a.abs() < EPS
    }

    /// Check if a is in range [min, max] with epsilon tolerance.
    #[inline]
    pub fn in_range(a: f64, min: f64, max: f64) -> bool {
        a >= min - EPS && a <= max + EPS
    }

    /// Check if a number is even (for cut level determination).
    #[inline]
    pub fn is_even(n: i32) -> bool {
        n % 2 == 0
    }
}

/// Utility functions for angle operations.
pub mod angle {
    /// Normalize angle to 0-360 range (exclusive of 360).
    #[inline]
    pub fn normalize_degrees(angle: f64) -> f64 {
        let mut a = angle % 360.0;
        if a < 0.0 {
            a += 360.0;
        }
        // Handle 360.0 and -0.0 cases
        if a >= 360.0 || a == 0.0 {
            a = 0.0;
        }
        a
    }
}
