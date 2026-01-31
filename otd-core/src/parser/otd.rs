//! Main OTD file parser.

use crate::error::{ConvertError, Result};
use crate::model::{Cut, Piece, Schema};
use std::path::Path;

use super::sections::*;

/// OTD file parser.
pub struct OtdParser {
    /// File content as lines.
    lines: Vec<String>,
    /// Section indices: (name, start_line, end_line).
    sections: Vec<(String, usize, usize)>,
}

impl OtdParser {
    /// Create a new parser from file content.
    pub fn new(content: String) -> Self {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let sections = Self::find_sections(&lines);
        Self { lines, sections }
    }

    /// Find all sections and their line ranges.
    fn find_sections(lines: &[String]) -> Vec<(String, usize, usize)> {
        let mut sections = Vec::new();
        let mut current_section: Option<(String, usize)> = None;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                // Close previous section
                if let Some((name, start)) = current_section.take() {
                    sections.push((name, start, i - 1));
                }
                // Start new section
                let section_name = trimmed[1..trimmed.len() - 1].to_string();
                current_section = Some((section_name, i));
            }
        }

        // Close last section
        if let Some((name, start)) = current_section {
            sections.push((name, start, lines.len() - 1));
        }

        sections
    }

    /// Get the lines for a section (excluding the header line).
    fn get_section_lines(&self, name: &str) -> Option<Vec<&str>> {
        self.sections
            .iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, start, end)| {
                self.lines[*start + 1..=*end]
                    .iter()
                    .map(|s| s.as_str())
                    .collect()
            })
    }

    /// Get all sections with a given name (for multiple Pattern sections).
    fn get_all_sections(&self, name: &str) -> Vec<Vec<&str>> {
        self.sections
            .iter()
            .filter(|(n, _, _)| n == name)
            .map(|(_, start, end)| {
                self.lines[*start + 1..=*end]
                    .iter()
                    .map(|s| s.as_str())
                    .collect()
            })
            .collect()
    }

    /// Count sections with a given name.
    fn count_sections(&self, name: &str) -> usize {
        self.sections.iter().filter(|(n, _, _)| n == name).count()
    }

    /// Get sections between two Pattern sections (for Info/Shape belonging to a pattern).
    fn get_sections_for_pattern(&self, pattern_index: usize) -> Vec<(String, Vec<&str>)> {
        let pattern_sections: Vec<_> = self
            .sections
            .iter()
            .enumerate()
            .filter(|(_, (n, _, _))| n == "Pattern")
            .collect();

        if pattern_index >= pattern_sections.len() {
            return Vec::new();
        }

        let (section_idx, (_, _, _pattern_end)) = pattern_sections[pattern_index];
        let next_pattern_start = if pattern_index + 1 < pattern_sections.len() {
            pattern_sections[pattern_index + 1].1 .1
        } else {
            self.lines.len()
        };

        self.sections[section_idx + 1..]
            .iter()
            .take_while(|(_, start, _)| *start < next_pattern_start)
            .filter(|(name, _, _)| {
                name == "Info" || name == "Shape" || name == "Cuttings" || name == "LowE"
            })
            .map(|(name, start, end)| {
                let lines = self.lines[*start + 1..=*end]
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                (name.clone(), lines)
            })
            .collect()
    }

    /// Parse all schemas from the file.
    pub fn parse(&self) -> Result<Vec<Schema>> {
        let num_patterns = self.count_sections("Pattern");
        if num_patterns == 0 {
            return Err(ConvertError::NoPatternSection);
        }

        // Parse header (shared by all patterns)
        let header_data = self
            .get_section_lines("Header")
            .map(|lines| parse_header(&lines))
            .unwrap_or_default();

        // Parse signature (shared by all patterns)
        let signature_data = self
            .get_section_lines("Signature")
            .map(|lines| parse_signature(&lines))
            .unwrap_or_default();

        let pattern_sections = self.get_all_sections("Pattern");
        let mut schemas = Vec::with_capacity(num_patterns);

        for (pattern_idx, pattern_lines) in pattern_sections.iter().enumerate() {
            let mut schema = Schema::new();

            // Apply header data
            schema.otd_version = header_data.otd_version.clone();
            schema.unit = header_data.unit;
            schema.date = header_data.date.clone();
            schema.creator = signature_data.creator.clone();

            // Parse pattern header
            let pattern_data = parse_pattern_header(pattern_lines);
            schema.machine_name = pattern_data.machine_name;
            schema.machine_number = pattern_data.machine_number;
            schema.glass_id = pattern_data.glass_id;
            schema.glass_description = pattern_data.glass_description;
            schema.thickness = pattern_data.thickness;
            schema.glass_structured = pattern_data.glass_structured;
            schema.glass_coated = pattern_data.glass_coated;
            schema.width = pattern_data.width;
            schema.height = pattern_data.height;
            schema.trim_left = pattern_data.trim_left;
            schema.trim_bottom = pattern_data.trim_bottom;
            schema.quantity = pattern_data.quantity;
            schema.cutting_order = pattern_data.cutting_order;
            // Apply default linear advance if not specified in OTD
            // Standard default is 1mm, converted to current units
            if pattern_data.linear_advance > 0.0 {
                schema.linear_advance = pattern_data.linear_advance;
            } else {
                // Convert 1mm to the file's units
                schema.linear_advance =
                    crate::config::DEFAULT_LINEAR_ADVANCE_MM / schema.unit.to_mm_factor();
            }
            schema.min_angle = pattern_data.min_angle;
            schema.coating_min_angle = pattern_data.coating_min_angle;
            schema.linear_tool = pattern_data.linear_tool;
            schema.shaped_tool = pattern_data.shaped_tool;
            schema.open_shaped_tool = pattern_data.open_shaped_tool;
            schema.incision_tool = pattern_data.incision_tool;
            schema.optimize_shape_order = pattern_data.optimize_shape_order;

            // Parse nested coordinates to get pieces and cuts
            let coord_entries = parse_pattern_coordinates(pattern_lines);
            let (linear_cuts, pieces) = self.process_coordinates(&coord_entries, &schema);
            schema.linear_cuts = linear_cuts;
            schema.pieces = pieces;

            // Parse associated sections (Info, Shape, Cuttings, LowE)
            let associated = self.get_sections_for_pattern(pattern_idx);

            for (section_name, section_lines) in associated {
                match section_name.as_str() {
                    "Info" => {
                        if let Some(piece_type) = parse_info(&section_lines) {
                            schema.piece_types.push(piece_type);
                        }
                    }
                    "Shape" => {
                        if let Some(shape) = parse_shape(&section_lines) {
                            schema.shapes.push(shape);
                        }
                    }
                    "Cuttings" => {
                        let (cuts, pieces) = parse_cuttings(&section_lines);
                        if !cuts.is_empty() {
                            schema.linear_cuts = cuts;
                            schema.linear_cuts_optimized = true;
                        }
                        if !pieces.is_empty() {
                            schema.pieces = pieces;
                            schema.optimize_shape_order = false;
                        }
                    }
                    "LowE" => {
                        let (cuts, pieces) = parse_lowe(&section_lines);
                        schema.lowe_cuts = cuts;
                        schema.lowe_pieces = pieces;
                    }
                    _ => {}
                }
            }

            // Resolve piece references
            schema.resolve_piece_references();
            schema.calculate_piece_edges();

            schemas.push(schema);
        }

        Ok(schemas)
    }

    /// Process nested coordinates to generate cuts and pieces.
    /// Implements hierarchical coordinate processing for the OTD nested format.
    fn process_coordinates(
        &self,
        entries: &[CoordEntry],
        schema: &Schema,
    ) -> (Vec<Cut>, Vec<Piece>) {
        use crate::model::LineType;

        let mut cuts = Vec::new();
        let mut pieces = Vec::new();

        if entries.is_empty() {
            return (cuts, pieces);
        }

        let num_entries = entries.len();

        // Arrays to track entry data
        let mut levels: Vec<i32> = vec![-1; num_entries]; // numArray1 - level index
        let mut orientations: Vec<&str> = vec![""; num_entries]; // strArray2 - "V" or "O"
        let mut values: Vec<f64> = vec![0.0; num_entries]; // numArray2 - coordinate value
        let mut info_ids: Vec<i32> = vec![-1; num_entries]; // numArray3
        let mut shape_ids: Vec<i32> = vec![-1; num_entries]; // numArray4
        let mut has_info: Vec<bool> = vec![false; num_entries]; // flagArray1
        let mut has_shape: Vec<bool> = vec![false; num_entries]; // flagArray2
        let mut rotations: Vec<f64> = vec![0.0; num_entries]; // numArray5
        let mut tcuts: Vec<i32> = vec![-1; num_entries]; // numArray6

        // Parse entries into arrays
        for (i, entry) in entries.iter().enumerate() {
            levels[i] = entry.level;
            values[i] = entry.value;
            // Even levels (X=0, Z=2, V=4, B=6, D=8) are vertical "V"
            // Odd levels (Y=1, W=3, A=5, C=7, E=9) are horizontal "O"
            orientations[i] = if entry.level % 2 == 0 { "V" } else { "O" };

            if let Some(info) = entry.info_id {
                info_ids[i] = info;
                has_info[i] = true;
            }
            if let Some(shape) = entry.shape_id {
                shape_ids[i] = shape;
                has_shape[i] = true;
            }
            rotations[i] =
                entry
                    .rotation
                    .unwrap_or_else(|| if orientations[i] == "V" { 0.0 } else { 90.0 });
            tcuts[i] = entry.tcut.unwrap_or(-1);
        }

        // Add trim cuts if needed
        if schema.trim_left > 0.0 {
            let mut cut = Cut::new_line(schema.trim_left, 0.0, schema.trim_left, schema.height);
            cut.line_type = LineType::Vertical;
            cut.level = 0;
            cut.rotation = 90.0;
            cuts.push(cut);
        }
        if schema.trim_bottom > 0.0 {
            let mut cut = Cut::new_line(0.0, schema.trim_bottom, schema.width, schema.trim_bottom);
            cut.line_type = LineType::Horizontal;
            cut.level = 0;
            cut.rotation = 0.0;
            cuts.push(cut);
        }

        // Process each entry to calculate position and create cuts/pieces
        for i in 0..num_entries {
            // Build the path from root to this entry
            // Walk backwards to find all ancestors with lower or equal level
            let mut path_indices: Vec<usize> = Vec::new();
            let current_level = levels[i];
            path_indices.push(i);

            let mut min_level = current_level;
            for j in (0..i).rev() {
                if levels[j] <= min_level {
                    min_level = levels[j];
                    path_indices.push(j);
                }
            }
            path_indices.reverse();

            // Calculate accumulated offsets and remaining dimensions
            let mut offset_x = 0.0; // num31 - accumulated X offset
            let mut offset_y = 0.0; // num32 - accumulated Y offset
            let mut dim_x = schema.width - schema.trim_left; // num33 - remaining X dimension
            let mut dim_y = schema.height - schema.trim_bottom; // num34 - remaining Y dimension
            let mut prev_level = -1;
            let mut rest_x = -1.0; // resto1
            let mut rest_y = -1.0; // resto2

            for &idx in &path_indices {
                let level = levels[idx];
                let value = values[idx];
                let orient = orientations[idx];

                if level > prev_level {
                    // New level - update dimensions
                    match orient {
                        "V" => {
                            rest_x = dim_x - value;
                            dim_x = value;
                        }
                        "O" => {
                            rest_y = dim_y - value;
                            dim_y = value;
                        }
                        _ => {}
                    }
                    prev_level = level;
                } else if level == prev_level {
                    // Same level - add to offset
                    match orient {
                        "V" => {
                            offset_x += dim_x;
                            rest_x -= value;
                            dim_x = value;
                        }
                        "O" => {
                            offset_y += dim_y;
                            rest_y -= value;
                            dim_y = value;
                        }
                        _ => {}
                    }
                }
            }

            // Calculate cut coordinates
            let (xi, yi, xf, yf) = if orientations[i] == "V" {
                // Vertical cut
                let x = offset_x + dim_x + schema.trim_left;
                let y1 = offset_y + schema.trim_bottom;
                let y2 = y1 + dim_y;
                (x, y1, x, y2)
            } else {
                // Horizontal cut
                let x1 = offset_x + schema.trim_left;
                let x2 = x1 + dim_x;
                let y = offset_y + dim_y + schema.trim_bottom;
                (x1, y, x2, y)
            };

            // Create linear cut
            let mut cut = Cut::new_line(xi, yi, xf, yf);
            cut.level = levels[i];
            cut.rotation = rotations[i];
            cut.quota = values[i];
            cut.tcut = tcuts[i];
            cut.rest = if orientations[i] == "V" {
                rest_x
            } else {
                rest_y
            };
            cut.line_type = if orientations[i] == "V" {
                LineType::Vertical
            } else {
                LineType::Horizontal
            };
            cuts.push(cut);

            // Create piece if has Info or Shape
            if has_info[i] || has_shape[i] {
                let mut piece = Piece::new(
                    offset_x + schema.trim_left,
                    offset_y + schema.trim_bottom,
                    dim_x,
                    dim_y,
                );
                if has_info[i] {
                    piece.info_id = Some(info_ids[i]);
                }
                if has_shape[i] {
                    piece.shape_id = Some(shape_ids[i]);
                }
                pieces.push(piece);
            }
        }

        (cuts, pieces)
    }
}

