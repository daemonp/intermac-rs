# otd-convert-rs

A Rust toolkit for working with OTD (Optimized Tool Data) files for Intermac glass cutting machines. Includes a CLI converter and a cross-platform GUI viewer.

## Overview

This workspace provides tools for parsing cutting optimization data and generating machine-ready output:

- **otd-core**: Shared library for OTD parsing and CNI generation
- **otd-cli**: Command-line converter (OTD → CNI)
- **otd-viewer**: Cross-platform GUI viewer for cut layouts

### Features

- Parse OTD/OTX files (including RC2-encrypted OTX)
- Generate CNI output with G-code and DXF visualizations
- Visual preview of glass sheets, cuts, pieces, and shapes
- Pan/zoom navigation with layer controls

## Installation

```bash
# Build all tools
cargo build --release

# Binaries location:
#   target/release/otd-convert   (CLI)
#   target/release/otd-viewer    (GUI)
```

**Requirements:** Rust 1.70+

## Quick Start

### CLI Converter

```bash
# Basic conversion
otd-convert -i input.otd -o output.cni

# Specify machine number
otd-convert -i input.otd -o output.cni -m 130

# Validate without generating output
otd-convert -i input.otd --validate

# Debug: output parsed data as JSON
otd-convert -i input.otd --debug
```

### GUI Viewer

```bash
# Open viewer (then use File > Open)
otd-viewer

# Open with file directly
otd-viewer path/to/layout.otd
```

---

## CLI Reference (`otd-convert`)

```bash
otd-convert [OPTIONS] -i <FILE>
```

### Options

| Option | Description |
|--------|-------------|
| `-i, --input <FILE>` | Input OTD/OTX file path (required) |
| `-o, --output <FILE>` | Output CNI file path (default: input with .cni extension) |
| `-m, --machine <NUM>` | Machine number, 100-199 (default: 130) |
| `--validate` | Validate input only, skip generation |
| `--debug` | Output parsed data as JSON |
| `-v, --verbose` | Enable verbose logging |

### Examples

```bash
# Convert with default settings
otd-convert -i layout.otd

# Convert encrypted OTX file
otd-convert -i layout.otx -o output.cni

# Validate multiple files
for f in *.otd; do otd-convert -i "$f" --validate; done
```

---

## GUI Viewer (`otd-viewer`)

A cross-platform viewer for inspecting OTD cut layouts before sending to the machine.

### Features

- **Visual Layers**: Sheet, trim zones, linear cuts, pieces, shapes, labels
- **Navigation**: Pan (middle/right mouse), zoom (scroll wheel)
- **Inspector Panel**: File info, sheet dimensions, statistics
- **Multi-Schema**: Navigate between patterns with Page Up/Down

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+O` | Open file |
| `Ctrl+Q` | Quit |
| `F` | Fit to window |
| `Home` | Reset view |
| `+` / `-` | Zoom in/out |
| `1` | Toggle sheet layer |
| `2` | Toggle linear cuts |
| `3` | Toggle pieces |
| `4` | Toggle shapes |
| `5` | Toggle labels |
| `Page Up/Down` | Navigate schemas |

### Screenshot

```
┌─────────────────────────────────────────────────────────┐
│ File  View  Help                              OTD Viewer│
├─────────────────────────────────────────────┬───────────┤
│                                             │ Inspector │
│     ┌─────────────────────────────┐         │───────────│
│     │  ┌───────┐  ┌───────┐      │         │ Sheet     │
│     │  │ Piece │  │ Piece │      │         │ 129.5x95.5│
│     │  │  #1   │  │  #2   │      │         │           │
│     │  └───────┘  └───────┘      │         │ Pieces: 7 │
│     │  ────────────────────      │         │ Cuts: 12  │
│     │       (linear cut)         │         │           │
│     └─────────────────────────────┘         │ [x] Sheet │
│              (glass sheet)                  │ [x] Cuts  │
├─────────────────────────────────────────────┴───────────┤
│ Loaded: layout.otd | 1 pattern | 7 pieces    Zoom: 85% │
└─────────────────────────────────────────────────────────┘
```

---

## Building & Testing

```bash
# Run all tests (104 total)
cargo test --workspace

