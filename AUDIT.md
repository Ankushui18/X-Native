# X-Native Architecture & UI Audit
**Foundation:** `X-Native-design-reset.zip` only  
**Date:** 2026-09-03  
**Scope:** Engine capabilities vs discarded UI vs new product brief

---

## A. Existing engine capabilities (preserve)

| Domain | Crate | Status |
|--------|-------|--------|
| Document model (Node, pages, kinds) | `x-core` | Working |
| Auto Layout / layout types | `x-core`, `x-components` | Working |
| Variables, modes, collections | `x-core` | Working |
| Components, instances, overrides | `x-core`, `x-components` | Working |
| Pins / constraints | `x-editor` | Working |
| Selection, hit-test, undo/redo | `x-editor` | Working |
| Align / distribute | `x-editor` | Working |
| Snapping, spatial index | `x-editor` | Working |
| Vector edit anchors | `x-editor` | Working |
| Boolean ops | `x-editor` / `x-core` | Working |
| Prototype player | `x-editor` | Working |
| Dev-mode CSS-style export | `x-editor` | Working |
| Vello scene build, IR, cache | `x-render` | Working |
| Text shaping / fonts | `x-text` | Working |
| `.x` / v2 serialize, SVG, PNG, Sketch, Figma JSON | `x-format` | Working |
| Libraries / xlib | `x-format` | Working |
| Retained UI primitives (widgets, theme, a11y roles) | `x-ui` | Present, underused |
| Facade | `x-native` | Working |
| Headless render / CLI tools | `x-designer` bins | Working |

**Tests:** Workspace test suite exists under crates (historically ~295 green).

---

## B. Existing UI capabilities (discard)

Per `DESIGN_RESET.md`, intentionally **removed**:

- Entire `x_native_app` shell (chrome, theme, icons, demo, app state)
- Previous design-system documents (Ink & Azure, etc.)
- Bundled demo assets / fixture images

**Do not restore** old chrome geometry, colors, icon set, or panel layout.

---

## C. Missing UI capabilities (must create)

| Area | Priority |
|------|----------|
| App shell + native window loop | P0 |
| Canvas viewport (pan/zoom + scene) | P0 |
| Tool rail (contextual, grouped) | P0 |
| Layers + pages panel | P0 |
| Contextual inspector | P0 |
| Status bar | P0 |
| Command palette (⌘/Ctrl+K) | P0 |
| Theme tokens + icon system | P0 |
| First-launch / home | P1 |
| Context menus | P1 |
| Auto Layout UX surface | P1 |
| Components / variables / assets workspaces | P2 |
| Prototype mode chrome | P2 |
| Export UI | P2 |

---

## D. Architecture problems to avoid

1. **Monolithic chrome.rs** — paint + hit-test + business logic in one file  
2. **Hardcoded hex in paint paths** — all chrome via design tokens  
3. **Page root listed as a layer** — page ≠ layer  
4. **Import always creating pages** — SVG/PNG → current page  
5. **Inspector showing every property always** — selection-driven sections only  
6. **Website-in-a-window feel** — dense native chrome, not dashboard cards  

**Target structure:**

```
x_native_app/
  main.rs          entry
  run.rs           winit + GPU loop
  state.rs         document + UI state
  theme.rs         tokens only
  layout.rs        region geometry
  paint.rs         primitive draws
  icons.rs         stroke icon set
  shell.rs         AppShell composition
  canvas.rs        viewport
  tools.rs         tool rail
  layers.rs        pages + layers
  inspector.rs     contextual properties
  command.rs       palette
  status.rs        status bar
  home.rs          first launch
```

---

## E. Figma comparison (patterns, not clone)

| Workflow | Figma pattern to learn | X-Native approach |
|----------|------------------------|-------------------|
| Empty page | Blank canvas, page props | Same; page not in layer list |
| Tools | Compact + shortcuts | Tool rail + ⌘K, not every tool always visible |
| Inspector | Selection-driven | Mandatory progressive disclosure |
| Layers | Pages above tree | Pages + Layers, clear hierarchy |
| Multiplayer | Core | Out of scope for Phase 1 |
| Plugins | Ecosystem | Later |

---

## F. Sketch comparison

| Sketch strength | Apply how |
|-----------------|-----------|
| Desktop-native density | Compact rows, real menus |
| Symbols/libraries discipline | Dedicated library workspace |
| Local-first file model | `.x` + recent files home |

---

## G. X-Native opportunity

1. **Design ↔ code** already in engine — surface in Inspect tab without a separate “Dev Mode product”  
2. **Native performance** narrative — honest GPU canvas, not Electron  
3. **Command-first discovery** — beginners learn via ⌘K, not tours  
4. **Calm canvas** — almost no permanent overlays  
5. **Own visual language** — Graphite & Signal (see DESIGN_SYSTEM.md), not purple-Figma or orange legacy  

---

## Implementation order (locked)

1. Phase 1 — Foundation (shell, canvas, tools, layers, inspector, status, palette, theme)  
2. Phase 2 — Core design tools wired to engine  
3. Phase 3 — Auto Layout UX  
4. Phase 4 — Systems (components, variables, assets)  
5. Phase 5 — Prototype chrome  
6. Phase 6 — Production (export UI, a11y, packaging)