/// Parse an OTD file from a path.
pub fn parse_otd_file(path: &Path) -> Result<Vec<Schema>> {
    use std::fs;

    if !path.exists() {
        return Err(ConvertError::FileNotFound {
            path: path.to_path_buf(),
        });
    }

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = if extension == "otx" {
        // Decrypt OTX file
        let encrypted = fs::read(path)?;
        decrypt_otx(&encrypted)?
    } else {
        fs::read_to_string(path)?
    };

    if content.trim().is_empty() {
        return Err(ConvertError::EmptyFile {
            path: path.to_path_buf(),
        });
    }

    let parser = OtdParser::new(content);
    parser.parse()
}

/// Decrypt an OTX file.
fn decrypt_otx(encrypted: &[u8]) -> Result<String> {
    use cipher::{BlockDecryptMut, KeyIvInit};
    use md5::{Digest, Md5};

    // Password and IV for OTX decryption
    let password = b"%x$Intermac^(zx";
    let iv: [u8; 8] = [68, 101, 67, 97, 114, 110, 101, 68]; // "DeCarneD"

    // Derive key using MD5 (simplified version of CryptDeriveKey)
    let mut hasher = Md5::new();
    hasher.update(password);
    let hash = hasher.finalize();

    // RC2 with 128-bit effective key length uses 16 bytes
    let key: [u8; 16] = hash.into();

    // Create RC2-CBC decryptor
    type Rc2CbcDec = cbc::Decryptor<rc2::Rc2>;

    let mut buffer = encrypted.to_vec();

    // Pad to block size if needed
    let block_size = 8;
    let padding = block_size - (buffer.len() % block_size);
    if padding != block_size {
        buffer.extend(std::iter::repeat_n(padding as u8, padding));
    }

    let decryptor =
        Rc2CbcDec::new_from_slices(&key, &iv).map_err(|e| ConvertError::DecryptionFailed {
            message: format!("Failed to create decryptor: {}", e),
        })?;

    let decrypted = decryptor
        .decrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut buffer)
        .map_err(|e| ConvertError::DecryptionFailed {
            message: format!("Decryption failed: {}", e),
        })?;

    String::from_utf8(decrypted.to_vec()).map_err(|e| ConvertError::DecryptionFailed {
        message: format!("Invalid UTF-8 in decrypted content: {}", e),
    })
}
