//! View transformation for converting between sheet and screen coordinates.

#![allow(dead_code)] // Some methods reserved for future features

use egui::{Pos2, Vec2};

/// Transformation state for pan and zoom.
#[derive(Debug, Clone)]
pub struct ViewTransform {
    /// Pan offset in screen pixels
    pub offset: Vec2,
    /// Zoom level (1.0 = 100%, 2.0 = 200%, etc.)
    pub zoom: f32,
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl ViewTransform {
    /// Minimum zoom level
    pub const MIN_ZOOM: f32 = 0.1;
    /// Maximum zoom level
    pub const MAX_ZOOM: f32 = 50.0;
    /// Zoom factor per scroll step
    pub const ZOOM_FACTOR: f32 = 1.1;

    /// Convert sheet coordinates to screen coordinates.
    ///
    /// The sheet coordinate system has origin at bottom-left with Y increasing upward.
    /// The screen coordinate system has origin at top-left with Y increasing downward.
    pub fn sheet_to_screen(&self, sheet_pos: Pos2, canvas_rect: egui::Rect) -> Pos2 {
        let x = sheet_pos.x * self.zoom + self.offset.x + canvas_rect.min.x;
        let y = canvas_rect.max.y - (sheet_pos.y * self.zoom + self.offset.y);
        Pos2::new(x, y)
    }

    /// Convert screen coordinates to sheet coordinates.
    pub fn screen_to_sheet(&self, screen_pos: Pos2, canvas_rect: egui::Rect) -> Pos2 {
        let x = (screen_pos.x - canvas_rect.min.x - self.offset.x) / self.zoom;
        let y = (canvas_rect.max.y - screen_pos.y - self.offset.y) / self.zoom;
        Pos2::new(x, y)
    }

    /// Zoom in/out centered on a screen position.
    pub fn zoom_at(&mut self, screen_pos: Pos2, canvas_rect: egui::Rect, factor: f32) {
        let old_zoom = self.zoom;
        self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);

        if (self.zoom - old_zoom).abs() > f32::EPSILON {
            // Adjust offset to keep the point under cursor fixed
            let sheet_pos = Pos2::new(
                (screen_pos.x - canvas_rect.min.x - self.offset.x) / old_zoom,
                (canvas_rect.max.y - screen_pos.y - self.offset.y) / old_zoom,
            );

            self.offset.x = screen_pos.x - canvas_rect.min.x - sheet_pos.x * self.zoom;
            self.offset.y = canvas_rect.max.y - screen_pos.y - sheet_pos.y * self.zoom;
        }
    }

    /// Pan the view by a screen-space delta.
    pub fn pan(&mut self, delta: Vec2) {
        self.offset.x += delta.x;
        self.offset.y -= delta.y; // Invert Y because screen Y is flipped
    }

    /// Fit the view to show the entire sheet with some padding.
    pub fn fit_to_sheet(&mut self, sheet_width: f64, sheet_height: f64, canvas_rect: egui::Rect) {
        let canvas_width = canvas_rect.width();
        let canvas_height = canvas_rect.height();

        // Calculate zoom to fit sheet in canvas with padding
        let padding = 40.0;
        let available_width = canvas_width - padding * 2.0;
        let available_height = canvas_height - padding * 2.0;

        let zoom_x = available_width / sheet_width as f32;
        let zoom_y = available_height / sheet_height as f32;
        self.zoom = zoom_x.min(zoom_y).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);

        // Center the sheet in the canvas
        let sheet_screen_width = sheet_width as f32 * self.zoom;
        let sheet_screen_height = sheet_height as f32 * self.zoom;

        self.offset.x = (canvas_width - sheet_screen_width) / 2.0;
        self.offset.y = (canvas_height - sheet_screen_height) / 2.0;
    }

    /// Reset to default view.
    pub fn reset(&mut self) {
        self.offset = Vec2::ZERO;
        self.zoom = 1.0;
    }

    /// Get the zoom level as a percentage string.
    pub fn zoom_percent(&self) -> String {
        format!("{:.0}%", self.zoom * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sheet_to_screen_origin() {
        let transform = ViewTransform::default();
        let canvas = egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        // Sheet origin (0, 0) should map to bottom-left of canvas
        let screen = transform.sheet_to_screen(Pos2::new(0.0, 0.0), canvas);
        assert!((screen.x - 0.0).abs() < 0.001);
        assert!((screen.y - 600.0).abs() < 0.001);
    }

    #[test]
    fn test_roundtrip_conversion() {
        let transform = ViewTransform {
            offset: Vec2::new(50.0, 30.0),
            zoom: 2.0,
        };
        let canvas = egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let original = Pos2::new(100.0, 50.0);
        let screen = transform.sheet_to_screen(original, canvas);
        let back = transform.screen_to_sheet(screen, canvas);

        assert!((original.x - back.x).abs() < 0.001);
        assert!((original.y - back.y).abs() < 0.001);
    }
}
