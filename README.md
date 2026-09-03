# X-Native Designer

Native design tool (Rust + Vello/wgpu). **Greenfield UI** on a preserved engine.

**Visual language:** Graphite & Signal (see `DESIGN_SYSTEM.md`)  
**Audit:** `AUDIT.md` · **Layout:** `LAYOUT_SPEC.md` · **Reset note:** `DESIGN_RESET.md`

## Foundation (engine)

- Document model, Auto Layout, components, variables (`x-core`)
- Selection, undo, constraints, prototype (`x-editor`)
- GPU scene pipeline (`x-render`)
- Typography (`x-text`)
- `.x` / SVG / Sketch / Figma JSON (`x-format`)

## UI (new)

Phase 1 shell under `apps/x-designer/src/bin/x_native_app/`:

- App shell, home, tool rail, pages + layers, contextual inspector, status, command palette
- Does **not** inherit previous chrome, colors, or icons

## Build

```bash
sudo apt-get install -y libgtk-3-dev   # Linux dialogs
cargo build --release -p x-designer --bin x_native_app
cargo run --release -p x-designer --bin x_native_app
```

## Status

| Area | State |
|------|--------|
| Engine | Preserved |
| Old UI | Removed by design-reset |
| Phase 1 shell | Scaffolded — Graphite & Signal |
| Phases 2–6 | Per `AUDIT.md` implementation order |
