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

## v0.27 addendum (review round 2, session 32)

| Pre-v1.0 item | Status |
|---|---|
| Exact-curve boolean clipper | ✅ **Greiner–Hormann clipper shipped, now the DEFAULT backend** (`x-editor/src/clip.rs`). Analytic intersections; <0.1% squares, <1% flattened curves; stress-tested on slivers, repeated ops, 1e6/1e-3 coordinate ranges. Raster backend retained as fallback for multi-contour/self-intersecting operands. Remaining: multi-contour operands, bezier re-fitting of result contours |
| Export images fixture | ✅ 8th fixture; SVG base64 data-URI embedding, PDF DeviceRGB XObjects with fit-mode CTMs. RMSE ≤0.11 |
| Undo/redo chain w/ Variant | ✅ switch_variant step added; depth ≥13; byte-exact undo/redo/save/reload |
| Grouped styles browser | ✅ PAINT/TEXT/EFFECT sections, linked-chip highlight, shared painter/hit-test geometry |

Remaining before v1.0 (updated):
1. Text export parity (shaped glyph outlines in SVG/PDF) — now the top visual gap
2. Bezier re-fit of exact-boolean output; multi-contour operand support
3. SVG focal/flip placement; PDF FlateDecode image compression
4. Color emoji; style rename/delete; native file picker

## v0.29 addendum (session 34): Text export parity SHIPPED
Top visual gap closed: one `node_text_outlines` pipeline drives canvas, SVG, and PDF text geometry (drift impossible by construction). Text RMSE: SVG 0.168→0.0038, PDF 0.183→0.018. Quad→cubic elevation bug fixed in the PDF path emitter. 4-test parity suite pins the contract.
Remaining before v1.0 (updated): PDF gradient interpolation (shading dicts), image focal/flip in SVG + FlateDecode PDF images, bezier re-fit of exact booleans, color emoji, selectable-text PDF mode (embedded fonts + ToUnicode).

## v0.30 addendum (session 35): Import Fidelity + Interoperability

Review directive: importers must share ONE intermediate representation.
Target architecture — now IMPLEMENTED:

```
External File → Importer (parse only) → Import IR (ImportDoc/ImportNode)
    → import_ir::lower()  ← ALL shared semantics live here
    → X-Native Document → Render IR → Editor
```

Shared semantics in lower(): id sanitize+dedupe, kind-default fills
(text=black), opacity clamp, NaN scrub, page auto-size, render-effective
"text:" override encoding, one rotation convention (radians, CW+ in
y-down; each importer converts at parse: Sketch negates, SVG passes
through, Figma negates).

Import matrix:
| Format | Status | Path |
|--------|--------|------|
| .x     | ✅ native | load_x / v2 / lenient recovery |
| Sketch | ✅ via IR (refactored from direct→Node) | zip+json → IR |
| SVG    | ✅ via IR (refactored from direct→Node) | xml → IR |
| PNG    | ✅ NEW via IR | IHDR dims → Image node + asset bytes |
| Figma  | ✅ NEW via IR | REST-API JSON (documented format); binary .fig is proprietary/undocumented — out of scope by design |
| PDF    | ❌ next slot in the matrix (content-stream subset: re-import our own exports first) |

Enforcement: `import_conformance.rs` runs the SAME contract over every
importer (unique/sanitized ids, black text default, >0 pages, clamped
opacity, finite geometry, byte-stable .x round trip, non-empty render
IR) + `same_scene_same_semantics_across_importers` proves one logical
scene from Sketch/Figma/SVG lands identically.

