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

    /// Get the current schema, if any.
    fn current_schema(&self) -> Option<&Schema> {
        self.schemas.get(self.current_schema)
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
                    if ui.button("About").clicked() {
                        // TODO: About dialog
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
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.render_menu(ctx);
        self.render_inspector(ctx);
        self.render_status_bar(ctx);
        self.render_canvas(ctx);
        self.show_error_dialog(ctx);
    }
}
