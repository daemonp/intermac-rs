# OTD Cut List Viewer Specification

A cross-platform (Windows + Linux/Wayland) GUI application for visualizing OTD glass cutting layouts.

## Purpose

The viewer provides a visual representation of OTD cut files before they are sent to Intermac glass cutting machines. It enables operators and engineers to:

1. **Verify layouts** - Confirm piece placement, dimensions, and cut paths before production
2. **Inspect shapes** - Examine complex contours (arcs, curves) that define non-rectangular pieces
3. **Review cut sequences** - Understand the order and hierarchy of linear and shape cuts
4. **Debug issues** - Identify problems with layouts that may cause machine errors

## Data Model

The viewer visualizes a `Schema` which represents a complete cutting layout for one glass sheet.

### Primary Entities

| Entity | Description | Visual Representation |
|--------|-------------|----------------------|
| **Glass Sheet** | Base material (`width` x `height`) with optional trim borders | Grey rectangle with dashed trim lines |
| **Linear Cuts** | Straight cut lines (vertical, horizontal, oblique) | Red lines |
| **Pieces** | Rectangular workpieces with position and dimensions | Blue outlined rectangles with labels |
| **Shapes** | Custom contours made of line and arc segments | Cyan filled regions |
| **Waste/Scrap** | Areas that will be discarded | Grey dashed regions |

### Entity Relationships

```
Schema
├── linear_cuts: Vec<Cut>        # Grid cuts that divide the sheet
├── pieces: Vec<Piece>           # Individual workpieces
│   └── shape_index → shapes[]   # Optional custom contour
│   └── piece_type_index → piece_types[]  # Optional order info
├── shapes: Vec<Shape>           # Contour definitions
│   └── cuts: Vec<Cut>           # Lines and arcs forming the shape
└── piece_types: Vec<PieceType>  # Customer/order metadata
```

### Cut Geometry Types

```rust
enum CutType {
    Line,    // Straight segment: (xi, yi) → (xf, yf)
    ArcCW,   // Clockwise arc: (xi, yi) → (xf, yf), center (xc, yc), radius
    ArcCCW,  // Counter-clockwise arc: same as above
}
```

---

## User Interface Layout

