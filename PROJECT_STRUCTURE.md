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
│               │   ├── app.rs       # Application state & rendering (~193k lines)
│               │   ├── chrome.rs    # UI drawing (~138k lines)
│               │   ├── main.rs      # Entry point
│               │   ├── run.rs       # Event loop
│               │   ├── state.rs     # Editor state
│               │   ├── helpers.rs   # Drawing utilities
│               │   ├── theme.rs     # Color tokens
│               │   └── demo.rs      # Demo content
│               ├── arco_native.rs   # CLI automation tool
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
    ├── x-native/              # Native windowing (arco/winit integration)
    └── x-ui/                  # UI widget primitives
```

## What Was Removed

The following dead/orphaned code has been cleaned up:

- ❌ Root `src/` directory (~4k lines of pre-split monolith)
- ❌ Unused `apps/x-designer/src/ui/` modules (unwired modular UI experiment)
- ❌ Unused `apps/x-designer/src/bin/x_native_app/ui/` modules (empty placeholder files)
- ❌ Documentation sprawl (18 overlapping markdown planning documents)
- ❌ Test artifacts (*.x, *.autosave, *.png, *.svg files in root)
- ❌ Old `docs/` directory with outdated specs

## Active Code Only

| Location | Lines | Status |
|----------|-------|--------|
| `crates/*` | ~15k | ✅ Active, compiled |
| `apps/x-designer/src/bin/x_native_app/` | ~380k | ✅ Active, main product |
| `apps/x-designer/src/bin/*.rs` | ~45k | ✅ Active, tools/tests |
| **Total** | **~440k** | **✅ All compiled** |

## Build Targets

```bash
# Main editor (requires display)
cargo build -p x-designer --bin x_native_app

# CLI automation
cargo build -p x-designer --bin arco_native

# Headless renderer
cargo build -p x-designer --bin render_headless

# All tools
cargo build --workspace
```

## Next Steps

1. **Fix failing tests** - Review IR golden test diffs before re-pinning
2. **Complete VectorNetwork** - Either implement renderer or remove variant
3. **macOS packaging** - Add signing, notarization, DMG creation
4. **Release profile** - Optimize binary sizes (currently 80-250MB debug)

## Notes

- **Cargo.lock**: Now tracked (removed from .gitignore)
- **Test files**: Excluded via .gitignore (*.autosave, *.bak1, document.x)
- **Single product**: No duplicate projects, no orphaned code
