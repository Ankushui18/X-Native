# X-Native Designer

A native, offline-first design tool built with Rust.

## Features

- **Native Performance**: GPU-accelerated rendering with Vello/wgpu
- **Complete Editor**: Vector editing, auto-layout, components, variables
- **File Interoperability**: `.x` format, SVG/PNG/PDF export, Sketch/Figma JSON import
- **Design Systems**: Components with variants, variables, styles, libraries
- **Professional Tools**: Pen tool, boolean operations, constraints, prototyping

## Project Structure

```
X-Native/
├── apps/
│   └── x-designer/          # Main application
│       └── src/
│           └── bin/
│               ├── x_native_app/    # Windowed editor (chrome.rs + app.rs)
│               ├── arco_native.rs   # CLI automation
│               └── render_headless.rs # GPU rendering tests
└── crates/
    ├── x-core/              # Document model, geometry, layout
    ├── x-editor/            # Editor logic, commands, selection
    ├── x-render/            # Vello scene generation
    ├── x-text/              # Text shaping, font cache
    ├── x-components/        # Component system
    ├── x-format/            # File I/O (.x, .xlib, SVG, PDF)
    ├── x-native/            # Native windowing (arco_native)
    └── x-ui/                # UI primitives
```

## Building

```bash
# Build all workspace members
cargo build --workspace

# Build release binaries
cargo build --release -p x-designer --bin x_native_app

# Run the editor (requires display/GPU)
cargo run --release -p x-designer --bin x_native_app

# Run headless renderer test
cargo run --release -p x-designer --bin render_headless
```

## Running Tests

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p x-core
cargo test -p x-editor
```

## Binary Outputs

| Binary | Size (debug) | Purpose |
|--------|--------------|---------|
| `x_native_app` | ~250 MB | Main windowed editor |
| `arco_native` | ~105 MB | CLI automation/scripting |
| `render_headless` | ~127 MB | GPU PNG rendering tests |

## System Requirements

- **OS**: macOS 10.15+, Windows 10+, Linux (with X11/Wayland)
- **GPU**: Vulkan, Metal, or OpenGL 4.3+ support
- **Display**: Required for `x_native_app` (headless mode available for rendering)

## File Formats

### Native Format
- `.x` - X-Native document format
- `.xlib` - Component/style library format

### Import
- Sketch (`.sketch`) - Partial support
- Figma JSON (REST API export) - Interoperability only
- SVG - High fidelity import

### Export
- SVG, PNG, PDF - Full fidelity
- Figma-compatible JSON - For round-trip workflows

## Legal & Trademarks

X-Native is an independent design tool. "Figma" is a trademark of Figma, Inc. References to Figma are for interoperability purposes only. X-Native does not parse proprietary `.fig` binary files.

## License

[Add your license here]

## Status

**Beta** - Core editor functional, UI polish in progress.

See individual crate READMEs for detailed feature documentation.
