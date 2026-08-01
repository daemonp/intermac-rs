# Developer Guide

Architecture, code structure, and file format internals for the Intermac
glass cutting toolkit.

---

## Workspace Layout

```
otd-convert-rs/             # Cargo workspace root
├── Cargo.toml              # Workspace definition (version, members)
├── clippy.toml             # Lint thresholds
│
├── otd-core/               # Shared library (no external binary deps)
├── otd-cli/                # CLI binary — thin wrapper around otd-core
└── otd-viewer/             # GUI binary — eframe/egui application
```

### Crates

| Crate        | Description                                                                                                                         |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| `otd-core`   | Parses OTD/OTX files, validates schemas, generates CNI output with G-code and DXF. Depends only on crypto and serialisation crates. |
| `otd-cli`    | Argument parsing via `clap`, passes input to `otd-core` for conversion.                                                             |
| `otd-viewer` | Cross-platform desktop app using `eframe` + `egui`. Renders cut layouts with pan/zoom and layer toggles.                            |

---

## otd-core Internals

### Source modules

| Module        | Purpose                                                       |
| ------------- | ------------------------------------------------------------- |
| `lib.rs`      | Public API — re-exports everything consumers need.            |
| `config.rs`   | Constants: unit conversions, tool codes, margins, defaults.   |
| `error.rs`    | Error types (`ConvertError` with typed variants).             |
| `model/`      | Data structures — what a cutting layout looks like in memory. |
| `parser/`     | Reads OTD/OTX files and populates the model.                  |
| `generator/`  | Takes the model and writes CNI output.                        |
| `transform/`  | Processes cuts: linear ordering, shape transformations.       |
| `validation/` | Schema-level validation after parsing.                        |

### Data model (`model/`)

```
Schema          — One cutting layout (one sheet)
├── dimensions, unit, machine info
├── pieces       — Each workpiece on the sheet
│   └── Piece    — Position, rotation, shape reference
├── shapes       — Custom contours (rounded corners, notches, etc.)
│   └── Shape    — Sequence of line/arc segments
├── cuts         — All cut segments for this sheet
│   └── Cut      — Single cut (line or arc) with tool, level, rotation
└── piece_types  — Customer/order metadata from [Info] sections
    └── PieceType — Order number, customer, rack, coating flags
```

### Parsing flow

1. **File read** — raw bytes loaded from disk.
2. **OTX decryption** — if the file starts with an OTX magic marker, it's
   decrypted using RC2-CBC with a static key (matching the C# reference
   converter). Decrypted bytes replace the original content for parsing.
3. **Section splitting** — the text is split on `[SectionName]` headers.
4. **Section parsing** — each section is handled by a dedicated parser:
   - `[Header]` → file metadata, dimension unit, version
   - `[Pattern]` → glass sheet dimensions, tool config, then coordinate
     hierarchy entries (X/Y/Z/W/V/A/B/C/D/E lines) defining pieces and cuts
   - `[Shape]` → custom contour definitions (line/arc segments)
   - `[Info]` → order metadata per piece type
   - `[Signature]` → creator info, date
5. **Validation** — parsed schemas are checked for consistency (e.g. shape
   references resolve, piece positions are within sheet bounds).
6. **Result** — `Vec<Schema>` (a file may contain multiple patterns).

### CNI generation flow

1. **Schema → CNI sections** — each schema produces one multi-section CNI:
   - `[COMMENTO]` — file metadata, generator version
   - `[PARAMETRI01]` — sheet dimensions, machine config, units
   - `[UTENSILI01]` — tool definitions referenced by cutting operations
   - `[CONTORNATURA01]` — G-code program (the actual cutting instructions)
   - `[*LDIST...]` — piece distribution metadata
   - `[*PRWB...]` — DXF preview (bottom view, mirrored)
   - `[*PRWC...]` — DXF preview (top view)

2. **G-code generation** — cuts are translated to machine coordinates with
   tool selection, feed rates, and arc interpolation (G02/G03).

3. **DXF generation** — boundary outlines are emitted as DXF primitives for
   the machine's preview display.

---

## File Formats

### OTD input format

INI-style text with section headers. Example:

```ini
[Header]
OTDCutVersion=1.01.00
Dimension=inch
Date=2024-01-15

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

Key points:

- `[Pattern]` contains both flat key=value fields and nested coordinate
  entries (indented X/Y/Z lines forming a tree). Each coordinate line can
  have attributes like `Shape=N` or `Info=N`.
- `[Shape]` entries define contours as a sequence of line and arc segments.
  Each line is `x=<val> y=<val> X=<val> Y=<val>` with optional `R=<radius>`.
- Coordinate variables are: `X`, `Y`, `Z`, `W`, `V`, `A`, `B`, `C`, `D`, `E`.

### OTX encryption

OTX files are OTD files encrypted with RC2 (128-bit key) in CBC mode.
The key is derived from a static passphrase using the
`PasswordDeriveBytes` + `CryptDeriveKey` scheme from .NET Framework,
matching the C# reference converter exactly.

### CNI output structure

The CNI format is specific to Intermac machine controllers and encodes the
full cutting program: sheet geometry, tool assignments, G-code paths for
the cutting head, and DXF outlines for the machine's preview screen.

---

## Library API

Use `otd-core` in your own Rust project:

```toml
[dependencies]
otd-core = { path = "path/to/otd-convert-rs/otd-core" }
```

```rust
use otd_core::{parse_otd_file, convert_otd_to_cni, Schema};
use std::path::Path;

// Parse an OTD file
let schemas: Vec<Schema> = parse_otd_file(Path::new("layout.otd"))?;

// Inspect schemas
for schema in &schemas {
    println!("Sheet: {} x {}", schema.width, schema.height);
    println!("Pieces: {}", schema.pieces.len());
}

// Convert to CNI
let cni_content = convert_otd_to_cni(Path::new("layout.otd"), 130)?;
std::fs::write("output.cni", cni_content)?;
```

### Key public functions

| Function                                   | Description                              |
| ------------------------------------------ | ---------------------------------------- |
| `parse_otd_file(path)`                     | Parse an OTD/OTX file into `Vec<Schema>` |
| `convert_otd_to_cni(path, machine_number)` | Parse and convert to CNI string          |
| `validate_schema(&Schema)`                 | Validate parsed schema                   |
| `parse_otd_bytes(data)`                    | Low-level: parse from a byte slice       |

---

## Publishing a Release

```bash
# Tag the release
git tag -a v1.0.0 -m "v1.0.0"

# Push tags — CI builds release binaries for Linux + Windows (32/64-bit)
# and creates a GitHub Release with generated release notes
git push origin v1.0.0
```

CI release workflow (`.github/workflows/release.yml`):

- Builds: `x86_64-pc-windows-msvc`, `i686-pc-windows-msvc`, `x86_64-unknown-linux-gnu`
- Produces `otd-convert` and `otd-viewer` binaries per target
- Uploads to a GitHub Release with auto-generated release notes

---

## Code Quality

### Clippy configuration (`clippy.toml`)

| Threshold            | Limit |
| -------------------- | ----- |
| Cognitive complexity | 25    |
| Function lines       | 100   |
| Function arguments   | 7     |