```
┌────────────────────────────────────────────────────────────────────────┐
│  File  View  Help                                           [─][□][×] │
├────────────────────────────────────────────────────────────────────────┤
│ ┌────────────────────────────────────────────────────┐ ┌─────────────┐ │
│ │                                                    │ │  INSPECTOR  │ │
│ │                                                    │ │─────────────│ │
│ │                                                    │ │ File Info   │ │
│ │                                                    │ │ ─────────── │ │
│ │                     CANVAS                         │ │ Version: .. │ │
│ │                                                    │ │ Unit: inch  │ │
│ │              (Glass Sheet View)                    │ │ Date: ...   │ │
│ │                                                    │ │             │ │
│ │                                                    │ │ Sheet       │ │
│ │                                                    │ │ ─────────── │ │
│ │                                                    │ │ 129.5 x 95.5│ │
│ │                                                    │ │ Trim: 0.5   │ │
│ │                                                    │ │             │ │
│ │                                                    │ │ Selection   │ │
│ │                                                    │ │ ─────────── │ │
│ │                                                    │ │ (none)      │ │
│ │                                                    │ │             │ │
│ └────────────────────────────────────────────────────┘ │ Layers      │ │
│ ┌────────────────────────────────────────────────────┐ │ ─────────── │ │
│ │ STATUS: Loaded: example.otd | 15 pieces | 23 cuts  │ │ [x] Sheet   │ │
│ └────────────────────────────────────────────────────┘ │ [x] Linear  │ │
│                                                        │ [x] Pieces  │ │
│                                                        │ [x] Shapes  │ │
│                                                        │ [ ] Labels  │ │
│                                                        │ [ ] Waste   │ │
│                                                        └─────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

### Components

#### 1. Menu Bar
- **File**: Open, Recent Files, Export PNG, Exit
- **View**: Fit to Window (F), Zoom In (+), Zoom Out (-), Reset View (Home)
- **Help**: About, Keyboard Shortcuts

#### 2. Canvas (Main View)
The central canvas displays the glass sheet and all cuts/pieces with:
- Infinite pan and zoom
- Coordinate system matching OTD (origin at bottom-left)
- Grid overlay (optional, toggleable)
- Ruler/scale indicator

#### 3. Inspector Panel (Right Sidebar)
Contextual information panel showing:
- **File Info**: OTD version, creation date, unit of measurement
- **Sheet Info**: Dimensions, glass type, thickness, coating flags
- **Selection Info**: Details of hovered/selected entity
- **Statistics**: Total pieces, cuts, shapes, utilization percentage

#### 4. Layer Controls
Checkboxes to show/hide visual layers:
- Sheet bounds and trim
- Linear cuts
- Pieces (bounding boxes)
- Shapes (contours)
- Labels (piece IDs, order info)
- Waste regions
- Cut sequence numbers

#### 5. Status Bar
- Current file name
- Summary statistics
- Mouse coordinates in sheet units
- Zoom level percentage

---

## Interaction Design

### Navigation

| Action | Input | Description |
|--------|-------|-------------|
| Pan | Middle-mouse drag | Move view without changing zoom |
| Pan | Right-mouse drag | Alternative pan method |
| Zoom | Scroll wheel | Zoom in/out centered on cursor |
| Zoom | Ctrl + Scroll | Fine zoom control |
| Fit to Window | F key | Fit entire sheet in view with padding |
| Reset View | Home key | Return to default zoom/position |
| Zoom to Selection | Z key | Zoom to fit selected entity |

### Selection

| Action | Input | Description |
|--------|-------|-------------|
| Hover | Mouse move | Highlight entity under cursor |
| Select | Left-click | Select entity, show details in inspector |
| Clear Selection | Escape | Deselect current selection |
| Multi-select | Ctrl + Left-click | Add/remove from selection |

### Hover Behavior

When the mouse hovers over an entity:
1. Entity is highlighted (thicker stroke, glow effect)
2. Tooltip appears showing basic info:
   - **Piece**: Index, dimensions, position, order info
   - **Cut**: Type, coordinates, length
   - **Shape**: Name, description, perimeter

### Selection Behavior

When an entity is selected:
1. Entity rendered with selection style (highlighted border)
2. Inspector panel shows full details
3. If piece has a shape, shape contour is emphasized
4. Related cuts are subtly highlighted

---

## Rendering Specification

### Coordinate System

```
OTD Coordinate System:
- Origin: Bottom-left corner of sheet
- X: Increases rightward
- Y: Increases upward
- Units: As specified in file (inch or mm)

Screen Coordinate System:
- Origin: Top-left of canvas
- X: Increases rightward  
- Y: Increases downward

Transformation: Y-axis flip required
```

### View Transform

```rust
struct ViewTransform {
    offset: (f64, f64),  // Pan offset in screen pixels
    zoom: f64,           // Zoom level (1.0 = 100%)
}

impl ViewTransform {
    /// Convert sheet coordinates to screen coordinates
    fn sheet_to_screen(&self, x: f64, y: f64, canvas_height: f64) -> (f32, f32) {
        let sx = (x * self.zoom + self.offset.0) as f32;
        let sy = (canvas_height - y * self.zoom - self.offset.1) as f32;
        (sx, sy)
    }
    
    /// Convert screen coordinates to sheet coordinates
    fn screen_to_sheet(&self, sx: f32, sy: f32, canvas_height: f64) -> (f64, f64) {
        let x = (sx as f64 - self.offset.0) / self.zoom;
        let y = (canvas_height - sy as f64 - self.offset.1) / self.zoom;
        (x, y)
    }
}
```

### Color Palette

| Element | Color (Hex) | RGBA |
|---------|-------------|------|
| Background | `#1e1e1e` | `(30, 30, 30, 255)` |
| Sheet fill | `#2d4a3e` | `(45, 74, 62, 255)` |
| Sheet border | `#4a7c6a` | `(74, 124, 106, 255)` |
| Trim zone | `#3d3d3d` | `(61, 61, 61, 128)` |
| Linear cut | `#e74c3c` | `(231, 76, 60, 255)` |
| Piece border | `#3498db` | `(52, 152, 219, 255)` |
| Piece fill | `#2980b9` | `(41, 128, 185, 64)` |
| Shape contour | `#1abc9c` | `(26, 188, 156, 255)` |
| Shape fill | `#16a085` | `(22, 160, 133, 96)` |
| Waste | `#7f8c8d` | `(127, 140, 141, 128)` |
| Selection | `#f1c40f` | `(241, 196, 15, 255)` |
| Hover | `#ffffff` | `(255, 255, 255, 128)` |
| Label text | `#ecf0f1` | `(236, 240, 241, 255)` |
| Grid lines | `#3d3d3d` | `(61, 61, 61, 128)` |

