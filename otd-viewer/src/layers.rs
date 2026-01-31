//! Layer visibility controls.

#![allow(dead_code)] // Some fields and methods reserved for future features

/// Controls which visual layers are displayed on the canvas.
#[derive(Debug, Clone)]
pub struct LayerVisibility {
    /// Show glass sheet bounds
    pub sheet: bool,
    /// Show trim zone overlay
    pub trim: bool,
    /// Show linear cut lines
    pub linear_cuts: bool,
    /// Show piece rectangles
    pub pieces: bool,
    /// Show shape contours
    pub shapes: bool,
    /// Show piece labels/IDs
    pub labels: bool,
    /// Show waste regions
    pub waste: bool,
    /// Show coordinate grid
    pub grid: bool,
}

impl Default for LayerVisibility {
    fn default() -> Self {
        Self {
            sheet: true,
            trim: true,
            linear_cuts: true,
            pieces: true,
            shapes: true,
            labels: false,
            waste: false,
            grid: false,
        }
    }
}

impl LayerVisibility {
    /// Show all layers
    pub fn show_all(&mut self) {
        self.sheet = true;
        self.trim = true;
        self.linear_cuts = true;
        self.pieces = true;
        self.shapes = true;
        self.labels = true;
        self.waste = true;
        self.grid = true;
    }

    /// Hide all layers except sheet
    pub fn minimal(&mut self) {
        self.sheet = true;
        self.trim = false;
        self.linear_cuts = false;
        self.pieces = false;
        self.shapes = false;
        self.labels = false;
        self.waste = false;
        self.grid = false;
    }
}
