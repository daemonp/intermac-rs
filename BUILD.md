# Building from Source

Instructions for building, testing, and packaging the Intermac glass
cutting toolkit.

---

## Prerequisites

### Rust toolchain

Install **Rust 1.70+** via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux system dependencies

On Linux you need GTK3 and XCB libraries for the GUI viewer:

```bash
# Debian / Ubuntu
sudo apt-get install -y libgtk-3-dev libxcb-render0-dev \
    libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

### macOS / Windows

No extra system dependencies — everything is pulled in by Cargo.

---

## Building

### Debug build (fast iteration)

```bash
cargo build
```

Binaries appear at:

- `target/debug/otd-convert`
- `target/debug/otd-viewer`

### Release build (optimised)

```bash
cargo build --release
```

Binaries appear at:

- `target/release/otd-convert`
- `target/release/otd-viewer`

Release builds use LTO and strip debug symbols (configured in
`Cargo.toml` under `[profile.release]`).

### Build only specific components

```bash
cargo build -p otd-core       # Library only
cargo build -p otd-cli        # CLI only
cargo build -p otd-viewer     # Viewer only
```

---

## Testing

### Run all tests

```bash
cargo test --workspace
```

This runs:

- **87 unit tests** in `otd-core`
- **14 integration tests** in `otd-core/tests/` (end-to-end conversion
  using production fixture files in `tests/fixtures/`)
- **1 doc test** in `otd-core`
- **2 unit tests** in `otd-viewer`

### Run tests for a specific crate

```bash
cargo test -p otd-core
cargo test -p otd-viewer
```

### Test output includes

- OTD parsing (plain and encrypted OTX)
- CNI generation with multiple machine numbers (100–199)
- Shape and piece coordinate transformations
- Schema validation (bounds checking, shape reference resolution)

---

## Code Quality

### Linting (Clippy)

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

The project uses strict Clippy linting — warnings are treated as errors in CI.

Configuration in `clippy.toml`:

- Cognitive complexity threshold: 25
- Max function lines: 100
- Max function arguments: 7

### Formatting

```bash
cargo fmt --all
```

All code must be formatted with `rustfmt` (checked in CI).

---

## Continuous Integration

### CI workflow (`.github/workflows/ci.yml`)

Triggered on push / PR to `main` or `master`. Runs on every commit:

| Job           | What it does                                |
| ------------- | ------------------------------------------- |
| `check`       | `cargo check --all-targets`                 |
| `test`        | `cargo test --all`                          |
| `fmt`         | `cargo fmt --all -- --check`                |
| `clippy`      | `cargo clippy --all-targets -- -D warnings` |
| `build-check` | `cargo build --all-targets` (debug)         |

### Release workflow (`.github/workflows/release.yml`)

Triggered on tags matching `v*`. Produces release binaries:

| Target                     | OS             |
| -------------------------- | -------------- |
| `x86_64-pc-windows-msvc`   | Windows 64-bit |
| `i686-pc-windows-msvc`     | Windows 32-bit |
| `x86_64-unknown-linux-gnu` | Linux 64-bit   |

Artifacts are uploaded to a GitHub Release with auto-generated notes.

---

## Common Issues

### `libgtk-3-dev` not found (Linux)

Make sure your package list is up to date:

```bash
sudo apt-get update
sudo apt-get install -y libgtk-3-dev
```

### Missing OpenSSL (Linux)

The `rfd` crate (file picker) needs OpenSSL development headers:

```bash
sudo apt-get install -y libssl-dev
```

### GUI won't start on Wayland

The viewer uses `eframe` which works on both X11 and Wayland. If you
have issues, try setting:

```bash
export WINIT_UNIX_BACKEND=x11
otd-viewer
```