### Rendering Layers (Bottom to Top)

1. **Background** - Solid dark canvas background
2. **Grid** - Optional coordinate grid (when zoomed in)
3. **Sheet** - Glass sheet rectangle with fill
4. **Trim Zone** - Semi-transparent overlay on trim borders
5. **Waste Regions** - Dashed grey areas (optional)
6. **Linear Cuts** - Red cut lines with appropriate stroke width
7. **Pieces** - Blue rectangles with borders
8. **Shapes** - Cyan contours for non-rectangular pieces
9. **Labels** - Text labels for pieces (optional)
10. **Hover Highlight** - Glow effect on hovered entity
11. **Selection Highlight** - Bold border on selected entity

### Drawing Primitives

#### Line Cut
```rust
fn draw_line_cut(painter: &Painter, cut: &Cut, transform: &ViewTransform, style: CutStyle) {
    let p1 = transform.sheet_to_screen(cut.xi, cut.yi);
    let p2 = transform.sheet_to_screen(cut.xf, cut.yf);
    
    painter.line_segment(
        [p1.into(), p2.into()],
        Stroke::new(style.width, style.color)
    );
}
```

#### Arc Cut
```rust
fn draw_arc_cut(painter: &Painter, cut: &Cut, transform: &ViewTransform, style: CutStyle) {
    // Approximate arc with line segments for rendering
    let segments = 32; // Increase for smoother arcs
    let start_angle = (cut.yi - cut.yc).atan2(cut.xi - cut.xc);
    let end_angle = (cut.yf - cut.yc).atan2(cut.xf - cut.xc);
    
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let angle = interpolate_angle(start_angle, end_angle, t, cut.cut_type);
        let x = cut.xc + cut.radius * angle.cos();
        let y = cut.yc + cut.radius * angle.sin();
        points.push(transform.sheet_to_screen(x, y));
    }
    
    painter.add(PathShape::line(points, Stroke::new(style.width, style.color)));
}
```

#### Shape Contour
```rust
fn draw_shape(painter: &Painter, shape: &Shape, piece: &Piece, transform: &ViewTransform) {
    let mut path = Vec::new();
    
    for cut in &shape.cuts {
        // Translate cut coordinates by piece origin
        let translated_cut = translate_cut(cut, piece.x_origin, piece.y_origin);
        
        match cut.cut_type {
            CutType::Line => {
                path.push(transform.sheet_to_screen(translated_cut.xi, translated_cut.yi));
                path.push(transform.sheet_to_screen(translated_cut.xf, translated_cut.yf));
            }
            CutType::ArcCW | CutType::ArcCCW => {
                // Tessellate arc into line segments
                path.extend(tessellate_arc(&translated_cut, transform));
            }
        }
    }
    
    // Draw filled shape
    painter.add(PathShape::convex_polygon(path.clone(), SHAPE_FILL, Stroke::NONE));
    
    // Draw contour
    painter.add(PathShape::line(path, Stroke::new(2.0, SHAPE_STROKE)));
}
```

---

## Multi-Schema Support

An OTD file may contain multiple schemas (layouts). The viewer supports this via:

### Schema Navigation

```
┌──────────────────────────────────────────────────┐
│  ◀  Schema 1 of 3: Layout_001  ▶                 │
└──────────────────────────────────────────────────┘
```

- **Left/Right arrows**: Navigate between schemas
- **Schema selector**: Dropdown to jump to specific schema
- **Keyboard**: Page Up/Down to cycle schemas

### Schema List Panel (Optional)

A collapsible panel showing all schemas:
```
┌─ Schemas ────────────────┐
│ ▸ Layout_001 (15 pieces) │
│   Layout_002 (12 pieces) │
│   Layout_003 (8 pieces)  │
└──────────────────────────┘
```

---

## Inspector Panel Details

### File Info Section
```
┌─ File ────────────────────────┐
│ Version:  1.01.00             │
│ Unit:     inch                │
│ Date:     2025/06/27 05:53    │
│ Creator:  XCAWCUT 1.40        │
│ Machine:  Master 33           │
└───────────────────────────────┘
```

