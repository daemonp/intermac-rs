//! Error types for OTD to CNI conversion.

use std::path::PathBuf;
use thiserror::Error;

/// Error codes for OTD processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// File not found (-1)
    FileNotFound = -1,
    /// Empty file (-2)
    EmptyFile = -2,
    /// General parse error (-3)
    ParseError = -3,
    /// No [Pattern] section found (-11)
    NoPatternSection = -11,
    /// Invalid arc geometry (E100)
    InvalidArc = 100,
    /// Coordinates out of bounds (E101)
    OutOfBounds = 101,
    /// Shape used on different-sized pieces (E200)
    ShapeSizeMismatch = 200,
    /// No cuts found in layout (E201)
    NoCutsFound = 201,
    /// Tool not found for thickness (E202)
    ToolNotFound = 202,
    /// OTX decryption failed (E300)
    DecryptionFailed = 300,
}

/// Main error type for the converter.
#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Empty file: {path}")]
    EmptyFile { path: PathBuf },

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("No [Pattern] section found in file")]
    NoPatternSection,

    #[error("Invalid section header at line {line}: {header}")]
    InvalidSection { line: usize, header: String },

    #[error("Missing required field '{field}' in section [{section}]")]
    MissingField { section: String, field: String },

    #[error("Invalid value for '{field}': expected {expected}, got '{value}'")]
    InvalidValue {
        field: String,
        expected: String,
        value: String,
    },

    #[error("Invalid arc geometry: radius {radius} is too small for endpoints ({x1}, {y1}) to ({x2}, {y2})")]
    InvalidArc {
        radius: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },

    #[error("Coordinates out of sheet bounds: ({x}, {y}) exceeds sheet size ({width} x {height})")]
    OutOfBounds {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },

    #[error("Shape {shape_id} is used on pieces of different sizes")]
    ShapeSizeMismatch { shape_id: i32 },

    #[error("No cuts found in layout")]
    NoCutsFound,

    #[error("Shape reference not found: Shape={shape_id}")]
    ShapeNotFound { shape_id: i32 },

    #[error("Info reference not found: Info={info_id}")]
    InfoNotFound { info_id: i32 },

    #[error("OTX decryption failed: {message}")]
    DecryptionFailed { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid numeric value at line {line}: {value}")]
    InvalidNumber { line: usize, value: String },
}

impl ConvertError {
    /// Get the error code for this error.
    pub fn code(&self) -> ErrorCode {
        match self {
            ConvertError::FileNotFound { .. } => ErrorCode::FileNotFound,
            ConvertError::EmptyFile { .. } => ErrorCode::EmptyFile,
            ConvertError::ParseError { .. } => ErrorCode::ParseError,
            ConvertError::NoPatternSection => ErrorCode::NoPatternSection,
            ConvertError::InvalidSection { .. } => ErrorCode::ParseError,
            ConvertError::MissingField { .. } => ErrorCode::ParseError,
            ConvertError::InvalidValue { .. } => ErrorCode::ParseError,
            ConvertError::InvalidArc { .. } => ErrorCode::InvalidArc,
            ConvertError::OutOfBounds { .. } => ErrorCode::OutOfBounds,
            ConvertError::ShapeSizeMismatch { .. } => ErrorCode::ShapeSizeMismatch,
            ConvertError::NoCutsFound => ErrorCode::NoCutsFound,
            ConvertError::ShapeNotFound { .. } => ErrorCode::ParseError,
            ConvertError::InfoNotFound { .. } => ErrorCode::ParseError,
            ConvertError::DecryptionFailed { .. } => ErrorCode::DecryptionFailed,
            ConvertError::Io(_) => ErrorCode::FileNotFound,
            ConvertError::InvalidNumber { .. } => ErrorCode::ParseError,
        }
    }

    /// Get the numeric error code value.
    pub fn code_value(&self) -> i32 {
        self.code() as i32
    }
}

/// Result type alias for converter operations.
pub type Result<T> = std::result::Result<T, ConvertError>;
