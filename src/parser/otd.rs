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
            schema.linear_advance = pattern_data.linear_advance;
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
    fn process_coordinates(
        &self,
        entries: &[CoordEntry],
        schema: &Schema,
    ) -> (Vec<Cut>, Vec<Piece>) {
        let cuts = Vec::new();
        let mut pieces = Vec::new();

        if entries.is_empty() {
            return (cuts, pieces);
        }

        // Stack to track current position at each level
        // Index 0=X (vertical cuts), 1=Y (horizontal cuts), etc.
        let mut stack: Vec<(char, f64)> = Vec::new();

        // Track dimensions at each level for piece calculation
        let mut level_positions: Vec<f64> = vec![0.0; 10];
        let mut level_sizes: Vec<f64> = vec![0.0; 10];

        // Initialize with sheet dimensions
        level_sizes[0] = schema.width - schema.trim_left;
        level_sizes[1] = schema.height - schema.trim_bottom;

        for entry in entries {
            let level = entry.level as usize;

            // Pop stack back to this level
            while stack.len() > level {
                stack.pop();
            }

            // Push this entry
            stack.push((entry.var, entry.value));

            // Calculate absolute position and create cut
            let _is_vertical = level % 2 == 0; // X, Z, V, B, D are vertical
            let cut_value = entry.value;

            // Update level tracking
            if level > 0 {
                level_positions[level] += level_sizes[level];
            }
            level_sizes[level] = cut_value;

            // Calculate origin for piece at this position
            let _origin_x = schema.trim_left;
            let _origin_y = schema.trim_bottom;
            let mut piece_width = cut_value;
            let mut piece_height = cut_value;

            // Walk the stack to compute actual position
            for (i, (_var, val)) in stack.iter().enumerate() {
                if i % 2 == 0 {
                    // Vertical variable (X, Z, V...)
                    piece_width = *val;
                } else {
                    // Horizontal variable (Y, W, A...)
                    piece_height = *val;
                }
            }

            // Compute origin by summing previous cuts at each level
            // This is a simplified calculation - the full algorithm is more complex
            // and involves tracking the "rest" at each level

            // If this entry has Shape/Info, it defines a piece
            if entry.shape_id.is_some() || entry.info_id.is_some() {
                let mut piece = Piece::default();
                piece.shape_id = entry.shape_id;
                piece.info_id = entry.info_id;
                piece.width = piece_width;
                piece.height = piece_height;
                // Origin calculation would go here - simplified for now
                pieces.push(piece);
            }

            // Create linear cut for this coordinate
            // In the full implementation, cuts are created based on the hierarchy
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

    // Password and IV from the C# code
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
        buffer.extend(std::iter::repeat(padding as u8).take(padding));
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