### Sheet Info Section
```
┌─ Sheet ───────────────────────┐
│ Dimensions: 129.50 × 95.50 in │
│ Trim:       L: 0.5  B: 0.5    │
│ Glass:      3mm 1/8" (#126)   │
│ Thickness:  0.126 in          │
│ Coating:    Low-E             │
│ Structured: No                │
└───────────────────────────────┘
```

### Selection Info - Piece
```
┌─ Selected Piece ──────────────┐
│ Index:      #3                │
│ Position:   (28.19, 35.22)    │
│ Size:       14.09 × 21.13     │
│ Shape:      Form99 (Rotated)  │
│                               │
│ ─── Order Info ───            │
│ Order:      623512            │
│ Position:   0                 │
│ Customer:   LIPPERT COMP...   │
│ Commission: P2044             │
│ Rack:       1                 │
└───────────────────────────────┘
```

### Selection Info - Cut
```
┌─ Selected Cut ────────────────┐
│ Type:       Vertical Line     │
│ From:       (28.19, 0.50)     │
│ To:         (28.19, 95.00)    │
│ Length:     94.50 in          │
│ Level:      1                 │
│ Pieces:     #1, #2, #3        │
└───────────────────────────────┘
```

### Statistics Section
```
┌─ Statistics ──────────────────┐
│ Pieces:       15              │
│ Linear Cuts:  23              │
│ Shapes:       2               │
│                               │
│ Total Area:   12,377.25 in²   │
│ Used Area:    10,892.50 in²   │
│ Utilization:  88.0%           │
└───────────────────────────────┘
```

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+O` | Open file |
| `Ctrl+E` | Export as PNG |
| `Ctrl+Q` | Quit |
| `F` | Fit to window |
| `Home` | Reset view |
| `+` / `=` | Zoom in |
| `-` | Zoom out |
| `Z` | Zoom to selection |
| `Escape` | Clear selection |
| `1` | Toggle sheet layer |
| `2` | Toggle linear cuts layer |
| `3` | Toggle pieces layer |
| `4` | Toggle shapes layer |
| `5` | Toggle labels layer |
| `6` | Toggle waste layer |
| `Page Up` | Previous schema |
| `Page Down` | Next schema |
| `?` | Show keyboard shortcuts |

---

## Technical Architecture

### Crate Structure

```
otd-convert-rs/
├── Cargo.toml              # Workspace root
├── otd-core/               # Shared library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── model/          # Schema, Cut, Piece, Shape, PieceType
│       ├── parser/         # OTD/OTX parsing
│       ├── config.rs       # Units, machine config
│       └── error.rs        # Error types
├── otd-cli/                # CLI tool
│   ├── Cargo.toml
│   └── src/
│       └── main.rs         # CLI entry point (convert command)
└── otd-viewer/             # GUI application
    ├── Cargo.toml
    └── src/
        ├── main.rs         # App entry point
        ├── app.rs          # Main App struct, eframe integration
        ├── canvas.rs       # Canvas widget, rendering
        ├── transform.rs    # ViewTransform, coordinate math
        ├── inspector.rs    # Inspector panel
        ├── layers.rs       # Layer visibility state
        ├── selection.rs    # Selection/hover state
        └── theme.rs        # Colors, styles
```

### Dependencies

```toml
# otd-viewer/Cargo.toml
[package]
name = "otd-viewer"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "otd-viewer"
path = "src/main.rs"

[dependencies]
otd-core = { path = "../otd-core" }
eframe = "0.29"              # egui framework with native backend
egui = "0.29"                # Immediate mode GUI
egui_extras = "0.29"         # Extra widgets (tables, etc.)
rfd = "0.15"                 # Native file dialogs
tracing = "0.1"              # Logging
tracing-subscriber = "0.3"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
# Native-only deps if needed
```

### Application State

```rust
pub struct ViewerApp {
    // Data
    schemas: Vec<Schema>,
    current_schema: usize,
    file_path: Option<PathBuf>,
    
    // View state
    transform: ViewTransform,
    layers: LayerVisibility,
    
    // Interaction state
    selection: Selection,
    hover: Option<EntityId>,
    
    // UI state
    show_inspector: bool,
    show_schema_list: bool,
}

pub struct ViewTransform {
    offset: egui::Vec2,
    zoom: f32,
}