# Run specific crate tests
cargo test -p otd-core          # 87 unit + 14 integration + 1 doc
cargo test -p otd-viewer        # 2 unit tests

# Check code quality
cargo clippy --workspace

# Format code
cargo fmt --all

# Build release binaries
cargo build --release --workspace
```

---

## Project Structure

```
otd-convert-rs/
├── Cargo.toml                    # Workspace definition
│
├── otd-core/                     # Shared library
│   ├── src/
│   │   ├── lib.rs                # Public API exports
│   │   ├── config.rs             # Constants (tools, margins, units)
│   │   ├── error.rs              # Error types (ConvertError)
│   │   │
│   │   ├── model/                # Data structures
│   │   │   ├── schema.rs         # Complete cutting layout
│   │   │   ├── piece.rs          # Individual glass workpiece
│   │   │   ├── piece_type.rs     # Customer/order metadata
│   │   │   ├── shape.rs          # Custom contour definition
│   │   │   └── cut.rs            # Cut segment (line/arc)
│   │   │
│   │   ├── parser/               # OTD/OTX parsing
│   │   │   ├── otd.rs            # Main parser, OTX decryption
│   │   │   └── sections.rs       # Section handlers
│   │   │
│   │   ├── generator/            # Output generation
│   │   │   ├── cni.rs            # CNI file generator
│   │   │   ├── gcode.rs          # G-code writer
│   │   │   └── dxf.rs            # DXF visualization
│   │   │
│   │   ├── transform/            # Cut processing
│   │   │   ├── linear.rs         # Linear cut ordering
│   │   │   └── shapes.rs         # Shape transformations
│   │   │
│   │   └── validation/           # Input validation
│   │       └── validate.rs       # Schema validator
│   │
│   └── tests/
│       ├── integration_tests.rs  # End-to-end conversion tests
│       └── fixtures/             # Test OTD/CNI pairs
│
├── otd-cli/                      # CLI tool
│   └── src/
│       └── main.rs               # Argument parsing, conversion
│
└── otd-viewer/                   # GUI viewer
    └── src/
        ├── main.rs               # Entry point, eframe setup
        ├── app.rs                # Application state, UI orchestration
        ├── canvas.rs             # Schema rendering
        ├── transform.rs          # Coordinate transformation (pan/zoom)
        ├── layers.rs             # Layer visibility state
        └── theme.rs              # Colors and styling
```

---

## File Formats

### OTD Input

INI-style text format with cutting optimization data:

```ini
[Header]
AWCutVersion=1.01.00
Dimension=inch

[Pattern]
GlassID=126
Width=129.500000
Height=95.500000
GlassThickness=0.125984
X=28.187500
  Y=14.093750
    Z=21.125000 Shape=1 Info=1

[Shape]
Id=1
Description=Form99
x=0 y=0 X=10 Y=0
x=10 y=0 X=10 Y=10 R=5

[Info]
Id=1
OrderNo=623512
Customer=ACME Corp
```

### CNI Output

Machine-ready format with multiple sections:

| Section | Purpose |
|---------|---------|
| `[COMMENTO]` | File metadata, creator info |
| `[PARAMETRI01]` | Sheet dimensions, machine config |
| `[UTENSILI01]` | Tool definitions |
| `[CONTORNATURA01]` | G-code cutting program |
| `[*LDIST...]` | Piece distribution metadata |
| `[*PRWB...]` | DXF preview (bottom view) |
| `[*PRWC...]` | DXF preview (top/mirrored view) |

---

## Library Usage

Use `otd-core` in your own Rust projects:

```toml
[dependencies]
otd-core = { path = "path/to/otd-convert-rs/otd-core" }
```

```rust
use otd_core::{parse_otd_file, convert_otd_to_cni, Schema};
use std::path::Path;

// Parse an OTD file
let schemas: Vec<Schema> = parse_otd_file(Path::new("layout.otd"))?;

// Access schema data
for schema in &schemas {
    println!("Sheet: {} x {}", schema.width, schema.height);
    println!("Pieces: {}", schema.pieces.len());
    println!("Shapes: {}", schema.shapes.len());
}

// Convert to CNI
let cni_content = convert_otd_to_cni(Path::new("layout.otd"), 130)?;
std::fs::write("output.cni", cni_content)?;
```

---

## License

MIT
