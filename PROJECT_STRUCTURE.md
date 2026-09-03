# X-Native Project Structure

## Clean Repository Layout

This repository contains **one project**: X-Native Designer, a native design tool built with Rust.

```
X-Native/
├── apps/                      # Application binaries
│   └── x-designer/            # Main design application
│       └── src/
│           └── bin/
│               ├── x_native_app/    # Windowed editor (main product)
│               │   ├── app.rs       # Application state & rendering
│               │   ├── chrome.rs    # UI drawing
│               │   ├── icons.rs     # Icon library (Ink & Azure v3)
│               │   ├── main.rs      # Entry point
│               │   ├── run.rs       # Event loop
│               │   ├── state.rs     # Editor state
│               │   ├── helpers.rs   # Drawing utilities
│               │   ├── theme.rs     # Ink & Azure design tokens
│               │   └── demo.rs      # Demo content
│               ├── x_native.rs   # CLI automation tool
│               ├── render_headless.rs # GPU test renderer
│               ├── bench_scale.rs   # Performance benchmarks
│               ├── export_regression.rs # Export tests
│               ├── type_proof.rs    # Typography tests
│               └── mklib.rs         # Library creation tool
└── crates/                    # Library crates
    ├── x-core/                # Core data models (Node, Document, AutoLayout)
    ├── x-editor/              # Editor logic (commands, selection, snapping)
    ├── x-render/              # Vello scene generation & rendering
    ├── x-text/                # Text shaping & font caching
    ├── x-components/          # Component system & variants
    ├── x-format/              # File I/O (.x, .xlib, SVG, PDF, Sketch)
    ├── x-native/              # Native windowing (x_native facade crate)
    └── x-ui/                  # UI widget primitives
```

## What Was Removed

The following dead/orphaned code has been cleaned up:

- ❌ Root `src/` directory (pre-split monolith)
- ❌ Unused `apps/x-designer/src/ui/` modules (unwired modular UI experiment)
- ❌ Empty placeholder files under `bin/x_native_app/ui/`
- ❌ Documentation sprawl (overlapping markdown planning documents)
- ❌ Test artifacts (*.x, *.autosave, *.png, *.svg files in root)
- ❌ Old `docs/` directory with outdated specs

## Active Code Only

| Location | Lines (measured 2026-09-02) | Status |
|----------|-------|--------|
| `crates/*` | ~19,200 | ✅ Active, compiled |
| `apps/x-designer/src/bin/x_native_app/` | ~7,600 | ✅ Active, main product |
| `apps/x-designer/src/bin/*.rs` | ~950 | ✅ Active, tools/tests |
| **Total** | **~27,800** | ✅ All compiled |

(`cargo test --workspace`: 295 tests green as of the 2026-09-02 render-IR
cleanup — transparent-fill and frame-clip regressions fixed, goldens
re-pinned. See CI.)

## Build Targets

```bash
# Main editor (requires display; Linux needs libgtk-3-dev for rfd)
cargo build -p x-designer --bin x_native_app

# CLI automation
cargo build -p x-designer --bin x_native

# Headless renderer
cargo build -p x-designer --bin render_headless

# All tools
cargo build --workspace
```

## Next Steps

1. ~~Fix failing tests~~ ✅ done 2026-09-02 (render IR cleanup + golden re-pin)
2. **Complete VectorNetwork** — Either implement the renderer or remove the variant
3. **macOS packaging** — Add signing, notarization, DMG creation
4. **Release profile** — Optimize binary sizes (currently 80-250MB debug)
5. **Adopt rustfmt or the current compact style deliberately** — CI reports
   `cargo fmt --check` as advisory until a dedicated formatting commit lands
6. **Clippy pass** — done: workspace is clippy-clean and enforced in CI; 7 documented `#[allow(too_many_arguments)]` exceptions where positional params are the natural shape