pub struct LayerVisibility {
    sheet: bool,
    trim: bool,
    linear_cuts: bool,
    pieces: bool,
    shapes: bool,
    labels: bool,
    waste: bool,
    grid: bool,
}

pub enum Selection {
    None,
    Piece(usize),
    Cut(usize),
    Shape(usize),
}

pub enum EntityId {
    Piece(usize),
    Cut(usize),
    Shape(usize),
}
```

### Rendering Flow

```rust
impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Menu bar
        self.render_menu(ctx);
        
        // 2. Side panel (Inspector)
        egui::SidePanel::right("inspector")
            .show(ctx, |ui| self.render_inspector(ui));
        
        // 3. Status bar
        egui::TopBottomPanel::bottom("status")
            .show(ctx, |ui| self.render_status(ui));
        
        // 4. Central canvas
        egui::CentralPanel::default()
            .show(ctx, |ui| self.render_canvas(ui));
    }
}

impl ViewerApp {
    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag()
        );
        
        // Handle input
        self.handle_canvas_input(&response);
        
        // Get current schema
        let Some(schema) = self.schemas.get(self.current_schema) else {
            return;
        };
        
        // Render layers
        self.render_background(&painter, &response.rect);
        
        if self.layers.sheet {
            self.render_sheet(&painter, schema);
        }
        if self.layers.trim {
            self.render_trim(&painter, schema);
        }
        if self.layers.linear_cuts {
            self.render_linear_cuts(&painter, schema);
        }
        if self.layers.pieces {
            self.render_pieces(&painter, schema);
        }
        if self.layers.shapes {
            self.render_shapes(&painter, schema);
        }
        if self.layers.labels {
            self.render_labels(&painter, schema);
        }
        
        // Render interaction overlays
        self.render_hover(&painter, schema);
        self.render_selection(&painter, schema);
    }
}
```

---

## Implementation Phases

### Phase 1: Workspace Refactoring
- [ ] Create Cargo workspace structure
- [ ] Extract `otd-core` crate with models and parser
- [ ] Create `otd-cli` crate with existing CLI functionality
- [ ] Verify all tests pass after refactoring

### Phase 2: Basic Viewer Scaffold
- [ ] Initialize `otd-viewer` crate with eframe
- [ ] Implement file open dialog
- [ ] Basic window with empty canvas
- [ ] Load and store Schema from OTD file

### Phase 3: Canvas Rendering
- [ ] Implement ViewTransform (zoom/pan math)
- [ ] Render glass sheet rectangle
- [ ] Render linear cuts as lines
- [ ] Render rectangular pieces
- [ ] Implement pan (middle-mouse drag)
- [ ] Implement zoom (scroll wheel)
- [ ] Fit-to-window (F key)

### Phase 4: Shape Rendering
- [ ] Render line segments in shapes
- [ ] Render arc segments (tessellation)
- [ ] Translate shape coordinates to piece position
- [ ] Handle shape fill and stroke

### Phase 5: Interaction
- [ ] Hit testing for hover detection
- [ ] Highlight hovered entity
- [ ] Click to select
- [ ] Inspector panel with selection details

### Phase 6: Polish
- [ ] Layer toggle controls
- [ ] Labels rendering
- [ ] Multi-schema navigation
- [ ] Export to PNG
- [ ] Keyboard shortcuts
- [ ] Performance optimization for large layouts

---

## Performance Considerations

### Large Layouts
- Layouts may have 100+ pieces and 500+ cuts
- Use spatial indexing (quadtree) for hit testing if needed
- Cull entities outside visible area
- Cache tessellated arcs

### Rendering Optimization
- egui repaints only when needed (input events, animations)
- Avoid allocations in render loop
- Pre-compute screen coordinates when zoom/pan changes
- Use `PathShape` batching for multiple cuts

### Memory
- Keep only one Schema fully loaded at a time
- Lazy-load schemas from multi-schema files
- Release textures when not visible

---

## Future Enhancements (Out of Scope for MVP)

- **Animation**: Show cut sequence as animation
- **Measurement Tool**: Click two points to measure distance
- **Comparison View**: Side-by-side two OTD files
- **Edit Mode**: Modify piece positions (write back to OTD)
- **Print Layout**: Generate printable cut list report
- **3D Preview**: Show glass thickness in 3D view
- **WebAssembly**: Run viewer in browser
