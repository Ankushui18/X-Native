# X-Native — Full App Feature Roadmap

From current state (v0.3: scene graph + Vello encoding PoC, ~330 LOC) to a complete
Figma-class native design tool. Ordered by dependency: each phase builds on the previous.

Legend: ✅ = exists (even if minimal) · 🟡 = partial/stub · ❌ = missing

---

## Phase 0 — Foundation Hardening (before any new features)

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 0.1 | Upgrade `vello 0.1 → 0.4+`, `wgpu 0.19 → 24+` | ❌ | Do this FIRST — API churn grows with every feature added on old deps |
| 0.2 | Replace `&'static str` variable/component names with owned `String`/interned IDs | ❌ | Required for any loaded document |
| 0.3 | Stable node IDs (UUID/u64), `HashMap<NodeId, Node>` arena instead of owned child `Vec`s | ❌ | Needed for selection, undo, multiplayer later; current tree can't reference nodes |
| 0.4 | `rustfmt` + `clippy` + CI (build/test on push) | ❌ | Code-golf style will not survive multiple contributors |
| 0.5 | Error handling (`Result`, no `.expect()` in library paths) | ❌ | |

## Phase 1 — Windowed App Shell (the stated next milestone)

| # | Feature | Status |
|---|---------|--------|
| 1.1 | `winit` window + event loop | ❌ |
| 1.2 | `wgpu` surface + live Vello renderer (swapchain, resize, vsync) | ❌ |
| 1.3 | Render loop with dirty-region redraw (dirty tracking exists 🟡 — wire it up) | 🟡 |
| 1.4 | Infinite canvas: pan (space+drag / middle mouse / trackpad), zoom-to-cursor, zoom-to-fit | ❌ |
| 1.5 | Viewport culling wired to live camera (helper exists ✅) | 🟡 |
| 1.6 | HiDPI / multi-monitor scale handling | ❌ |
| 1.7 | Cursor management (default, move, resize handles, crosshair, text I-beam) | ❌ |

## Phase 2 — Core Editing (what makes it an *editor*)

| # | Feature | Status |
|---|---------|--------|
| 2.1 | Hit testing (point→node, respecting transforms, z-order, groups/frames) | ❌ |
| 2.2 | Selection: click, shift-click, marquee/rubber-band, select-inside (double-click into groups) | ❌ |
| 2.3 | Move / resize / rotate with on-canvas handles; modifier keys (shift = constrain, alt = from-center/duplicate) | ❌ |
| 2.4 | **Undo/redo** — command pattern or immutable document snapshots. Design this EARLY; retrofitting is brutal | ❌ |
| 2.5 | Shape creation tools: frame (F), rect (R), ellipse (O), line (L), polygon/star | 🟡 (model exists, no tools) |
| 2.6 | Pen tool / editable vector networks (bezier paths, point editing, boolean ops: union/subtract/intersect/exclude) | ❌ |
| 2.7 | Copy / paste / duplicate / delete; paste-in-place | ❌ |
| 2.8 | Z-order operations (bring forward/back, front/back) | ❌ |
| 2.9 | Group / ungroup, frame selection | 🟡 (model only) |
| 2.10 | Snapping: to pixel grid, to other objects (smart guides), to layout grids; alignment red-lines | ❌ |
| 2.11 | Align & distribute (left/center/right, tidy-up) | ❌ |
| 2.12 | Constraints (pin left/right/center/scale when parent resizes) | ❌ |
| 2.13 | Keyboard shortcuts system (configurable map) | ❌ |

## Phase 3 — Text (its own subsystem; do not underestimate)

