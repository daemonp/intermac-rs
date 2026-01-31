//! OTD file parser module.

mod otd;
mod sections;

pub use otd::{parse_otd_file, OtdParser};
pub use sections::*;
