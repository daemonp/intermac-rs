# otd-convert-rs

A Rust CLI tool that converts OTD (Optimized Tool Data) files to CNI (CNC ISO) format for Intermac glass cutting machines.

## Overview

This tool parses cutting optimization data and generates machine-ready G-code with:

- Linear and shaped cut paths
- DXF visualizations for operator preview
- Tool configurations and machine parameters

## Installation

```bash
# Build from source
cargo build --release

# Binary location: target/release/otd-convert
```

**Requirements:** Rust 1.70+

## Usage

```bash
# Basic conversion
otd-convert -i input.otd -o output.cni

# Specify machine number
otd-convert -i input.otd -o output.cni -m 130

# Validate without generating output
otd-convert -i input.otd --validate

# Verbose output for debugging
otd-convert -i input.otd -o output.cni -v
```

### CLI Options

| Option | Description |
|--------|-------------|
| `-i, --input <FILE>` | Input OTD/OTX file path (required) |
| `-o, --output <FILE>` | Output CNI file path |
| `-m, --machine <NUM>` | Machine number, 100-199 (default: 130) |
| `--validate` | Validate input only, skip generation |
| `--debug` | Output parsed data as JSON |
| `-v, --verbose` | Enable verbose logging |

## Building & Testing

```bash
# Run all tests (87 unit + 14 integration)
cargo test

# Run with output visible
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_tests

# Check code quality
cargo clippy

# Format code
cargo fmt
```

## Project Structure

```
src/
  main.rs           # CLI entry point
  lib.rs            # Library exports
  config.rs         # Constants (tool codes, margins)
  error.rs          # Error types

  model/            # Data structures
    schema.rs       # Pattern/layout container
    piece.rs        # Individual glass piece
    shape.rs        # Custom shape definition
    cut.rs          # Cut segment (line/arc)

  parser/           # OTD parsing
    otd.rs          # Main parser
    sections.rs     # Section handlers
    otx.rs          # Encrypted file decryption

  generator/        # CNI generation
    cni.rs          # Main generator
    gcode.rs        # G-code output
    dxf.rs          # DXF visualization

  transform/        # Processing
    linear.rs       # Cut path ordering
    shapes.rs       # Shape transformations

  validation/       # Input validation
    validate.rs     # Schema validator

tests/
  integration_tests.rs        # End-to-end tests
  fixtures/integration/       # Test OTD/CNI pairs
```

## File Formats

### OTD Input

INI-style text format with cutting optimization data:

```ini
[Header]
OTDCutVersion=3.0
Dimension=in

[Pattern]
MachineNumber=130
Width=129.5
Height=95.5
GlassThickness=0.125

[Piece1]
Quantity=1
Width=48
Height=36
Customer=ACME Corp

[Shape1]
Description=Curved Corner
x=0 y=0 X=10 Y=0
x=10 y=0 X=10 Y=10 R=5
```

### CNI Output

Machine-ready format with multiple sections:

| Section | Purpose |
|---------|---------|
| `[COMMENTO]` | File metadata |
| `[PARAMETRI01]` | Sheet dimensions, machine config |
| `[UTENSILI01]` | Tool definitions |
| `[CONTORNATURA01]` | G-code cutting program |
| `[*LDIST...]` | Piece metadata |
| `[*PRWB...]` | DXF preview (bottom view) |
| `[*PRWC...]` | DXF preview (top view) |

## License

MIT