| # | Feature | Status |
|---|---------|--------|
| 3.1 | Text shaping & layout — use `parley` (Vello's companion) or `cosmic-text` | ❌ (Text nodes render NOTHING today) |
| 3.2 | Font loading/enumeration (system fonts + custom), fallback chains, emoji | ❌ |
| 3.3 | Inline text editing on canvas (caret, selection, IME support for non-Latin input) | ❌ |
| 3.4 | Text properties: size, weight, line height, letter spacing, alignment, auto-resize modes (fixed/hug/truncate) | ❌ |
| 3.5 | Rich text spans (mixed styles within one text node), text decoration, lists | ❌ |
| 3.6 | Text styles (shared, updatable) | ❌ |

## Phase 4 — Visual Fidelity (fills, effects, images)

| # | Feature | Status |
|---|---------|--------|
| 4.1 | Gradient fills (linear, radial, angular, diamond) with on-canvas gradient handles | ❌ (only solid ✅) |
| 4.2 | Image fills: decode (png/jpeg/webp/avif), fill/fit/crop/tile modes, GPU texture cache | ❌ (Image nodes render nothing) |
| 4.3 | Multiple fills & strokes per node, per-paint opacity + visibility toggles | ❌ |
| 4.4 | Stroke options: align (inside/center/outside), caps, joins, dashes, individual side widths | 🟡 (width+color only) |
| 4.5 | Effects: drop shadow, inner shadow, layer blur, background blur (multiple per node) | ❌ |
| 4.6 | Blend modes (multiply, screen, overlay…) + masks (vector, alpha, luminance) | ❌ |
| 4.7 | Per-corner radius + corner smoothing | 🟡 (uniform radius only) |
| 4.8 | Color picker: HSB/RGB/hex, eyedropper, document swatches, P3/sRGB awareness | ❌ |

## Phase 5 — Design Systems (upgrade the existing slices)

| # | Feature | Status |
|---|---------|--------|
| 5.1 | Auto Layout v2: cross-axis alignment, fill-container, wrap, reverse, space-between, absolute-position children, **recursive nested layout solving**, canvas drag-to-reorder | 🟡 (single-axis stack only, non-recursive) |
| 5.2 | Components v2: create-from-selection, editable masters with live instance updates, variants + component properties (boolean/text/instance-swap), detach, reset overrides | 🟡 (name lookup + fill override only) |
| 5.3 | Override system v2: typed overrides (text content, visibility, any property — not just hex fill), override inheritance rules | 🟡 |
| 5.4 | Variables v2: string & boolean types, **modes** (light/dark/brand themes), aliasing (var→var), scoping, bind any property to a variable | 🟡 (color+number lookup only) |
| 5.5 | Shared styles: color styles, text styles, effect styles, grid styles | ❌ |
| 5.6 | Team/shared libraries: publish, import, update notifications | ❌ (much later) |

## Phase 6 — UI Chrome (the app around the canvas)

Pick a strategy first: pure-Rust UI (egui/xilem/custom Vello UI) vs hybrid (Tauri/web chrome + native canvas).

| # | Feature | Status |
|---|---------|--------|
| 6.1 | Layers panel: tree view, drag-reorder, rename, lock/hide, search | ❌ |
| 6.2 | Properties/inspector panel: numeric inputs with scrubbing, all node properties | ❌ |
| 6.3 | Toolbar (tools, zoom control, view settings) | ❌ |
| 6.4 | Assets panel (components, styles, variables manager) | ❌ |
| 6.5 | Pages (multiple canvases per document) | ❌ |
| 6.6 | Context menus, tooltips, command palette / quick actions (Ctrl+/) | ❌ |
| 6.7 | Rulers, guides, layout grids (columns/rows/grid overlay) | ❌ |
| 6.8 | Measurement overlay (hold Alt to inspect distances) | ❌ |

## Phase 7 — Documents & Interop

| # | Feature | Status |
|---|---------|--------|
| 7.1 | Native `.x` file format (versioned, forward-compatible; binary e.g. flatbuffers/postcard, or zip+JSON) | ❌ |
| 7.2 | Open/save/save-as, autosave, crash recovery | ❌ |
| 7.3 | Recent files / welcome screen | ❌ |
| 7.4 | Import: SVG (paths, fills, text) | ❌ |
| 7.5 | Import: `.fig` (reverse-engineered — kiwi schema; large effort, mark experimental) | ❌ |
| 7.6 | Export: PNG/JPG/WebP @1x/2x/3x, SVG, PDF; per-node export settings; batch export | ❌ (headless PNG PoC ✅ is the seed) |
| 7.7 | Copy as SVG / copy as PNG to clipboard | ❌ |

## Phase 8 — Prototyping (make the existing metadata real)

| # | Feature | Status |
|---|---------|--------|
| 8.1 | Interaction triggers: click, hover, press, drag, key, after-delay | 🟡 (destination+ms metadata only) |
| 8.2 | Actions: navigate, open overlay, swap, back, scroll-to, open URL | 🟡 |
| 8.3 | Transitions: instant, dissolve, slide/push, **smart animate** (property matching by layer name) | ❌ |
| 8.4 | Presentation/preview mode (play window, device frames, flows/starting points) | ❌ |
| 8.5 | Scroll behavior: fixed position, sticky, overflow scrolling in frames | ❌ |

## Phase 9 — Performance & Quality (continuous, but budget for it)

| # | Feature | Status |
|---|---------|--------|
| 9.1 | Spatial index (R-tree/BVH) for hit-testing & culling at 100K+ nodes | 🟡 (linear culling only) |
| 9.2 | Incremental scene re-encoding (only dirty subtrees re-encode) | 🟡 |
| 9.3 | Tiled/cached rendering of static content while editing | ❌ |
| 9.4 | Background threads: file IO, image decode, font loading off the UI thread | ❌ |
| 9.5 | Benchmark suite in CI (the 10K/50K scenes ✅ are a start — add frame-time budgets) | 🟡 |
| 9.6 | Fuzzing the file-format parser; property tests for layout solver | ❌ |

## Phase 10 — Collaboration & Platform (the long game)

| # | Feature | Status |
|---|---------|--------|
| 10.1 | Multiplayer: CRDT or OT document sync, presence cursors, comments | ❌ |
| 10.2 | Version history / named checkpoints | ❌ |
| 10.3 | Plugin API (WASM sandbox is the natural native choice) | ❌ |
| 10.4 | Dev mode: inspect, copy CSS/Swift/Compose values | ❌ |
| 10.5 | Cross-platform packaging: Windows/macOS/Linux installers, code signing, auto-update | ❌ |
| 10.6 | Accessibility: keyboard-navigable UI, screen-reader labels (AccessKit) | ❌ |

---

## Suggested build order (critical path)

```
0. Deps upgrade + node-ID arena + CI          ← unblocks everything
1. Window + live renderer + pan/zoom          ← first "it's an app" moment
2. Hit test → selection → move/resize → UNDO  ← first "it's an editor" moment
3. Inspector + layers panel (minimal)         ← first usable loop
4. Save/load .x format                        ← work survives restart
5. Text rendering + editing                   ← biggest single subsystem
6. Fills/effects/images                       ← designs start looking real
7. Auto Layout v2 + Components v2 + Variables v2
8. Export (PNG/SVG) + SVG import
9. Prototyping playback
10. Performance passes, then collaboration/plugins
```

## Biggest risk items (plan extra time)

1. **Text** (Phase 3) — shaping, IME, editing. Easily 30% of total effort.
2. **Undo/redo architecture** (2.4) — decide command-log vs snapshot before Phase 2 code exists.
3. **Nested Auto Layout solving** (5.1) — a real constraint/measure-arrange pass, not the current single loop.
4. **.fig import** (7.5) — unofficial format; treat as stretch goal.
5. **UI chrome strategy** (Phase 6) — pure-Rust UI is more work up front but avoids a permanent JS dependency, which is the whole point of "X-Native".

---

## v0.26 hardening-pass addendum (post-review, session 31)

Status shifts from the "short hardening pass" review:

| Area | Was | Now |
|------|-----|-----|
| Styles UI | engine-only registry | ✅ create (+P/+T/+FX), apply = LINKED binding (`style:paint/text/fx`), Shift+click chip = redefine-from-selection → `resolve_styles` updates every consumer on all pages; re-sync on file open; tested (`style_mutation_updates_all_consumers`) |
| Image placement | fit modes engine-only | ✅ inspector: fit chips, REPLACE, focal X/Y %, SCALE %, FH/FV flips, RESET; `ImagePlacement` persisted in .x; rendered by VelloSink (focal-aware crop, zoom, mirror) |
| Boolean API | editor called raster backend directly | ✅ stable facade `boolean(op, a, b) -> BooleanResult` with `Backend::{RasterGuided, Exact}`; app routed through it. Exact clipper still TODO before v1.0 (currently falls back) |
| Mask semantics | 1 IR test | ✅ 5-test matrix: image / vector / group / component+auto-layout under mask + full chain mask→boolean→gradient→effect→SVG+PDF |
| Export verification | structural only (%PDF sniff) | ✅ visual regression: 7 fixtures × (GPU canvas render vs rsvg-rasterized SVG vs ghostscript-rasterized PDF), RMSE-gated (`export_regression` bin + `tools_visual_compare.sh`). Caught & fixed: SVG had NO mask/instance support; PDF had NO clip support |
| Undo/redo | per-op tests | ✅ realistic chain test: create→auto-layout→component→variable→boolean→mask→image-crop→style→export, full undo to empty, full redo to final, save+reload byte-identical |

Remaining before v1.0 (unchanged priorities):
1. Exact-curve boolean clipper behind `Backend::Exact` (raster tolerance ~8% is beta-only)
2. Text visual parity in exports (SVG/PDF use fallback text: RMSE ~0.17-0.18 vs canvas — shaped-glyph outlines needed)
3. Color emoji sink path; style rename/delete UI; native file picker for image replace
