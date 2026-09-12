# X-Native Professional UI

This package contains a clean native desktop workspace for X-Native. The shell uses the existing Rust, Winit, WGPU, Vello, and X-Native text stack; no web view or browser UI is introduced.

## Product direction

- Native decorated window on macOS, Windows, and Linux
- Inter Regular is resolved explicitly for application chrome through `SystemFonts`; a safe system fallback is used only when Inter is unavailable
- Platform-aware `Command K` / `Ctrl K` labels and input
- Calm Graphite surfaces with a single Framer-inspired Electric blue accent
- Compact 24–34 px control density suited to a professional design tool
- Responsive 216/240 px navigator and 264/288 px inspector
- Canvas-first hierarchy with quiet chrome and high-contrast selection
- Contextual inspector states for page and object selection
- Layer search, page/layer hierarchy, libraries, variables, plugins, export, and auto-layout entry points
- Command palette plus V/F/T/R tool shortcuts
- Custom Vello stroke icons; emoji glyphs are not used for tools
- Clean viewport at normal zoom; the 1 x 1 document-pixel grid appears only from 800% zoom
- Original centered Graphite command capsule, 232 px navigator, and 296 px contextual inspector
- Neutral editable desktop and mobile frames keep attention on the editor UI rather than a showcase design
- Figma-style eight-handle selection bounds, live size badge, center snapping guides, and named frame labels
- Framer-style floating creation toolbar and compact Preview/Share actions

## Important source

- `apps/x-designer/src/bin/x_native_app/main.rs` — native window bootstrap
- `apps/x-designer/src/bin/x_native_app/chrome.rs` — shell, interaction, platform adaptation, and Vello rendering

## Run

```bash
cargo run -p x-designer --bin x_native_app
```

## Design notes

The current pass intentionally focuses on the desktop product shell and visual interaction model. Existing engine crates remain unchanged, so the next implementation pass can bind the visible inspector fields, layers, assets, exports, and toolbar actions to the existing editor APIs without rewriting the interface.