## v0.31 addendum (session 36): AssetManager shipped
Content-addressed AssetStore (asset://<hash> ids; hash/mime/dims/bytes/source records; magic-byte sniffing; dedup; GC) in x-core. .x embeds assets (portable documents — live-proven by deleting the source zip and reloading). Import IR lowers ImportDoc.assets into the store; Sketch bitmaps use the real image._ref. Render cache syncs from the store. Solved per review: Sketch imports ✅ image replacement (cache-level) ✅ portability ✅ export (resolver can read store) ✅ dedup ✅ caching ✅.
Next: jpeg/webp decode in sync_store, font assets in store, store-aware replace picker UI, legacy-filename migration pass.

## v0.32 addendum (session 37): Booleans 2.0 + PDF quality
Curve-preserving boolean backend (bezier intersections → topology → bezier output) is the new default; polygon+raster tiers retained as fallbacks behind the same facade. Repeated-op degradation eliminated (tested to 4 generations; live union = 8 CurveTo vs old ~130 LineTo). PDF: gradient shading dicts (0.119→0.0075), real tiling, FlateDecode images (0.109→0.037). Text parity confirmed shipped (v0.29). Remaining export gaps: SVG image fit modes (0.10), PDF sweep gradients, color emoji (deprioritized per review).

## v0.33 addendum (session 38): Canonical image transforms + Styles UI complete
resolve_image_placement() = one fit/focal/zoom/flip/tile resolution for Vello/SVG/PDF (images-SVG 0.10→0.023; sinks now geometry-free). Styles UI finished per review: search, usage counts, apply=link, shift-redefine→consumers update, Ctrl-select + REN(rebinds all)/DUP/DEL(detaches)/DET row — live-verified, persisted. NOT built: local-vs-library (no library concept yet); style ops not undoable.

## v0.34 addendum (session 39): Libraries (engine) + Asset browser
Versioned library system per review: .xlib artifact, library:// URIs (no copying), pinned LibraryDependency + inline snapshots in .x (self-contained docs), diff_library/accept_update = review-accept flow; silent-update impossibility is TESTED. Asset browser: Shift+A overlay w/ real thumbnails, search, place, replace, rename, del-unused, usage counts. Next: library link/review/accept UI + library styles in the styles browser; browser drag/scroll/sort.

## v0.35 addendum (session 40): Library integrity + Library UI + library components
snapshot_hash integrity (canonical-serialization fnv1a128, Verified/LegacyUnhashed/Corrupt/MissingSnapshot, live-caught a hand-edited .x). LIBS inspector tab = strict client of diff_library/accept_update: link, per-library card, update banner, review overlay w/ old→new swatches, accept/cancel — full flow live-verified v1→v2. Library components place as INSTANCES over one hidden registry master (no cloning). Next: master-refresh on accept, freeze-on-Corrupt, library styles in styles browser.

## v0.36 addendum (session 41): Beta checklist wave 1
Designer Beta: component propagation on accept, freeze-on-corrupt, shared LIBS layout, asset browser scroll/sort/drag/rename — DONE. Reliability: atomic writes, autosave+crash recovery (live-proven), rolling backups, corruption chain, legacy-hash upgrade, recent files — DONE. Performance: 1k–100k benchmarks (mixed 10k=138ms — virtualization now REQUIRED, baselines recorded) + frame HUD. Interop: import diagnostics engine-side. Deferred: virtualization, memory profiling, clipboard, import preview UI, SVG/Figma import round 2, resolver abstraction.

## v0.37 addendum (session 42): Performance wave
ShapedTextCache (full shaping key, position-free by construction, review's invalidation matrix tested) + incremental Render IR via SceneCache (identical frames skip encode — live HUD shows CACHED) + phase-instrumented HUD + layer/thumbnail virtualization. ACCEPTANCE: mixed 1k=1.4ms ✓ (<=16.7), 10k=14ms ✓ (<=33, was 138), 100k=215ms ✗ (<=100 stress; was 1590). Next perf: dirty-subtree IR reuse + viewport culling for 100k, memory profiling.

## v0.38 addendum (session 43): Perf wave 2 + import UX + golden CI
FrameCache dirty-subtree reuse (bucketed encoded segments, full-hit fast path; drag 10k=14ms, 1k=1.3ms; 100k=237ms — needs culling). Memory profiling (peak 670MB@100k), text-cache byte budget+eviction, command latency in HUD. Import preview overlay w/ IR thumbnails + fidelity warnings + accept/cancel; sketch skip diagnostics live. Golden-project CI (pinned IR shape, byte-stable .x, undo-exact) + ci.sh. Deferred: viewport culling (top perf lever), SVG import 2, clipboard, Figma ImportDoc/diagnostics.

## v0.39 addendum (session 44): Perf wave 3
Viewport culling in FrameCache (conservative blur-inflated bounds, memoized; visibility in bucket keys; live "CULLED n" HUD): 10k drag 6.9ms, 100k 110ms (floor = O(n) hash walk — needs editor dirty-IDs next). Memory breakdown across doc/styles/vars/assets/libs/text/segments/GPU/undo in HUD. GPU/thumbnail eviction on browser close (store keeps bytes; decode-on-demand). 275 tests.

## v0.40 addendum (session 45): UI overhaul to product mockup
Real shaped typography in ALL chrome (label() -> ShapedTextCache; vector font retired). Two-row header (logo, doc tab w/ dirty dot, menus, centered tools, zoom, ▶ Present). Bottom live page-thumbnail strip + status bar w/ selection geometry. Left panel icon tabs + search field; inspector filled fields + section restyle; #3366FF theme. Next UI: wire Assets/Components/Library tabs, real menus, collapsible inspector sections, Export section.
