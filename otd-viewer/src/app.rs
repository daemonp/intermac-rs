//! Main application state and eframe integration.

use std::path::PathBuf;

use egui::{CentralPanel, Context, Key, Modifiers, SidePanel, TopBottomPanel, Vec2};
use otd_core::{parse_otd_file, Schema};

use crate::canvas;
use crate::layers::LayerVisibility;
use crate::theme;
use crate::transform::ViewTransform;

/// Main application state.
pub struct ViewerApp {
    /// Loaded schemas (one per pattern in OTD file)
    schemas: Vec<Schema>,
    /// Currently displayed schema index
    current_schema: usize,
    /// Path to the loaded file
    file_path: Option<PathBuf>,

    /// View transformation (pan/zoom)
    transform: ViewTransform,
    /// Layer visibility toggles
    layers: LayerVisibility,

    /// Show the inspector panel
    show_inspector: bool,
    /// Status message
    status_message: String,

    /// Error message to display
    error_message: Option<String>,

    /// Flag to trigger fit-to-window on next frame
    fit_pending: bool,

    /// Current mouse position in sheet coordinates
    mouse_sheet_pos: Option<egui::Pos2>,

    /// Index of piece currently being hovered
    hovered_piece: Option<usize>,

    /// Index of currently selected piece
    selected_piece: Option<usize>,

    /// Show keyboard shortcuts help dialog
    show_shortcuts_dialog: bool,

    /// Show about dialog
    show_about_dialog: bool,

    /// Canvas rect from last frame (for export)
    last_canvas_rect: Option<egui::Rect>,
}

impl ViewerApp {
    /// Create a new viewer application.
    pub fn new(_cc: &eframe::CreationContext<'_>, initial_file: Option<PathBuf>) -> Self {
        let mut app = Self {
            schemas: Vec::new(),
            current_schema: 0,
            file_path: None,
            transform: ViewTransform::default(),
            layers: LayerVisibility::default(),
            show_inspector: true,
            status_message: "No file loaded. Use File > Open or Ctrl+O".to_string(),
            error_message: None,
            fit_pending: false,
            mouse_sheet_pos: None,
            hovered_piece: None,
            selected_piece: None,
            show_shortcuts_dialog: false,
            show_about_dialog: false,
            last_canvas_rect: None,
        };

        // Load initial file if provided
        if let Some(path) = initial_file {
            app.load_file(path);
        }

        app
    }

    /// Load an OTD file.
    fn load_file(&mut self, path: PathBuf) {
        match parse_otd_file(&path) {
            Ok(schemas) => {
                let num_schemas = schemas.len();
                let total_pieces: usize = schemas.iter().map(|s| s.pieces.len()).sum();

                self.schemas = schemas;
                self.current_schema = 0;
                self.file_path = Some(path.clone());
                self.error_message = None;
                self.fit_pending = true;
                self.selected_piece = None;

                self.status_message = format!(
                    "Loaded: {} | {} pattern(s) | {} pieces",
                    path.file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    num_schemas,
                    total_pieces
                );

                tracing::info!("Loaded {} with {} schemas", path.display(), num_schemas);
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load file: {}", e));
                self.status_message = "Error loading file".to_string();
                tracing::error!("Failed to load {}: {}", path.display(), e);
            }
        }
    }

    /// Open file dialog and load selected file.
    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OTD Files", &["otd", "OTD", "otx", "OTX"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            self.load_file(path);
        }
    }

    /// Export current view to PNG.
    fn export_to_png(&mut self) {
        let Some(schema) = self.current_schema() else {
            self.error_message = Some("No file loaded to export".to_string());
            return;
        };

        // Calculate export dimensions based on sheet size
        let scale = 10.0; // 10 pixels per unit (inch)
        let width = (schema.width * scale) as u32;
        let height = (schema.height * scale) as u32;

        // Clamp to reasonable size
        let max_dim = 4096u32;
        let (width, height) = if width > max_dim || height > max_dim {
            let ratio = (max_dim as f64) / (width.max(height) as f64);
            (
                (width as f64 * ratio) as u32,
                (height as f64 * ratio) as u32,
            )
        } else {
            (width, height)
        };

        // Get default filename from loaded file
        let default_name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| {
                format!(
                    "{}_pattern{}.png",
                    s.to_string_lossy(),
                    self.current_schema + 1
                )
            })
            .unwrap_or_else(|| "export.png".to_string());

        // Show save dialog
        let Some(save_path) = rfd::FileDialog::new()
            .add_filter("PNG Image", &["png"])
            .set_file_name(&default_name)
            .save_file()
        else {
            return;
        };

        // Create image buffer
        let mut img = image::RgbaImage::new(width, height);

        // Fill with background color
        let bg = theme::CANVAS_BG;
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([bg.r(), bg.g(), bg.b(), 255]);
        }

        // Create a transform that fits the sheet to the image
        let export_transform = ViewTransform {
            offset: egui::Vec2::new(0.0, 0.0),
            zoom: width as f32 / schema.width as f32,
        };
        let canvas_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(width as f32, height as f32),
        );

        // Render to the image buffer
        self.render_to_image(&mut img, schema, &export_transform, canvas_rect);

        // Save the image
        match img.save(&save_path) {
            Ok(()) => {
                self.status_message = format!("Exported to {}", save_path.display());
                tracing::info!("Exported PNG to {}", save_path.display());
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to save PNG: {}", e));
                tracing::error!("Failed to save PNG: {}", e);
            }
        }
    }

    /// Render schema to an image buffer (software rendering for export).
    fn render_to_image(
        &self,
        img: &mut image::RgbaImage,
        schema: &Schema,
        transform: &ViewTransform,
        canvas_rect: egui::Rect,
    ) {
        let width = img.width() as i32;
        let height = img.height() as i32;

        // Helper to convert sheet coords to image coords
        let to_img = |x: f64, y: f64| -> (i32, i32) {
            let screen =
                transform.sheet_to_screen(egui::Pos2::new(x as f32, y as f32), canvas_rect);
            (screen.x as i32, screen.y as i32)
        };

        // Helper to draw a filled rectangle
        let draw_rect = |img: &mut image::RgbaImage,
                         x1: i32,
                         y1: i32,
                         x2: i32,
                         y2: i32,
                         color: egui::Color32| {
            let (x1, x2) = (x1.min(x2), x1.max(x2));
            let (y1, y2) = (y1.min(y2), y1.max(y2));
            for y in y1.max(0)..y2.min(height) {
                for x in x1.max(0)..x2.min(width) {
                    let pixel = img.get_pixel_mut(x as u32, y as u32);
                    // Alpha blend
                    let alpha = color.a() as f32 / 255.0;
                    let inv_alpha = 1.0 - alpha;
                    pixel[0] = (color.r() as f32 * alpha + pixel[0] as f32 * inv_alpha) as u8;
                    pixel[1] = (color.g() as f32 * alpha + pixel[1] as f32 * inv_alpha) as u8;
                    pixel[2] = (color.b() as f32 * alpha + pixel[2] as f32 * inv_alpha) as u8;
                    pixel[3] = 255;
                }
            }
        };

        // Helper to draw a line using Bresenham's algorithm
        let draw_line = |img: &mut image::RgbaImage,
                         x1: i32,
                         y1: i32,
                         x2: i32,
                         y2: i32,
                         color: egui::Color32| {
            let dx = (x2 - x1).abs();
            let dy = -(y2 - y1).abs();
            let sx = if x1 < x2 { 1 } else { -1 };
            let sy = if y1 < y2 { 1 } else { -1 };
            let mut err = dx + dy;
            let mut x = x1;
            let mut y = y1;

            loop {
                if x >= 0 && x < width && y >= 0 && y < height {
                    let pixel = img.get_pixel_mut(x as u32, y as u32);
                    *pixel = image::Rgba([color.r(), color.g(), color.b(), 255]);
                }
                if x == x2 && y == y2 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y += sy;
                }
            }
        };

        // Draw sheet
        if self.layers.sheet {
            let (x1, y1) = to_img(0.0, 0.0);
            let (x2, y2) = to_img(schema.width, schema.height);
            draw_rect(img, x1, y1, x2, y2, theme::SHEET_FILL);
        }

        // Draw pieces
        if self.layers.pieces {
            for piece in &schema.pieces {
                let (x1, y1) = to_img(piece.x_origin, piece.y_origin);
                let (x2, y2) = to_img(piece.x_origin + piece.width, piece.y_origin + piece.height);
                draw_rect(img, x1, y1, x2, y2, theme::PIECE_FILL);
                // Draw border
                draw_line(img, x1, y1, x2, y1, theme::PIECE_BORDER);
                draw_line(img, x2, y1, x2, y2, theme::PIECE_BORDER);
                draw_line(img, x2, y2, x1, y2, theme::PIECE_BORDER);
                draw_line(img, x1, y2, x1, y1, theme::PIECE_BORDER);
            }
        }

        // Draw linear cuts
        if self.layers.linear_cuts {
            for cut in &schema.linear_cuts {
                if !cut.active {
                    continue;
                }
                let (x1, y1) = to_img(cut.xi, cut.yi);
                let (x2, y2) = to_img(cut.xf, cut.yf);
                draw_line(img, x1, y1, x2, y2, theme::LINEAR_CUT);
            }
        }
    }

    /// Get the current schema, if any.
    fn current_schema(&self) -> Option<&Schema> {
        self.schemas.get(self.current_schema)
    }

    /// Zoom to fit the selected piece in view.
    fn zoom_to_selection(&mut self) {
        let Some(idx) = self.selected_piece else {
            return;
        };
        let Some(canvas_rect) = self.last_canvas_rect else {
            return;
        };

        // Get piece dimensions - need to copy to avoid borrow issues
        let Some(schema) = self.schemas.get(self.current_schema) else {
            return;
        };
        let Some(piece) = schema.pieces.get(idx) else {
            return;
        };
        let piece_width = piece.width;
        let piece_height = piece.height;
        let piece_x = piece.x_origin;
        let piece_y = piece.y_origin;

        // Add some padding around the piece
        let padding_factor = 0.2;
        let padded_width = piece_width * (1.0 + padding_factor * 2.0);
        let padded_height = piece_height * (1.0 + padding_factor * 2.0);

        // Calculate zoom to fit piece
        let zoom_x = canvas_rect.width() as f64 / padded_width;
        let zoom_y = canvas_rect.height() as f64 / padded_height;
        self.transform.zoom =
            (zoom_x.min(zoom_y) as f32).clamp(ViewTransform::MIN_ZOOM, ViewTransform::MAX_ZOOM);

        // Center on piece
        let piece_center_x = piece_x + piece_width / 2.0;
        let piece_center_y = piece_y + piece_height / 2.0;

        self.transform.offset.x =
            canvas_rect.width() / 2.0 - piece_center_x as f32 * self.transform.zoom;
        self.transform.offset.y =
            canvas_rect.height() / 2.0 - piece_center_y as f32 * self.transform.zoom;
    }

    /// Render the menu bar.
    fn render_menu(&mut self, ctx: &Context) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // File menu
                ui.menu_button("File", |ui| {
                    if ui.button("Open... (Ctrl+O)").clicked() {
                        self.open_file_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    let has_schema = self.current_schema().is_some();
                    if ui
                        .add_enabled(has_schema, egui::Button::new("Export PNG... (Ctrl+E)"))
                        .clicked()
                    {
                        self.export_to_png();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit (Ctrl+Q)").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                // View menu
                ui.menu_button("View", |ui| {
                    if ui.button("Fit to Window (F)").clicked() {
                        self.fit_pending = true;
                        ui.close_menu();
                    }
                    if ui.button("Reset View (Home)").clicked() {
                        self.transform.reset();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_inspector, "Inspector Panel");
                });

                // Help menu
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard Shortcuts (?)").clicked() {
                        self.show_shortcuts_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("About").clicked() {
                        self.show_about_dialog = true;
                        ui.close_menu();
                    }
                });
            });
        });
    }

    /// Render the inspector side panel.
    fn render_inspector(&mut self, ctx: &Context) {
        if !self.show_inspector {
            return;
        }

        SidePanel::right("inspector")
            .min_width(200.0)
            .max_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();

                if let Some(schema) = self.current_schema() {
                    // File info
                    ui.collapsing("File Info", |ui| {
                        ui.label(format!("Version: {}", schema.otd_version));
                        ui.label(format!("Unit: {}", schema.unit));
                        ui.label(format!("Date: {}", schema.date));
                        ui.label(format!("Creator: {}", schema.creator));
                    });

                    ui.separator();

                    // Sheet info
                    ui.collapsing("Sheet", |ui| {
                        ui.label(format!(
                            "Dimensions: {:.2} x {:.2}",
                            schema.width, schema.height
                        ));
                        ui.label(format!("Thickness: {:.4}", schema.thickness));
                        ui.label(format!("Glass: {}", schema.glass_description));
                        if schema.trim_left > 0.0 || schema.trim_bottom > 0.0 {
                            ui.label(format!(
                                "Trim: L={:.2} B={:.2}",
                                schema.trim_left, schema.trim_bottom
                            ));
                        }
                    });

                    ui.separator();

                    // Statistics
                    ui.collapsing("Statistics", |ui| {
                        ui.label(format!("Pieces: {}", schema.pieces.len()));
                        ui.label(format!("Linear Cuts: {}", schema.linear_cuts.len()));
                        ui.label(format!("Shapes: {}", schema.shapes.len()));

                        // Calculate utilization
                        let sheet_area = schema.width * schema.height;
                        let piece_area: f64 =
                            schema.pieces.iter().map(|p| p.width * p.height).sum();
                        let utilization = (piece_area / sheet_area) * 100.0;
                        ui.label(format!("Utilization: {:.1}%", utilization));
                    });

                    ui.separator();

                    // Selected piece info
                    if let Some(idx) = self.selected_piece {
                        if let Some(piece) = schema.pieces.get(idx) {
                            ui.heading(format!("Piece #{}", idx + 1));
                            ui.separator();

                            ui.label(format!("Width:  {:.4}\"", piece.width));
                            ui.label(format!("Height: {:.4}\"", piece.height));
                            ui.label(format!("Area:   {:.2} sq in", piece.width * piece.height));

                            ui.separator();
                            ui.label(format!(
                                "Position: ({:.2}\", {:.2}\")",
                                piece.x_origin, piece.y_origin
                            ));

                            if piece.shape_index.is_some() {
                                ui.label("Has custom shape");
                            }

                            ui.separator();
                            if ui.button("Clear Selection (Esc)").clicked() {
                                self.selected_piece = None;
                            }

                            ui.separator();
                        }
                    }
                }

                // Layer controls
                ui.collapsing("Layers", |ui| {
                    ui.checkbox(&mut self.layers.sheet, "Sheet (1)");
                    ui.checkbox(&mut self.layers.trim, "Trim Zone");
                    ui.checkbox(&mut self.layers.linear_cuts, "Linear Cuts (2)");
                    ui.checkbox(&mut self.layers.pieces, "Pieces (3)");
                    ui.checkbox(&mut self.layers.shapes, "Shapes (4)");
                    ui.checkbox(&mut self.layers.labels, "Labels (5)");
                    ui.checkbox(&mut self.layers.waste, "Waste Regions (6)");
                    ui.checkbox(&mut self.layers.grid, "Grid (7)");
                });

                ui.separator();

                // View info
                ui.label(format!("Zoom: {}", self.transform.zoom_percent()));
            });
    }

    /// Render the status bar.
    fn render_status_bar(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(self.transform.zoom_percent());

                    // Show mouse coordinates if we have a schema
                    if self.current_schema().is_some() {
                        if let Some(pos) = &self.mouse_sheet_pos {
                            ui.separator();
                            ui.label(format!("({:.2}, {:.2})", pos.x, pos.y));
                        }
                    }

                    // Schema navigation
                    if self.schemas.len() > 1 {
                        ui.separator();

                        // Next button (drawn first due to right-to-left layout)
                        let next_enabled = self.current_schema < self.schemas.len() - 1;
                        if ui
                            .add_enabled(next_enabled, egui::Button::new("▶"))
                            .on_hover_text("Next pattern (Page Down)")
                            .clicked()
                        {
                            self.current_schema += 1;
                            self.fit_pending = true;
                            self.selected_piece = None;
                        }

                        // Schema indicator
                        ui.label(format!(
                            "{} / {}",
                            self.current_schema + 1,
                            self.schemas.len()
                        ));

                        // Previous button
                        let prev_enabled = self.current_schema > 0;
                        if ui
                            .add_enabled(prev_enabled, egui::Button::new("◀"))
                            .on_hover_text("Previous pattern (Page Up)")
                            .clicked()
                        {
                            self.current_schema -= 1;
                            self.fit_pending = true;
                            self.selected_piece = None;
                        }

                        ui.separator();
                        ui.label("Pattern:");
                    }
                });
            });
        });
    }

    /// Render the main canvas.
    fn render_canvas(&mut self, ctx: &Context) {
        CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::CANVAS_BG))
            .show(ctx, |ui| {
                // Handle keyboard shortcuts
                self.handle_keyboard(ctx);

                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
                let canvas_rect = response.rect;

                // Store canvas rect for zoom_to_selection
                self.last_canvas_rect = Some(canvas_rect);

                // Handle fit-to-window
                if self.fit_pending {
                    if let Some(schema) = self.current_schema() {
                        self.transform
                            .fit_to_sheet(schema.width, schema.height, canvas_rect);
                    }
                    self.fit_pending = false;
                }

                // Handle pan (middle mouse or right mouse drag)
                if response.dragged_by(egui::PointerButton::Middle)
                    || response.dragged_by(egui::PointerButton::Secondary)
                {
                    self.transform.pan(response.drag_delta());
                }

                // Handle zoom (scroll wheel)
                let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
                if scroll_delta.y != 0.0 {
                    if let Some(hover_pos) = response.hover_pos() {
                        let factor = if scroll_delta.y > 0.0 {
                            ViewTransform::ZOOM_FACTOR
                        } else {
                            1.0 / ViewTransform::ZOOM_FACTOR
                        };
                        self.transform.zoom_at(hover_pos, canvas_rect, factor);
                    }
                }

                // Track mouse position in sheet coordinates
                self.mouse_sheet_pos = response
                    .hover_pos()
                    .map(|screen_pos| self.transform.screen_to_sheet(screen_pos, canvas_rect));

                // Detect hovered piece
                self.hovered_piece = None;
                if let (Some(sheet_pos), Some(schema)) =
                    (self.mouse_sheet_pos, self.current_schema())
                {
                    for (i, piece) in schema.pieces.iter().enumerate() {
                        if sheet_pos.x >= piece.x_origin as f32
                            && sheet_pos.x <= (piece.x_origin + piece.width) as f32
                            && sheet_pos.y >= piece.y_origin as f32
                            && sheet_pos.y <= (piece.y_origin + piece.height) as f32
                        {
                            self.hovered_piece = Some(i);
                            break;
                        }
                    }
                }

                // Handle click to select piece
                if response.clicked() {
                    self.selected_piece = self.hovered_piece;
                }

                // Render schema if loaded
                if let Some(schema) = self.current_schema() {
                    canvas::render_schema(
                        &painter,
                        schema,
                        &self.transform,
                        canvas_rect,
                        &self.layers,
                        self.hovered_piece,
                        self.selected_piece,
                    );
                } else {
                    // Show placeholder text
                    painter.text(
                        canvas_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "No file loaded\n\nUse File > Open or Ctrl+O",
                        egui::FontId::proportional(20.0),
                        theme::DIM_TEXT,
                    );
                }
            });
    }

    /// Handle keyboard shortcuts.
    fn handle_keyboard(&mut self, ctx: &Context) {
        ctx.input(|i| {
            // Ctrl+O: Open file
            if i.modifiers.ctrl && i.key_pressed(Key::O) {
                self.open_file_dialog();
            }

            // Ctrl+Q: Quit
            if i.modifiers.ctrl && i.key_pressed(Key::Q) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Ctrl+E: Export PNG
            if i.modifiers.ctrl && i.key_pressed(Key::E) {
                self.export_to_png();
            }

            // ?: Show keyboard shortcuts
            if i.key_pressed(Key::Questionmark) || (i.modifiers.shift && i.key_pressed(Key::Slash))
            {
                self.show_shortcuts_dialog = true;
            }

            // F: Fit to window
            if i.key_pressed(Key::F) && i.modifiers == Modifiers::NONE {
                self.fit_pending = true;
            }

            // Home: Reset view
            if i.key_pressed(Key::Home) {
                self.transform.reset();
                self.fit_pending = true;
            }

            // +/=: Zoom in
            if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) {
                self.transform.zoom *= ViewTransform::ZOOM_FACTOR;
            }

            // -: Zoom out
            if i.key_pressed(Key::Minus) {
                self.transform.zoom /= ViewTransform::ZOOM_FACTOR;
            }

            // Page Up/Down: Navigate schemas
            if self.schemas.len() > 1 {
                if i.key_pressed(Key::PageUp) && self.current_schema > 0 {
                    self.current_schema -= 1;
                    self.fit_pending = true;
                    self.selected_piece = None;
                }
                if i.key_pressed(Key::PageDown) && self.current_schema < self.schemas.len() - 1 {
                    self.current_schema += 1;
                    self.fit_pending = true;
                    self.selected_piece = None;
                }
            }

            // Escape: Clear selection
            if i.key_pressed(Key::Escape) {
                self.selected_piece = None;
                self.show_shortcuts_dialog = false;
                self.show_about_dialog = false;
            }

            // Z: Zoom to selection
            if i.key_pressed(Key::Z) && i.modifiers == Modifiers::NONE {
                self.zoom_to_selection();
            }

            // Number keys for layer toggles
            if i.key_pressed(Key::Num1) {
                self.layers.sheet = !self.layers.sheet;
            }
            if i.key_pressed(Key::Num2) {
                self.layers.linear_cuts = !self.layers.linear_cuts;
            }
            if i.key_pressed(Key::Num3) {
                self.layers.pieces = !self.layers.pieces;
            }
            if i.key_pressed(Key::Num4) {
                self.layers.shapes = !self.layers.shapes;
            }
            if i.key_pressed(Key::Num5) {
                self.layers.labels = !self.layers.labels;
            }
            if i.key_pressed(Key::Num6) {
                self.layers.waste = !self.layers.waste;
            }
            if i.key_pressed(Key::Num7) {
                self.layers.grid = !self.layers.grid;
            }
        });
    }

    /// Show error dialog if there's an error.
    fn show_error_dialog(&mut self, ctx: &Context) {
        if let Some(error) = self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(&error);
                    ui.separator();
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
                });
        }
    }

    /// Show keyboard shortcuts help dialog.
    fn show_shortcuts_help(&mut self, ctx: &Context) {
        if !self.show_shortcuts_dialog {
            return;
        }

        egui::Window::new("Keyboard Shortcuts")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.strong("File");
                        ui.end_row();
                        ui.label("Ctrl+O");
                        ui.label("Open file");
                        ui.end_row();
                        ui.label("Ctrl+E");
                        ui.label("Export PNG");
                        ui.end_row();
                        ui.label("Ctrl+Q");
                        ui.label("Quit");
                        ui.end_row();

                        ui.end_row();
                        ui.strong("Navigation");
                        ui.end_row();
                        ui.label("Scroll");
                        ui.label("Zoom in/out");
                        ui.end_row();
                        ui.label("Middle/Right drag");
                        ui.label("Pan view");
                        ui.end_row();
                        ui.label("F");
                        ui.label("Fit to window");
                        ui.end_row();
                        ui.label("Home");
                        ui.label("Reset view");
                        ui.end_row();
                        ui.label("+/-");
                        ui.label("Zoom in/out");
                        ui.end_row();
                        ui.label("Z");
                        ui.label("Zoom to selection");
                        ui.end_row();
                        ui.label("Page Up/Down");
                        ui.label("Previous/next pattern");
                        ui.end_row();

                        ui.end_row();
                        ui.strong("Selection");
                        ui.end_row();
                        ui.label("Click");
                        ui.label("Select piece");
                        ui.end_row();
                        ui.label("Escape");
                        ui.label("Clear selection");
                        ui.end_row();

                        ui.end_row();
                        ui.strong("Layers");
                        ui.end_row();
                        ui.label("1");
                        ui.label("Toggle sheet");
                        ui.end_row();
                        ui.label("2");
                        ui.label("Toggle linear cuts");
                        ui.end_row();
                        ui.label("3");
                        ui.label("Toggle pieces");
                        ui.end_row();
                        ui.label("4");
                        ui.label("Toggle shapes");
                        ui.end_row();
                        ui.label("5");
                        ui.label("Toggle labels");
                        ui.end_row();
                        ui.label("6");
                        ui.label("Toggle waste regions");
                        ui.end_row();
                        ui.label("7");
                        ui.label("Toggle grid");
                        ui.end_row();
                    });

                ui.separator();
                if ui.button("Close").clicked() {
                    self.show_shortcuts_dialog = false;
                }
            });
    }

    /// Show about dialog.
    fn show_about(&mut self, ctx: &Context) {
        if !self.show_about_dialog {
            return;
        }

        egui::Window::new("About OTD Viewer")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading("OTD Viewer");
                ui.label("Version 0.1.0");
                ui.separator();
                ui.label("A viewer for Intermac glass cutting layouts.");
                ui.label("");
                ui.label("Built with egui/eframe");
                ui.separator();
                if ui.button("Close").clicked() {
                    self.show_about_dialog = false;
                }
            });
    }

    /// Show tooltip for hovered piece.
    fn show_piece_tooltip(&self, ctx: &Context) {
        let Some(idx) = self.hovered_piece else {
            return;
        };
        // Don't show tooltip if piece is selected (info in inspector)
        if self.selected_piece == Some(idx) {
            return;
        }
        let Some(schema) = self.current_schema() else {
            return;
        };
        let Some(piece) = schema.pieces.get(idx) else {
            return;
        };

        let piece_width = piece.width;
        let piece_height = piece.height;
        let has_shape = piece.shape_index.is_some();

        egui::show_tooltip_at_pointer(
            ctx,
            egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("piece_tooltip_layer")),
            egui::Id::new("piece_tooltip"),
            |ui| {
                ui.strong(format!("Piece #{}", idx + 1));
                ui.label(format!("{:.2}\" × {:.2}\"", piece_width, piece_height));
                ui.label(format!("Area: {:.2} sq in", piece_width * piece_height));
                if has_shape {
                    ui.label("Has custom shape");
                }
            },
        );
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.render_menu(ctx);
        self.render_inspector(ctx);
        self.render_status_bar(ctx);
        self.render_canvas(ctx);
        self.show_error_dialog(ctx);
        self.show_shortcuts_help(ctx);
        self.show_about(ctx);
        self.show_piece_tooltip(ctx);

        // Only repaint when there's actual interaction, not continuously
        // This prevents high CPU usage and "not responding" issues when unfocused
        if ctx.input(|i| {
            i.pointer.is_moving() || i.pointer.any_down() || i.raw_scroll_delta != egui::Vec2::ZERO
        }) {
            ctx.request_repaint();
        }
    }
}
