# Changes made to arco-native-x-test-v03

## Session 59 — editable interchange and export-surface polish

- Added dedicated, filtered import actions for native `.x`, Figma REST JSON,
  and Sketch documents, all routed through the existing safe preview/accept
  workflow.
- Added native `.x`, Figma REST JSON and real Sketch-package export actions to
  both the File menu and the inspector's compact 3×2 export grid.
- Figma JSON export preserves editable frames/groups/components/instances,
  geometry, paths, fills, gradients, strokes, opacity and transforms.
- Sketch export now writes a standards-compliant ZIP package with valid CRC32,
  page references, editable vector points, symbols, styles and embedded PNGs.
- Added exporter→importer round-trip regression tests for Figma JSON and Sketch.
- Kept the product language honest: Figma support is REST JSON interchange,
  not the proprietary undocumented binary `.fig` format.

## 1. Fixed a real compile error the README's "cargo test/cargo run" claim missed
`Color::with_alpha()` / `.alpha()` don't exist on this project's pinned
`vello`/`peniko` version — only `with_alpha_factor(f32)` does (and it
already does exactly what the code was manually trying to compute: existing
alpha × factor). This affected all 3 fill call sites (Rect, Ellipse, Line).
As shipped, this crate did not compile — worth knowing, since it means the
README's stated verification either wasn't run against this exact
dependency tree, or wasn't run at all before packaging.

## 2. Verified claims in the README against the actual code (not just read them)
- **Confirmed real**: scene graph, transforms, viewport culling, dirty
  counting, deterministic 10K/50K stress scenes. All covered by passing
  tests.
- **Confirmed overstated**: "variables/tokens: color and number
  resolution" — `Variables.numbers` is stored but never read/resolved
  anywhere. "component + instance property overrides" — `Node.overrides`
  is stored but never applied to anything. Both are inert data fields, not
  working systems, as of this version.
- **Confirmed via actual run output, not just reading**: `Text`,
  `Vector`, `Component`, and `Instance` node kinds have an empty match arm
  in `encode()` — they render nothing. The demo scene's own printed stats
  prove it: `nodes=4 paths=2` (only the rect and ellipse produced draw
  calls; the text node did not).

## 3. Restructured as a real lib + bin crate
Was a single `main.rs` (binary-only). Split into `src/lib.rs` (all the
actual model/scene/render logic, now testable and reusable) + a thin
`src/main.rs` that just builds the demo scene using the library. This
was necessary to add a second binary without duplicating code.

## 4. New: `src/bin/render_headless.rs` — actual rendered pixels
Builds a real `wgpu` device (software Vulkan/`llvmpipe` in this sandbox —
a real GPU on any actual machine), runs the demo scene through
`vello::Renderer::render_to_texture`, copies the result off the GPU, and
writes `render_output.png`. This is the strongest verification available
without a live window: real rendered output, not draw-op counts or
compiling code. See `render_output.png` — a visibly rotated (22.5°)
rounded rectangle with correct corners, and a circle whose muted color
confirms 0.75-opacity alpha blending is working correctly.

Required installing `mesa-vulkan-drivers` + `vulkan-tools` in this sandbox
(not present by default; a real dev machine with a GPU wouldn't need the
software driver).

## Toolchain note (same as the earlier renderer PoC)
Built against Rust 1.75 via apt with a chain of transitive dependency pins
(`indexmap`, `hashbrown`, `num_enum`, `proc-macro-crate`, `pxfm`,
`moxcms`) worked around to avoid edition2024 requirements this sandbox's
Rust can't satisfy (can't reach `rustup.rs` from here to get current
Rust). Also swapped the `image` crate for the lower-level `png` crate
directly — `image`'s PNG path pulls in modern color-management
dependencies (`pxfm`/`moxcms`) with the same edition2024 problem, which
`png` alone avoids. None of this pinning should be necessary on a real
machine with current Rust via `rustup`.

## Verification
- `cargo build` — clean (warnings only, no errors).
- `cargo test` — 5/5 pass.
- `cargo run --bin arco_native` — runs, matches expected output.
- `cargo run --bin render_headless` — runs, produces a correct PNG
  (visually confirmed).

---

# Session 2 — closed the two gaps flagged as stubs

Both gaps identified in session 1 (`Variables.numbers` unused,
`Node.overrides` unapplied) are now real, wired-up, tested features —
not just added methods.

## 5. `Variables::number()` wired into Auto Layout gap/padding
Added `AutoLayout.gap_var`/`padding_var: Option<&'static str>`. When set,
`apply_auto_layout` (now takes `&Variables`) resolves the gap/padding from
the variable, falling back to the struct's literal value if the variable
name isn't defined. Two new tests: one proving the variable value is what
actually drives layout (not the fallback), one proving the fallback
behavior when the variable is missing.

## 6. Instance -> Component resolution, with real fill overrides
- `collect_components()` walks the tree once per `build_scene()` call,
  building a name -> `&Node` registry of every `Component` node.
- Encoding an `Instance` node looks up its component by name and encodes
  the component's children in the instance's place — the component
  definition itself renders nothing directly (matches the existing
  no-op-for-`Component`-kind behavior), only what's actually instanced
  does.
- `Node.overrides` (`HashMap<id, hex color string>`) is now read: any
  descendant of the resolved component whose id matches an override key
  gets that fill color instead of its own, via a new `effective_fill()`
  used everywhere a fill was previously resolved directly. Overrides
  propagate to all nested descendants by id (not just direct children),
  but a nested `Instance` inside the resolved subtree switches to *its
  own* overrides rather than inheriting the outer instance's — matches
  how per-instance overrides actually behave in comparable systems.
- **Cycle guard**: a `MAX_INSTANCE_DEPTH` (32) stops a component that
  (directly or transitively) instances itself from recursing forever.
  Added a test that builds exactly that malformed document and asserts
  the render terminates rather than hanging or overflowing the stack.

4 new tests (10 total, up from 5+1): instance resolution renders the
right number of paths, overrides change the resolved color (tested
directly against `effective_fill`, matching this suite's existing style
of asserting on resolved values rather than decoding vello's opaque
`Scene` encoding), the missing-variable fallback, and the cycle guard.

## 7. Visual re-verification
Extended `render_headless.rs`'s demo scene to include the hidden master
component + 3 instances (two overridden, one not) alongside the existing
rotated/rounded card and translucent dot, re-ran the real headless GPU
render, and looked at the actual output. First attempt revealed a real
demo-composition mistake (not a library bug): placing the hidden master
*inside* the auto-layout row still consumed layout space — a hidden node
isn't a zero-width node — pushing the last button off the 800px canvas
entirely. Moved the master outside the row (the correct pattern), re-ran,
confirmed all 5 expected shapes render with correct colors, including the
non-overridden instance correctly falling back to the component's own
default fill (visible as a subtle tonal difference from the background,
not just "nothing rendered").

## Verification (session 2)
- `cargo build` — clean.
- `cargo test` — 10/10 pass.
- `cargo run --bin arco_native` and `cargo run --bin render_headless` —
  both run correctly; the headless render's output was visually inspected,
  not just assumed correct from exit code 0.

---

# Session 3 — v0.4: every roadmap phase gets a working native slice

Scope: everything that can run and be verified headless in this sandbox.
Phase 1 (winit window) needs a display; all editor ops were built
UI-independent so the window layer is now purely event translation.

## New modules
- `src/editor.rs` — hit testing (rotation/ellipse/lock aware), marquee,
  command-log **undo/redo**, move/resize/rotate/fill/text/delete/z-order,
  group/ungroup (snapshot-undo), align/distribute, snapping, constraints,
  prototype Player, SpatialGrid (100K nodes ~20ms build, ~10µs queries),
  version checkpoints, dev-mode CSS export.
- `src/text.rs` — Text nodes now draw real vector glyphs (built-in
  16-segment stroke font); v0.3's empty match arm is gone.
- `src/fileio.rs` — versioned `.x` JSON format with zero-dependency
  writer + parser (byte-stable double roundtrip, forward-compatible),
  plus SVG export.

## lib.rs upgrades
- gradients (linear/radial), per-corner radii, drop shadows, blend modes
  (real Vello mix layers), strokes on rects
- Auto Layout v2: cross-axis align, space-between, cross-axis hug,
  recursive solve
- Variables v2: strings/bools, aliases (cycle-limited), modes (light/dark)
- typed overrides: `text:` prefix replaces Text content per instance
- all `&'static str` model fields replaced with owned `String` (Phase 0)

## Verification (session 3)
- `cargo test` — **45/45 pass** (was 10).
- `cargo run --bin arco_native` — scripted editor session prints verifiable
  results for every phase slice (undo/redo roundtrip, .x roundtrip stable,
  prototype navigation, spatial-index speedup vs full-tree hit test).
- `cargo run --bin render_headless` — re-rendered on real (software) GPU;
  output now also shows: gradient bar, vector text "X NATIVE 0.4",
  drop shadow under the rotated card. Visually inspected.

---

# Session 4 — v0.5: window shell + vectors + interop + smart animate

## Phase 1: `x_native_app` — the actual windowed application
winit 0.29 + wgpu surface + `render_to_surface`, camera pan/zoom, click &
drag editing, keyboard shortcuts, save/load `document.x`, selection
overlays. Every interaction is a thin translation onto the already-tested
`Editor` — no editing logic lives in the window layer. Verified to compile
here (no display in sandbox); run on a real machine.

## Phase 2.6: NodeKind::Vector became real
`Vector { path_count }` (inert metadata since v0.3) is now
`Vector { path: Vec<PathCmd> }` — Move/Line/Cubic/Close in local coords —
rendered as real filled + stroked paths, serialized in `.x`, exported to
SVG, and drawn in render_output.png (the gold star).

## Phase 2.7: copy / paste / duplicate
Clipboard of cloned subtrees; paste remaps every id in the subtree against
the full document id set; insert goes through the command log so undo
removes pasted nodes cleanly.

## Phase 7.4: SVG import
Hand-rolled XML lexer (attrs, self-closing tags, comments) + element
parser. Imports rect/circle/ellipse/line/path/text/g with fills, rx,
opacity, translate/rotate, and a real `d`-attribute parser (absolute +
relative commands, H/V shorthands). Round-trip test: export our page to
SVG, re-import, shape kinds/counts/renderability all assert.

## Phase 8.3: smart animate
`smart_animate(from, to, t)` matches nodes by id and lerps
position/size/rotation/opacity/solid-fill; unmatched nodes fade in/out.
Mid-frame is a plain renderable Node — proven by rendering t=0.5 of a
red→blue morph in render_output.png (the purple box, exactly halfway in
position, size, and color).

## Verification (session 4)
- `cargo test` — **55/55 pass** (was 45).
- `cargo build --bin x_native_app` — compiles clean against winit 0.29.
- `cargo run --bin arco_native` — new sections print verifiable results:
  paste ids + undo, star renders 1 path, SVG re-import renders 13 paths,
  smart-anim midpoint x=100 w=150 fill=#800080.
- `cargo run --bin render_headless` — updated PNG visually inspected:
  star + smart-animate mid-frame present and correct.

---

# Session 5 — ran the actual windowed app (Xvfb) and click-tested it

No physical display in this sandbox, so: Xvfb virtual X server (1280x800)
+ xdotool for real mouse/keyboard events + screenshots of the live window.
This is the actual `x_native_app` binary running its real winit event loop
and wgpu surface (llvmpipe software Vulkan) — not a mock.

## One real bug found and fixed by doing this
`x_native_app` hardcoded `Rgba8Unorm` as the surface format; the X11
surface only offers `Bgra8UnormSrgb`/`Bgra8Unorm`, so the app panicked on
`Surface::configure`. Now queries `surface.get_capabilities()` and picks a
supported non-sRGB format. This would have crashed on many real Linux
machines too — exactly the kind of thing only actually running it reveals.

## Interactions verified with screenshots (app_screenshot_*.png)
1. App opens, demo document renders in-window (title "X Native", 1280x800).
2. Click on card -> blue selection outline + corner handles appear.
3. Drag -> card moves with the cursor, outline tracks it.
4. Ctrl+Z -> drag segment undone (visible position step-back).
5. Ctrl+D -> duplicate appears offset behind the original, selected.
6. Ctrl+S -> `document.x` written; contains `"id":"card-copy"` proving the
   duplicate went through the command log into the saved file.
7. Ctrl+scroll -> zoom-to-cursor (screenshot at ~2x, text glyphs scale
   crisply — vector text, not bitmaps).

Sandbox prerequisites (a real desktop needs none): Xvfb, xdotool,
libxkbcommon-x11 (+ unversioned .so symlink), XDG_RUNTIME_DIR set,
mesa-vulkan-drivers for llvmpipe.

---

# Session 6 — v0.6-beta.1: the beta app

`x_native_app` grew from a proof-of-life shell into a usable beta editor.
All chrome is drawn by Vello itself (same renderer as the document; the
built-in vector font renders every label) — no UI toolkit dependency.

## New in the app
- Toolbar: Select/Frame/Rect/Ellipse/Line/Text tools (click or V/F/R/O/L/T)
- Drag-to-create shapes on canvas (undoable Insert via command log)
- Marquee selection when dragging empty canvas with Select
- Resize handles on single selection (opposite corner stays pinned)
- Layers panel: live tree, indented, click / shift-click to select
- Inspector: id/kind/x/y/w/h/opacity + 8-swatch fill palette (undoable)
- Arrow-key nudge (Shift=10px), Ctrl+]/[ z-order, Ctrl+E SVG export,
  Esc to deselect, live zoom % display, status bar messages
- Drag gestures merge into ONE undo step (`Editor::merge_last`)

## Engine additions (tested)
- `Editor::merge_last` / `undo_depth` — gesture merging (test:
  three 5px moves + merge -> single undo returns to origin)
- `Editor::insert_node` — undoable programmatic insert (test)

## Click-tested live in Xvfb (beta_*.png, all visually verified)
1. Launch: chrome + demo doc render, layers panel lists the tree.
2. Canvas click -> selection outline + handles + panel row highlight +
   live inspector properties.
3. Inspector swatch click -> card recolored green, undoable, logged.
4. R + drag -> "r-1" rect created exactly under the drag (W:300 H:200
   in inspector), auto-selected, appears in layers panel and saved file.
5. Corner-handle drag -> resized to W:467 H:300, status "RESIZED".
6. Layers-panel click on TITLE -> selects the text node on canvas.
7. Ctrl+E + Ctrl+S -> export.svg (with gradient defs) + document.x
   (contains r-1 with resized W/H and card fill #2ecc71).
8. Ctrl+Z -> the whole resize gesture reverts in one step (W back to 300).

`cargo test`: 57/57 pass.

---

# Session 7 — v0.6-beta.2: text editing, rotate, fields, pages

## New in the app (all click-tested live in Xvfb, beta2_*.png)
- **Inline text editing**: double-click a Text node -> yellow edit frame,
  live character-by-character preview on canvas, status echoes the buffer,
  Enter commits (undoable via SetText), Esc restores the original.
  Verified: retyped title to "HELLO BETA", committed, persisted in .x.
- **Rotate handle**: knob + stem above single selection; drag rotates
  around center, Shift snaps to 15° steps, gesture merges to one undo.
  Verified: card rotated to 92° (status + inspector + saved file agree).
- **Numeric inspector fields**: click X/Y/W/H box -> yellow active state,
  type digits, Enter applies (Move/Resize through the command log).
  Verified: W set to 400 exactly.
- **Multi-page**: tab bar in the top bar, "+" adds a page, switching
  stores/restores each page's tree; Ctrl+S saves ALL pages.
  Verified: page-2 created, ellipse drawn on it, switch back intact,
  document.x contains both pages ('page-1','page-2' with 'o-1').

## Notes
- Focused edits capture the keyboard entirely (no tool-shortcut leaks).
- Text cancel restores content without polluting the undo stack.
- 57/57 engine tests still pass; app builds clean.

---

# Session 8 — v0.7-beta.3: images, presentation mode, smart guides, opacity

## Engine (60/60 tests, was 57)
- `Assets` (Phase 4.2): PNG decoding (rgba/rgb/gray/gray-alpha -> RGBA8)
  via the existing `png` crate; `build_scene_with_assets` draws real
  bitmaps for Image nodes via `Scene::draw_image`, placeholder otherwise.
  Test writes a real PNG, decodes, renders, asserts.
- `alignment_guides` (Phase 2.10): edge/center match detection against
  every other visible node. Tests: edge match, center match, no-match.
- `Command::SetOpacity` + `Editor::set_opacity`, undoable. Test.

## App
- **Image assets**: PNGs in ./assets/ auto-load by filename stem; an
  Image node with that asset name renders the actual bitmap. Verified:
  injected a 'photo' node referencing 'checker' -> checkerboard rendered
  on canvas, scaled into its 256px box, listed in layers panel.
- **Presentation mode** (Ctrl+P): full-window black-backed playback,
  page fitted to screen, click advances pages with a 350ms ease-in-out
  SMART-ANIMATE transition (live use of `smart_animate` per frame),
  Esc exits. Screenshot mid-transition shows the interpolation actually
  rendering (all shared shapes mid-fade, page frame mid-morph).
- **Smart guides**: red alignment lines across the canvas while dragging
  a single node, tolerance 3px, edges + centers. Screenshot shows 3
  horizontal guides while the dot aligns with its row neighbors.
- **Opacity control**: -/+ buttons in the inspector, 0.1 steps, undoable.
  Verified 0.8 -> 0.5 after 3 clicks, persisted through Ctrl+S.

All verified live in Xvfb (beta3_*.png).

---

# Session 9 — v0.7-beta.4: auto-layout controls in the UI

## Engine (61/61 tests)
- `Command::ReplaceNode`: whole-node swap command for mutations with wide
  side effects; inverse is the reverse swap.
- `Editor::set_auto_layout(id, Option<AutoLayout>, &vars)`: sets/clears a
  frame's layout AND re-solves child positions as ONE undoable command.
  Rejects non-frames. `auto_layout_of(id)` reads current config.
- Test: apply -> children restacked; one undo -> original scattered
  positions AND no layout; redo; clear keeps positions; non-frame rejected.

## App: LAYOUT section in the inspector (frames only)
- NONE / H / V direction buttons (active one highlighted)
- GAP and PAD -/+ steppers (4px, floor 0), live re-flow on every click
- defaults on first enable: gap 16, pad 16, cross-axis center

## Click-tested live in Xvfb (beta4_*.png)
1. Select PAGE-1 -> LAYOUT section appears showing H / GAP 24 / PAD 40.
2. Click V -> entire page re-stacks vertically, center-aligned, instantly.
3. GAP + twice -> 24 -> 32, spacing visibly widens.
4. Ctrl+Z x2 -> back to horizontal gap 24 (screenshot verified).
5. Ctrl+Shift+Z x2 -> vertical gap 28 again; saved file confirms
   `dir v gap 28`.

## Real bug found by live-testing (and fixed)
Redo was dead: with Shift held, winit delivers the UPPERCASE character
("Z"), so the `"z"` match arm never fired for Ctrl+Shift+Z. Normalized
with `to_ascii_lowercase()`. Undo had always worked; only redo-by-
keyboard was affected — exactly the kind of bug only real input testing
catches.

---

# Session 10 — v0.8-beta.5: component workflow

## Engine (62/62 tests)
- `Editor::make_component(name)`: selection -> hidden master Component at
  the document root (members re-based to origin) + an Instance replacing
  the selection in place. Snapshot-undo (one step reverts everything).
- `Editor::place_instance(component, x, y)`: stamps a new uniquely-id'd
  Instance sized from the master. Undoable insert.
- `Editor::component_names()` for the assets UI.
- Test covers: instance replaces selection at collective origin, master
  hidden with re-based children, render resolves instances (path counts),
  second placement, undo of placement, undo of componentization.

## App
- **Ctrl+K**: create component from selection (auto-named ComponentN).
- **ASSETS panel** (bottom of layers): one row per component with a
  Figma-style purple diamond; click arms stamping, next canvas click
  places an instance there (status bar guides the flow).
- Layers panel shows INST rows and the hidden COMP master row.

## Click-tested live in Xvfb (beta5_*.png)
1. Ctrl+K on selection -> Component1-1 instance selected, COMP-COMPONENT1
   master in layers, COMPONENT1 in ASSETS with diamond.
2. Stamped Component1-2 and -3 via ASSETS click + canvas click.
3. Selected the MASTER's dot child in layers, clicked green swatch ->
   ALL THREE instances turned green simultaneously (live master->instance
   propagation, the core component value).
4. Saved file verifies: hidden master (visible:false), dot fill #2ecc71,
   instances Component1-1/-2/-3 on the page.

---

# Session 11 — v0.9-beta.6: prototype linking + tool polish

## New: prototype linking & click-through (Phase 8 completed in-app)
- Engine: `Command::SetPrototype` + `Editor::set_prototype` (undoable,
  clearable). Test: set -> undo -> redo -> clear.
- Inspector "PROTOTYPE" section: NONE + one button per other page;
  clicking links the selected node (350ms transition).
- Canvas: linked nodes show a purple "»" badge chip at their top-right.
- Presentation mode is now a real prototype player: clicking maps the
  cursor back into page space, hit-tests the page, walks ancestors for
  the nearest link, and smart-animates to THAT destination; clicking
  empty space still advances sequentially.
- Verified live: card linked to page-2 (badge visible), Ctrl+P, click ON
  the card -> transitioned to page-2; saved file has {'to':'page-2'}.

## Tool improvements (all verified live)
- **Hover highlight**: thin blue outline under the cursor (Select tool),
  suppressed for already-selected nodes and over chrome.
- **Shift = aspect-lock resize**: corner drag keeps w:h exactly —
  verified 260x160 -> 360x221.5, ratio 1.625 preserved to 4 decimals.
- **Ctrl+0 / Ctrl+1**: zoom 100% / zoom-to-fit (fit computed against the
  canvas area, verified at 49% for the 1600x1000 page).
- **Scrollable layers panel**: wheel over the panel scrolls rows (2/tick),
  "..." indicator when scrolled, click mapping accounts for offset.
- Layers wheel no longer pans the canvas underneath.

## Environment note
Sandbox lost apt packages + part of the cargo registry between sessions
(snapshots exclude caches); reinstalled and cleared ~/.cargo/registry/src
to force re-extraction. Source tree was unaffected.

63/63 engine tests pass.

---

# Session 12 — v0.10-beta.7: features & interface derived from Figma/Sketch docs

Sources mined this session:
- Figma "Access design tools from the toolbar" + "Tour the interface"
- Sketch "The Mac app interface"

## New tools (Figma toolbar parity)
- **Hand tool (H)** + **spacebar-hold temporary hand** (Figma tip verbatim):
  drag pans the canvas; space release returns to the previous tool.
- **Polygon (P)** and **Star (S)** shape tools (Figma's shape-tool menu):
  drag-create real vector nodes — regular hexagon and 5-point star path
  generators (`regular_polygon`, `star_path`) feeding NodeKind::Vector.
  Verified: s-1 saved with 11 path cmds, p-2 with 7.

## Interface (Sketch Mac-app parity)
- **Minimap** (Sketch #5): bottom-right overlay showing the page outline,
  top-level layers as colored blocks, and the current viewport rectangle;
  click anywhere on it to jump the viewport there.
- **Search Layers** (Sketch layer list): FIND box atop the layers panel;
  typing filters rows live by id or kind (verified: "vector" -> only the
  two vector layers). Esc clears, Enter keeps the filter.
- **Hide Interface** (Sketch ⌘. -> our Ctrl+.): full-bleed canvas with all
  chrome hidden, hint text in the corner, toggle back on.
- Big nudge (Shift = 10px) already matched Figma's default nudge values;
  kept as-is per the nudge doc.

All click-tested live in Xvfb (beta7_*.png): star+hexagon drawn on canvas,
search filtering, minimap present with viewport rect, hidden-UI mode,
space-pan.

63/63 engine tests still pass.

---

# Session 13 — v0.11-beta.8: Scale tool, frame presets, rulers/guides, outline view

Continuing through the Figma/Sketch doc mining.

## Engine (64/64 tests)
- `Editor::scale_node(id, factor)` — Figma's Scale tool semantics: scales
  the node AND subtree uniformly (sizes, child offsets, stroke widths,
  corner radii, vector path coords). One undoable ReplaceNode. Test
  verifies child offset/size/radius scaling and undo.
- Frames now RENDER their background fill (Figma: frames have fills,
  groups don't) — found because the phone-preset frame was invisible on
  canvas; fixed in encode() with drop-shadow support included.

## App
- **Scale tool (K)**: click selects, vertical drag scales the subtree
  live (200px = ±100%, clamped 20%–500%), whole gesture = one undo.
  Verified: phone frame 390x844 -> exactly 585x1266 (+50% for 100px).
- **Frame presets** (Figma's frame-tool panel): with Frame tool active,
  inspector lists PHONE/TABLET/DESKTOP/WATCH/SLIDE; click drops a
  preset-sized white frame. Verified: 390x844 phone frame created.
- **Rulers (Shift+R)**: top/left strips with labeled ticks every 100
  units; click a ruler to drop a cyan guide at that position; Ctrl+;
  clears guides.
- **Outline view (Ctrl+Y)**: wireframe rendering of the whole document
  (gray strokes, no fills), toggles back.

All click-tested in Xvfb (beta8_*.png).

---

# Session 14 — v0.12-beta.9: interface rebuilt to match Figma's layout

Per the Figma Design help category (nav/sidebar + right-sidebar articles):
- **Toolbar moved to a floating bar at the BOTTOM of the canvas** (Figma:
  "the toolbar at the bottom of the screen"). Click or keys to switch.
- **Left sidebar = Figma File tab**: PAGES section (click row to switch,
  "+ NEW PAGE" row) above the LAYERS panel with FIND; ASSETS below.
  Top-bar page tabs removed.
- **Right properties panel = Design | Prototype tabs** exactly like
  Figma's edit-access panel: Design holds position/size/rot/opacity/fill/
  auto-layout; Prototype holds the link-destination buttons (moved from
  the always-on section).
- FIGMA_PARITY.md added: full feature matrix vs the docs, including
  honest gaps (pen tool, comments, font shaping, variables UI).

Click-verified in Xvfb (beta9_*.png): layout renders, card select shows
Design tab, tab switch to Prototype, PAGE-2 link via Prototype tab
(badge appears), bottom-bar tool click (R) + drag creates R-1.
64/64 engine tests pass.

---

# Session 15 — v0.13-beta.10: Figma behavior parity wave

## Engine (69/69 tests, was 64)
- `click_figma` / `top_level_ancestor`: Figma's selection model — plain
  click = top-level object, Ctrl+click = deep select, shift toggles.
- `drill_into`: double-click descends one level toward the hit.
- `ungroup`: dissolves group/frame, children re-parented with world
  positions preserved, snapshot-undo, selects the children.
- `select_all`: page-level, or scoped inside a selected frame.
- `snap_delta`: magnetic move snapping (edge/center pull), separate from
  the visual guides.
- `set_pin`: undoable constraints change.

## App
- Selection: plain/deep/drill/Esc-to-parent all per Figma; double-click
  drill that lands on Text opens inline editing.
- Alt+drag duplicates the selection then moves the copy (Figma).
- Move drags now magnet-snap onto neighbors (4px/zoom) and show red
  guides only at exact alignment.
- Ctrl+G / Ctrl+Shift+G / Ctrl+A.
- Design tab: alignment button row (multi-select aligns selection;
  single selection aligns to page) + CONSTRAINTS picker (2x5 pins).

## Verified live (beta10_*.png + saved-file assertions)
- group -> plain click selects GROUP-0, ctrl+click selects GRAD inside.
- Esc from GRAD -> GROUP-0 ("SELECTED PARENT").
- Alt+drag -> card-copy in document.
- Ctrl+Shift+G -> children back at page level, group gone (file check).
- Constraints panel renders with active pins; click sets pin (undoable).

---

# Session 16 — v0.13.1-beta.11: selection visuals now match Figma, not Photoshop

User-reported mismatch: our selection had a rotate KNOB on a stem
(Photoshop/PowerPoint pattern). Figma has no knob at all.

## Now matching Figma exactly
- Selection chrome = tight blue outline + 4 small corner squares. Nothing
  else. No stem, no knob, no edge dots.
- **Dimension badge**: blue "W X H" pill centered under the selection,
  live-updating (Figma's size label).
- **Rotation = invisible ring outside the corners** (6..24px past each
  corner, only outside the bounds). Grab and turn — exactly how Figma
  does it. Shift still snaps to 15°.
- **Edge resize**: all four edges are grabbable (invisible 4px zones,
  corner zones win) for single-axis resize; opposite edge stays pinned;
  Shift aspect-lock applies to corners only (like Figma).

## Verified live (beta11_*.png + file assertions)
- Selection shows badge "220 X 130", no knob anywhere.
- Drag 14px outside TR corner -> rotated to 36 deg (status + inspector),
  badge still shows unrotated dims like Figma.
- Right-edge drag -> w 220->437, h unchanged 130 (file-verified).

---

# Session 17 — v0.14-beta.12: interface polish beyond Figma/Sketch

Goal: friendlier + more discoverable than both references.

## Visual refresh
- New softer theme (slate panels #24262b, deep canvas #1b1d21, hover tint).
- Floating panels get rounded corners + soft drop shadows (toolbar,
  minimap, tooltips, cards).

## Real tool icons (drawn as vectors by our own renderer)
- Cursor/hand/scale/frame-grid/rect/circle/line/hexagon/star/T icons in
  the bottom bar replace bare letters; hover highlights the slot and
  shows a TOOLTIP with the tool name + shortcut key — something neither
  Figma nor Sketch shows without a delay.

## Layers panel affordances
- Per-row fill COLOR CHIP (instant visual identification of layers).
- Hover a row -> eye + lock affordances appear; click toggles
  visibility / lock right in the list (Figma parity, discoverable).
  Verified via saved file: card visible=false, grad locked=true.

## Learnability (the "easier than Figma" part)
- "?" chip next to the toolbar + ? key -> full-screen KEYBOARD SHORTCUTS
  overlay (30 shortcuts, 3 columns). Esc or click closes.
- Inspector empty state = GET STARTED card (R/T/F/Ctrl+P/?) instead of
  a blank panel.
- Multi-selection state shows contextual hints (align row / Ctrl+G).
- Zoom widget in the top bar: [-] [100%] [+], click the % = zoom-to-fit.

All verified live in Xvfb (beta12_*.png). 69/69 engine tests unchanged.

---

# Session 18 — v0.15: the three P0s

## P0-1: Workspace split — module boundaries are now explicit crates
```
crates/
  x-core     document model, nodes, geometry, layout, variables (no IO/render)
  x-render   Vello scene encoding, paints, effects, assets, instances
  x-text     segment font + NEW real typography (FontManager)
  x-editor   selection, commands/undo, snapping, clipboard, vector editing
  x-format   .x format, SVG import/export
  x-native   facade crate (drop-in `arco_native` API for apps)
apps/
  x-designer x_native_app / render_headless / arco_native binaries
```
Dependency flow is acyclic: core <- (render, text, editor, format) <- facade <- app.
All 80 workspace tests pass; the app binary needed zero code changes
beyond font wiring thanks to the facade.

## P0-2: Real typography (x-text/font.rs)
FontManager -> TTF/OTF loading (ttf-parser, pure Rust) -> fallback chain
-> glyph mapping -> advances + KERNING -> greedy line breaking -> block
layout (ascent/descent/line-gap) -> Vello fill paths, with per-glyph
outline caching. `build_scene_full` renders Text nodes with real fonts
when a FontManager is supplied; segment font remains the no-font
fallback. App loads system fonts at startup ("loaded 8 system fonts")
and the canvas title renders in real DejaVu Sans (screenshot).
Tests: real glyph outlines, monotonic advances, AV kerning tightens,
greedy wrap respects max width, multi-line block renders 20+ glyph
paths, missing-glyph resilience. Honest note: complex-script shaping
(Arabic/Indic) needs rustybuzz next; the shape() API is swap-ready.

## P0-3: Vector editing (x-editor/vector_edit.rs)
Pen: add anchor / close path (grows node bounds). Node tool: anchors(),
anchor_at() hit test, move_anchor (rigid handle carry), delete_anchor
(re-roots subpath when MoveTo dies), convert_anchor (line <-> auto-handle
cubic), split_segment (lines at midpoint; cubics via de Casteljau at
t=0.5 — endpoints and midpoint verified exactly). All ops are undoable
ReplaceNode commands. 5 new test suites.

Workspace totals: 80 tests green (core+render 23, editor 39, text 9,
format 9). Old single-crate layout fully retired.

---

# Session 19 — v0.16: Component System 2.0 (P0) + Variables workflow (P1)

## Component System 2.0 (x-core/components.rs + component_layout.rs)
1. Typed overrides: Fill/Text/Visible/Opacity/Swap (OverrideValue),
   lossless roundtrip through the legacy string map -> .x files.
2. Component properties: Text/Bool/Swap props bound to internal node
   targets (PropRegistry::apply) — Figma component properties.
3. Boolean props toggle visibility inside instances (render-tested:
   path count drops when bg hidden).
4. Instance swap: swap overrides replace nested instances' components
   at render time (render-tested: 1-path icon -> 2-path icon).
5. Variants: "Set/Name" naming, variants_of(), switch_variant()
   (cross-set switches refused), editor-level swap_instance (undoable).
6. Nested components: override scoping stops at nested instance
   boundaries (each instance owns its subtree's overrides).
7. Detach: detach_instance resolves master + overrides into a plain
   group at the instance transform; nested instances stay live;
   editor detach_selected_instance is undoable (Delete+Insert pair).
8. DependencyGraph: master->deps edges, dependents_of() (edit-impact),
   would_cycle() (self/transitive cycle prevention).
9. THE pipeline test: Master -> Instance -> Text override ->
   measurement -> Auto Layout -> parent resize. resolve_instance_layout
   remeasures text via a MeasureFn, re-solves hug layout bottom-up
   (OK=68px wide button; "CONFIRM PURCHASE"=208px), and
   sync_instance_sizes grows the instance AND re-solves its layout
   parent (toolbar 260px, spacer pushed to x=224). Exact assertions.
10. Serialization tests: typed overrides, variant names, hug layout in
    masters, double-roundtrip stability.

## Variables P1 (engine + UI)
- Node.bindings: radius/opacity (+w/h/fontsize keys reserved) ->
  number variables; renderer resolves them live; serialized in .x.
- Variables.collections (Primitives/Semantic/... groups), catalog()
  for UI, mode_names(); strings/bools/collections/MODES now fully
  serialized (they weren't before — found by writing the roundtrip test).
- VARS tab in the inspector: mode chips (DEFAULT/DARK switch live),
  catalog grouped by collection with color swatches + number values,
  and per-row bind buttons: FILL (color var), RAD/OPA (number var).
- Live-verified: card fill -> var brand, radius -> radius-lg (saved
  file proves it), dark mode chip flips the canvas instantly.

Workspace tests: 80 -> 96 (core 7 new, render 3, editor 2, format 3+).

---

# Session 20 — v0.17: the four P1s

## P1-A: .x v2 — a serious contract (x-format/v2.rs + docs/X_FORMAT_V2_SPEC.md)
- Frozen v2 spec document (envelope: metadata/fonts/assets/uuids/
  variables/pages; styles+prototypes sections reserved).
- Schema version sniffing; stepwise migrations (migrate_v1_to_v2 is a
  pure function, deterministic: same v1 input -> identical v2 output).
- Stable IDs: 128-bit FNV-1a UUIDs per node path, backfilled once,
  preserved byte-for-byte across saves.
- Deterministic serialization: save(load(save)) byte-identical (tested).
- validate(): E001 dup ids, E002 missing component, E003 bad override
  target, E004 undefined variable binding, E005 missing proto page,
  E006 NaN/negative geometry — all covered by one test doc.
- load_x_lenient(): brace-balance truncation repair, dangling-separator
  cleanup, garbage-tail tolerance, total-garbage -> empty doc + notes.
  LIVE-TESTED: truncated document.x to 75% mid-"crash", app reopened it
  with "RECOVERED (1 pages, 1 note)".
- Partial loading: list_pages() byte-range scan without full parse,
  load_page() decodes one page subtree (thumbnail/lazy-load seed).
- App saves v2 (with validation warning counts) and loads v1/v2/corrupt.

## P1-B: Render IR (x-render/ir.rs)
Document -> build_render_tree -> RenderCommand list -> sink:
- Commands carry stable node-path keys; changed_keys() diffs two trees
  and returns exactly the moved node (partial-redraw seed, tested).
- VelloSink consumes commands into a Scene; blend layers, instance
  resolution, typed overrides, variable bindings all resolve at LOWER
  time, so sinks never see document types.
- A test drives the same commands into a non-GPU sink, proving the
  export/PDF/thumbnail path.
- The app's document canvas now renders through render_via_ir.

## P1-C: Retained UI layer (new crate x-ui)
Widgets (Button/Checkbox/TextField/Tab/Label/Slider) as retained objects:
identity, rect, state, tab_index. UiTree owns hit testing, event routing
(pointer + keyboard), FOCUS SYSTEM (Tab/Shift+Tab traversal with wrap,
Enter/Space activation, arrow-key sliders, full text editing with cursor),
and painting via backend-agnostic PaintOps. New chrome starts here.

## P1-D: Accessibility (in x-ui from day one)
- semantics(): AccessKit-shaped tree (role/label/value/focused/disabled).
- Focus ring painted for keyboard users; disabled/hidden widgets excluded
  from focus AND hit testing.
- Theme: high-contrast palette, global UI scale (paint ops scale,
  tested at 2x), reduced-motion flag.

Workspace: 113 tests green (was 96): format 19, render 30, ui 7, plus
existing. App renders documents through the IR and saves the v2 contract.

---

# Session 21 — v0.17.1: P0 module split (no monoliths left)

Every crate's lib.rs was still a monolith internally. Split along the
existing section markers into focused modules, zero behavior change:

x-core/          (445-line lib.rs -> 12 modules)
  transform, paint, pins, layout_types, node, document, geometry,
  auto_layout, variables, registry, components, component_layout
x-editor/        (1,681-line lib.rs -> 11 modules)
  selection (hit test + tree plumbing), commands (log + apply/invert),
  editor_core (Editor + ops), align, snapping, constraints,
  prototype (player + smart animate), spatial, devmode, vector_edit,
  tests_mod
x-format/        (1,090-line lib.rs -> 6 modules)
  serialize, deserialize, svg_export, svg_import, v2, tests_mod
x-render/        (645-line lib.rs -> 5 modules)
  assets, scene, stress, ir, tests_mod
apps/x-designer/x_native_app  (2,252-line single file -> 8-module bin dir)
  main (uses + mod tree), theme, state, app (input handling),
  chrome (panel painting), helpers, demo, run (event loop)

Largest file is now app.rs at 903 lines (was 4 files over 1,000; the
old pre-workspace editor.rs had been 4,000). Cross-module seams made
explicit with pub(crate): selection::find_parent_mut, commands::apply/
invert. 113/113 workspace tests green; app smoke-tested live after the
split (select/drag/undo/v2 save all working).

---

# Session 22 — v0.18: IR payoffs, retained-UI adoption, format contract hardening

## Rendering: the IR now pays rent (x-render/sinks.rs)
- thumbnail_scene(): any page -> fit-scaled preview scene. USED LIVE:
  the PAGES sidebar rows now show real page thumbnails rendered
  through the IR (visible in p2_4 screenshot).
- export_pdf(): RenderCommands -> valid single-page PDF (fills,
  strokes, text; xref table; tested for structure + content).
- SceneCache: identical frame -> ZERO re-encode (tested via
  encode_count); changed frame -> damage rects bounded to the moved
  node's old+new area (partial-redraw contract, tested).

## UI: retained concepts land in x-ui/containers.rs and the app
- ScrollView (clamp, project, ensure_visible for keyboard nav, painted
  scrollbar), Dropdown (mouse + full keyboard cycle), Menu (hover
  highlight, disabled items, shortcut column), TooltipState (delayed,
  reduced-motion aware), Modal (input trap, Esc/Enter routing, scrim).
  12 x-ui tests.
- App adoption (not just library code): right-click CONTEXT MENU on
  canvas built from x-ui::Menu (copy/duplicate/delete/z-order/group,
  disabled states, live-verified: picked DUPLICATE -> card-copy in
  saved v2 file); toolbar tooltips moved to retained TooltipState
  (old immediate-mode tooltip deleted after live-testing caught the
  double render); PaintOp -> Vello bridge in one helper.

## Format: contract hardening (x-format/v2.rs)
- MIGRATIONS: registry of stepwise pure functions; load walks
  v1 -> v2 -> ... -> CURRENT. Adding v3 = one entry.
- load_checked(): validate-before-load API returning (document,
  issues, recovery notes) — malformed .x can never crash the editor
  (garbage/truncation tested; app already uses the lenient path).

Workspace: 122 tests green (was 113).

---

# Session 23 — v0.19: the real typography stack (P0 #1)

The segment font era is over for documents. New x-text/shaping.rs:

  FontManager -> loading (TTF/OTF/TTC incl. collections) ->
  Unicode shaping (rustybuzz = HarfBuzz port) -> glyph runs ->
  BiDi (unicode-bidi) + CJK-aware line breaking -> rich-text layout
  (spans, letter spacing, line height, alignment) -> Vello fills

Delivered against the target list:
- TTF/OTF: yes (+ .ttc collections; Noto CJK auto-guaranteed at startup)
- Font fallback: per-run coverage scoring (pick_font), mixed-font runs
- Kerning: GPOS/kern via rustybuzz (AV tightening asserted in tests)
- Ligatures: GSUB ("fi" -> 1 glyph asserted; office/waffle/fjord visible)
- Unicode: full BiDi paragraph splitting into directional runs
- RTL + Arabic: joining forms verified different from isolated cmap
  forms; mixed LTR/RTL sentences split into >=3 runs
- CJK: ideograph-boundary line breaking + real Noto CJK rendering
- Variable fonts: Span.variation("wght", 700.0) -> rustybuzz set_variation
- Rich text: styled Span ranges flowing in one paragraph (mixed
  size/color/spacing), greedy wrap measured with REAL shaped widths
- Letter spacing: per-span px tracking (7x5px asserted)
- Line height: multiplier verified 2x -> exactly 2x block height
- Emoji: fonts load; color emoji rendering (COLR/bitmap) still pending —
  the one honest gap, needs a color-glyph path in the sink

Renderer: VelloSink Glyphs commands now route through encode_rich_text
(BiDi+fallback+CJK wrap) instead of shaping-lite encode_text_block.
The app therefore shapes everything on canvas; verified live (inline
edit renders ligatures).

Proof artifacts: type_specimen.png (headless GPU render: ligatures,
kerning, joined RTL Arabic, mixed direction, CJK wrap in 3 scripts,
tracking, rich text) + type_in_app.png (live editing).

18 x-text tests (was 9); workspace 131 green (was 122).

---

# Session 24 — v0.20: system font enumeration + Google Fonts

## x-text/sources.rs
- SystemFonts::enumerate(): recursive scan of platform font dirs
  (Linux/macOS/Windows paths), REAL family/style names read from each
  face's name table (typographic names preferred), .ttc collections
  enumerated per-face with indices, variable-font flag captured.
  Result in this sandbox: 211 families from 281 files.
- Style matching: find("dejavu sans", "bold") case-insensitive; empty
  style prefers non-bold/non-italic cuts (handles "Book"/"Roman"
  regular-spelled-differently faces — found by a failing test).
- FontManager::load_face_bytes(name, data, ttc_index): faces inside
  collections load individually; LoadedFont remembers its index.
- GoogleFonts: css2 API -> TTF url -> curl download -> disk cache
  (~/.cache/x-native/google-fonts) -> validated (parse check, cache
  poisoning prevented) -> loaded. Offline-safe: cache hits need no
  network; failures are Results; bogus family names error cleanly.
  NETWORK-TESTED in CI test (auto-skips offline).

## Renderer + app
- RenderCommand::Glyphs carries an optional font name; VelloSink
  resolves it through the FontManager (falls back to default). Cache
  fingerprint includes the font so font changes dirty correctly.
- Node.bindings["font"] = registered font name -> serialized in .x.
- Inspector FONT section for Text nodes: 5 system families + Roboto/
  Inter/Lobster from Google. Click = load (download if needed) + bind.
- LIVE-VERIFIED: clicked "LOBSTER (G)" -> real 392KB download into the
  cache -> canvas re-rendered "X NATIVE" in Lobster script -> saved
  file carries {"font":"Lobster 400"}. Screenshot at 188% zoom.
- Layout bug found by clicking (FONT list overlapped constraints panel,
  click hit V-PIN) and fixed by moving the section below.

24 x-text tests (was 18). Workspace: 137 green.

---

# Session 25 — v0.21: pen/node workflow in-app + Auto Layout regression suite

Status check against the 5-item priority list:
1. Architecture refactor — DONE session 21 (largest file now 955 lines).
2. Typography engine — DONE sessions 23-24 (rustybuzz/BiDi/CJK/GF).
3. Vector editing — engine existed; THIS session adds the actual tool.
4. Component 2.0 — DONE session 19 (all six capabilities + tests).
5. Auto Layout regression suite — THIS session.

## Pen/Node tool (P0 #3, the in-app half)
- Pen tool (B, pen icon in toolbar): click places anchors into a live
  vector node (semi-transparent blue fill + stroke while drawing),
  click near the start anchor closes the path, Esc finishes open.
- Closing drops straight into NODE-EDIT mode; double-clicking any
  vector node also enters it (drill-aware).
- Node edit: square anchors (corner) vs round anchors (curve) with
  control-handle stems — drag moves an anchor (one undo per gesture),
  Ctrl+click converts line<->curve, Alt+click deletes, Esc exits,
  clicking empty canvas exits.
- LIVE-VERIFIED: drew a 5-anchor polygon, closed it, dragged the top
  anchor, converted anchor 2 -> saved .x contains the real ["C",...]
  cubic alongside ["L",...] lines.

## Auto Layout regression suite (P0 #5) — x-core/layout_regression.rs
10 exact-number tests covering the risky interactions:
- 3-level hug chain solves bottom-up (row 90x30, col 102x60, exact
  child positions); 5-level nesting terminates (10 + 5*4 = 30).
- Components+text+layout: text override cascades through nested hug
  ("CHECKOUT" -> instance 128w, toolbar 180w, sibling pushed to 144;
  shrink back to "GO" -> 68/120 exactly).
- Two instances of one master resize independently (33 vs 88).
- space-between + cross-center in fixed frames (gaps 120, centers).
- CrossAlign::End with vertical direction.
- gap/padding VARIABLES cascade through nesting and re-solve when the
  variable changes (68 -> 32).
- Constraints after layout growth (right-pin follows +100).
- Degenerates: empty hug frame = pads only; single child = no gap;
  overflowing space-between clamps gap to 0; zero-size children;
  missing-master instances are no-ops. No panics anywhere.
- Idempotence: second solve is a byte-identical no-op.

Workspace: 147 tests green (was 137).

---

# Session 26 — v0.22: FULL Google Fonts catalog + families

User callout: "why not the full google fonts with their families?" —
correct; the previous version was a hardcoded 3-family demo. Now:

## Catalog (x-text/sources.rs)
- GoogleFonts::catalog(): the complete famiy list from the public
  fonts.google.com/metadata/fonts endpoint (no API key) — 1,946
  families with per-family cuts ("100".."900" + "i" italics),
  categories, and variable-font axes. Disk-cached (offline reuse)
  + in-memory per run. Zero-dependency tolerant parser.
- BUG found live: the endpoint returns pretty-printed JSON; the parser
  initially only handled compact keys ("fonts":{) and produced 0
  families in the app while tests (using compact fixtures) passed.
  Fixed whitespace-tolerant; home-cache regression test added+removed.
- GfFamily: weights(), has_italic(), is_variable(), axes.
- fetch_style()/load_style_into(): weight + ITALIC cuts, cached as
  family-weight[i].ttf. install_family() downloads every static cut.
- search(): substring family search for the browser UI.

## App: real font browser (replaces the 8-item fixed list)
- Search box ("SEARCH 2000+ FONTS"), typing filters live across
  system families + full Google catalog; scroll wheel pages results.
- Click a Google family -> download/cache/bind + WEIGHTS chip row
  appears (from catalog metadata: 100..900 + IT).
- Click a weight chip -> that exact cut downloads and rebinds.

## Live-verified end to end
- Startup: "211 system families + 1946 google families".
- Searched "pacifico" -> 1 hit -> applied (downloaded pacifico-400).
- Searched "montserrat" -> applied -> WEIGHTS row 100..900+IT ->
  clicked 900 -> montserrat-900.ttf downloaded, canvas re-rendered in
  Montserrat Black, document saved with "font":"Montserrat 900".

26 x-text tests. Workspace: 149 green.

---

# Session 27 — v0.23: dependency enforcement, x-components crate, typography fixture

## 1. Dependency graph — defined AND mechanically enforced
crates/x-native/tests/dependency_rules.rs: the graph is now a TEST.
- allowed edges declared explicitly (x-core -> nothing; components/
  text -> core; render -> core+text; editor/format -> core+components;
  ui -> text; facade -> all; app -> facade only)
- any forbidden edge or unknown crate fails the build with the edge named
- cycle detection over the real Cargo.toml files
- headless rule enforced twice: x-core has zero internal deps AND its
  sources are scanned for GPU/windowing tokens (vello::Scene, wgpu::,
  winit::) — proving core stays usable for CLI/server/web builds.

## 2. x-components extracted from x-core
New crate: model.rs (typed overrides, properties, variants, detach,
dependency graph) + layout.rs (instance resolve + text measure + hug
re-solve) + the layout regression suite moved with it. x-core shrank
and now REALLY knows nothing about components. Editor keeps a prod
dep (detach); render/format only dev-deps for tests. Facade re-exports
preserve the arco_native API unchanged (app untouched).

## 3. Typography validation fixture (the requested proof)
crates/x-native/tests/typography_fixture.rs — 5 staged tests:
  shape: every script shapes with ZERO tofu glyph ids; fi=1 glyph,
    ffi<=2; Arabic RTL; Devanagari conjuncts in expected glyph range
  measure: monotonic, size-linear (2x size = 2x width +-2%) for latin,
    Hindi, Arabic, Chinese; AV kerns tighter
  wrap: mixed-script paragraph wraps under 160px; CJK breaks with NO
    spaces present
  resize/save/reload/render: narrowing a text node dirties exactly that
    IR key; v2 save byte-stable; reloaded doc renders IDENTICAL path
    count; Hindi/Arabic/Chinese/emoji text survives byte-for-byte
  emoji: honest contract test (shapes without panic; color rendering
    still the known gap)

## Two REAL bugs the fixture caught
1. IR fingerprint omitted max_width -> resizing a text node was
   invisible to the damage differ (partial redraw would have skipped
   re-wrapping). Fixed.
2. Mixed-script runs used one font per BiDi run by majority vote ->
   Devanagari rendered as tofu in the visual specimen ("Aa..हिन्दी..中文"
   voted CJK). Fixed with per-character font-coverage segmentation in
   the shaper (whitespace glues to the current segment). dbg run now
   shows 5 runs, 0 tofu, each script in its correct font.

type_specimen.png regenerated: the exact fixture sheet — family row in
per-family faces (Inter/Roboto downloaded live), Aa AV fi ffi 123
हिन्दी العربية 中文 — all correct.

Workspace: 157 tests green (was 149).

---

# Session 28 — v0.23.1: five-item verification pass + bezier handle drag

Checklist audit (all previously built; re-verified green this session):
1. Typography 1.0 — 26 x-text tests + 5-stage fixture (shape/measure/
   wrap/resize/save/reload/render) all pass.
2. Vector Editor 1.0 — pen/anchors/convert/split/delete verified; GAP
   FOUND: bezier handles rendered but were NOT draggable. Closed:
   - engine: Editor::move_handle(anchor, outgoing, x, y) moves c2 of
     the incoming or c1 of the outgoing cubic independently (undoable
     ReplaceNode); out_handle() accessor; line segments refuse.
     Test: drag in-handle -> only c2 moves; drag out-handle -> only
     next c1 moves; both undo cleanly; 6 vector tests now.
   - app: handle hit-targets (6px/zoom, win over anchors), drag with
     gesture merge, outgoing handles now drawn too.
   - LIVE: pen triangle -> ctrl+click converts edge to curve -> dragged
     the in-handle 100px up -> top edge visibly swooped, status
     "DRAGGING IN-HANDLE OF ANCHOR 1", saved file's c1=(361,698).
3. Components 2.0 — 19 x-components tests (typed overrides, props,
   variants, swap, detach, dep graph) green.
4. Auto Layout regression — all 10 exact-number tests green.
5. Dependency enforcement — 3 architecture tests green (edges, cycles,
   headless-core token scan).

Workspace: 158 tests green.

---

# Session 29 — v0.24: paint completion, BOOLEANS, masks, styles, images, export

## Vector booleans (union/subtract/intersect/exclude) — x-editor/booleans.rs
Raster-guided clipper: flatten (cubics -> 16-seg polylines) -> even-odd
coverage grid (~360 cells max) -> EDGE-CHAINING contour extraction
(every filled cell emits boundary edges facing empty cells; edges form
closed loops by construction — outer loops AND holes) -> collinear
simplification -> PathCmd contours. node_to_path converts rects
(incl. rounded, kappa arcs) and ellipses to paths so any shape pair
booleans. Editor::boolean_selected: 2 nodes -> 1 vector, keeps A's
fill, single undo step (Delete+Delete+Insert).
Tests: set-theory areas within 8% (union 15000/intersect 5000/
subtract 5000/exclude 10000), disjoint union keeps 2 contours,
subtract-hole yields ring, editor op undoable. FIXED during dev: the
first wall-follower tracer failed on curved shapes (0 contours);
edge-chaining replaced it — fixture caught it before shipping.

## Masks — core flag + IR clip layers
Node.is_mask: a mask node clips its FOLLOWING SIBLINGS (Figma
semantics); vector/rect/ellipse masks supported; mask paints nothing
itself. IR emits PushClip/PopLayer pairs; tested (clip-fill-pop order,
n_clips > 0). Serialized in .x. "USE AS MASK" in the context menu.

## Image fit modes — Fill / Fit / Crop / Tile
ImageFit on NodeKind::Image; VelloSink implements all four (contain
letterboxing, cover-crop with clip, tiling with clip). Serialized;
fit changes dirty the damage cache (tested).

## Styles — Document.styles registry
Style::Paint/Text/Effect named presets on the document (engine level;
UI application next round).

## Paint/effects UX (Design tab)
STROKE width -/+ (auto-white color on first +), GR chip toggles a
linear gradient from the current solid, S chip toggles drop shadow.
All live-verified ("SHADOW ON", stroke chips).

## Component UX
Instance selection shows COMPONENT section: variant chips (same-set
switching via swap_instance) + DETACH button.

## Export
Ctrl+E = SVG (existing), Ctrl+Shift+E = PDF through the render IR
(export.pdf written, %PDF-1.4 verified).

## Context menu grew
UNION/SUBTRACT/INTERSECT/EXCLUDE (2 shapes) + USE AS MASK; right-click
inside a multi-selection now PRESERVES it (bug found live: right-click
was re-selecting under the cursor and dropping the pair).

Live-verified end-to-end: card+dot union -> BOOL-1 vector on canvas
(pill+bump silhouette), file has bool node and inputs gone; stroke+
shadow toggles; PDF export. Workspace: 166 tests green (was 158).

## Session 30 (v0.25 era — crates 0.17.0 / x-components 0.23.0)

Wave: close the top "honest gaps" — Styles UI, image placement controls, PNG export, first git commit.

### Built
| Feature | Where | Proof |
|---|---|---|
| `apply_style()` engine op (Paint→fill, Text→font binding+size, Effect→effects) | x-core/document.rs | 3 new unit tests |
| Styles persistence in .x (v1 payload + v2 envelope), deterministic byte-stable | x-format serialize/deserialize | `styles_roundtrip_through_x_format` (save(load(save)) byte-identical) |
| STYLES inspector section: +P/+T/+FX create-from-selection, wrapping apply chips w/ paint swatches | chrome.rs + app.rs @ TOP_H+362 | s30_4 (dot→Brand/Blue live), status "STYLE CREATED: PAINT/3", "TEXT/3" |
| IMAGE inspector section: FILL/FIT/CROP/TILE chips + REPLACE (cycles asset library) | chrome.rs + app.rs @ TOP_H+210 | s30_3 (fit vs tile pixels), "IMAGE -> SUNSET", file shows `"asset":"sunset","fit":"tile"` |
| PNG export (Ctrl+Alt+E): offscreen wgpu texture at page size via render IR, readback → export.png | helpers.rs `export_png` | export.png 1600×1000 RGBA, real rendered pixels incl. checker image + gradient + text |
| `Assets::names()` for the replace picker | x-render/assets.rs | used live |
| Demo doc seeds 2 styles + an image node so the UI shows real content | demo.rs | s30_0 baseline |
| App styles load/save wired (Ctrl+S/Ctrl+O carry `Document.styles`) | run.rs | document.x contains Brand/Blue, Elev/Card, Text/3 after reload |

### Bugs found & fixed live
- IMAGE section at TOP_H+176 collided with the fill palette rows → moved to TOP_H+210 (screenshot caught it).
- Style chips overflowed the single row with 3+ styles → wrapping rows (chrome + hit-test kept in lockstep).

### Tests: 170 workspace tests green (was 166).

### Honest remainder
- Style *renaming/deleting* not in UI (create+apply only); text style ls/lh captured but only font+size re-applied.
- REPLACE cycles loaded assets; no file-open dialog (no native file picker dependency yet).
- PNG export is 1x page size; no scale selector (@2x etc.).
- Color emoji sink path still absent; exact-curve booleans still raster-guided.

## Session 31 (v0.26 era — crates 0.18.0 / x-components 0.24.0) — REVIEW HARDENING PASS

User pasted a P0 hardening review. Every item addressed with tests + live proof:

### 1. Styles UI completed (P0)
- `bind_style()` — applying a style now LINKS the node (`bindings["style:paint|text|fx"] = name`), not just copies values.
- `resolve_styles(root, styles)` — the "style mutation → all consumers update" pass; returns update count; runs on file open.
- App: chip click = bind+apply; **Shift+chip = redefine style from selection → propagates to all pages**.
- LIVE: bound card+dot to Brand/Blue, changed grad to yellow, Shift+clicked chip → status "STYLE BRAND/BLUE REDEFINED -> 2 CONSUMER(S) UPDATED", both nodes turned yellow on canvas; bindings + mutated def verified in document.x.
- TEST: `style_mutation_updates_all_consumers` (deep-nested consumer, text style, unbind stops updates).

### 2. Image placement designer-complete (P0)
- New `ImagePlacement { focal, scale, flip_h, flip_v }` on `NodeKind::Image`; VelloSink honors it in all 4 fit modes (focal-anchored zoom, mirror transforms, all now clip to box).
- Inspector: focal X/Y % steppers, SCALE % stepper, FH/FV toggles, RESET — plus existing fit chips + REPLACE.
- Persisted in .x (compact: omitted when default). LIVE: crop+focal 0.7+flipH+scale 1.2 → pixels changed (RMSE 0.23), saved file shows `"fx":0.7,"fliph":true,...`, survives reload.

### 3. Boolean facade API (architecture decision)
- `boolean(op, &PositionedPath, &PositionedPath) -> BooleanResult` + `boolean_with(Backend, ...)`; `Backend::{RasterGuided(default), Exact(falls back, reserved)}`.
- `boolean_selected` now routes through the facade — the app no longer knows the backend. TEST: `facade_api_is_backend_agnostic`.

### 4. Mask semantics matrix (5 new tests, mask_semantics.rs)
- mask over image ✅, vector ✅, group subtree (all fills inside clip) ✅,
- component instance w/ auto-layout content under mask ✅ (the review's hard case),
- full chain: boolean→gradient fill→drop shadow→mask→SVG (has gradient+path)→PDF (has clip ops) ✅.

### 5. Export visual regression (the big one)
- New `export_regression` bin: 7 fixtures (gradients/masks/booleans/text/components/effects/vectors) × 3 renderers — real GPU canvas PNG, SVG, PDF.
- `tools_visual_compare.sh`: rasterizes SVG (rsvg-convert) + PDF (ghostscript), RMSE vs canvas, threshold-gated.
- **Immediately caught 3 real exporter bugs, all fixed:**
  a. SVG export silently dropped masks → now emits `<mask>` defs + wrapping groups (RMSE masks 0.76 → 0.0045)
  b. SVG export silently dropped component instances → now resolves via registry (components 0.97 → 0.0013)
  c. PDF sink ignored PushClip/PopLayer → now emits `q <path> W n` / `Q` (masks PDF 0.28 → 0.02)
- Final RMSE table: all fixtures ≤0.02 except gradients-PDF 0.12 (PDF sink flattens gradient to first stop — known) and text 0.17-0.18 (font parity — known).

### 6. Undo/redo integration test (undo_redo_chain.rs)
- Realistic chain: create×4 → auto-layout(gap-var) → make_component → variable fill bind → boolean union → mask flag → image crop/focal/flip → style bind → SVG+PDF export.
- Full undo ⇒ byte-identical to empty snapshot; full redo ⇒ byte-identical to final; save→load→save byte-identical; placement/mask/binding spot-checks pass.
- Added `Editor::replace_node()` public undoable whole-node swap.

### Tests: 178 workspace tests green (was 170: +5 mask matrix, +1 undo chain, +1 style mutation, +1 facade, was-166 baseline shifted by earlier suite).
### Roadmap updated (~/X-Native_FEATURE_ROADMAP.md v0.26 addendum).

### Honest remainder
- `Backend::Exact` is a facade slot, NOT an implementation — still falls back to raster-guided.
- Text export parity (0.17 RMSE) needs shaped-glyph outlines in SVG/PDF sinks.
- PDF gradients flatten to first stop color (0.12 RMSE on gradients fixture).
- Focal/scale steppers are ±10% buttons, not a drag-crop overlay on canvas; image opacity uses the node's generic opacity control (works, but no dedicated image-opacity slider).
- Env note: typography_fixture requires fonts-noto-core/cjk reinstall after sandbox reset (now in setup list).

## Session 32 (v0.27 era — crates 0.19.0 / x-components 0.25.0) — REVIEW ROUND 2

Same review re-pasted; prior items re-verified, remaining gaps CLOSED:

### 1. Exact boolean clipper — Backend::Exact is now REAL (was a fallback stub)
- New `crates/x-editor/src/clip.rs`: Greiner–Hormann polygon clipper with
  analytic segment intersections, entry/exit marking, contour tracing,
  containment/disjoint fast paths, exclude = (A−B)⊎(B−A), degeneracy
  handling via perturb-retry (vertex-on-edge), runaway budget guard.
- `Backend::Exact` is now the DEFAULT; falls back to raster-guided only on
  topology it can't trace (multi-contour operands, self-intersections).
- Precision contract in tests: raster ≈8% tolerance, exact <0.1% squares,
  <1% flattened curves. Review's stress cases covered: thin slivers (1px
  overlap), repeated boolean chains (6 unions, no drift), huge (1e6) and
  tiny (1e-3) coordinate ranges, containment/disjoint. 5 new clip tests.
- LIVE: card ∪ dot (disjoint) via context menu → BOOL-0 with exactly 2
  contours / 132 line segs in document.x, fill preserved.

### 2. Images fixture + image export (review's missing fixture)
- SVG: `export_svg_with_assets` embeds PNGs as base64 data URIs,
  preserveAspectRatio maps Fill/Fit/Crop; std-only base64 encoder.
- PDF: `export_pdf_with_assets` embeds real DeviceRGB image XObjects with
  per-fit-mode CTM placement + box clip; xref stays valid (N image objs).
- Export regression now 8 fixtures × 3 renderers; images RMSE: SVG 0.10,
  PDF 0.11 (crop/tile approximations; under 0.30 gate; montage verified
  visually — all four fit modes render in all three sinks).

### 3. Variant step added to the undo/redo chain test
- Chain is now the review's full list: Create → Auto Layout → Component →
  **Variant (Button/Default → Button/Primary via switch_variant +
  ReplaceNode)** → Variable → Boolean → Mask → Image crop → Style → Export
  → full undo (byte-exact empty) → full redo (byte-exact final) →
  save/reload (byte-identical). Depth assertion raised 9 → 13.

### 4. Styles browser grouped per review sketch
- PAINT STYLES / TEXT STYLES / EFFECT STYLES sections with headers;
  bound style chip highlights (accent) on the selected node.
- `styles_layout()` = single geometry source shared by painter AND
  hit-test (kills the drift bug class found last session).
- LIVE: sections render, "APPLIED STYLE: BRAND/BLUE" chip turns accent.

### Tests: 183 workspace green (was 178: +5 clip.rs; facade test upgraded w/ precision contracts; chain test extended in place).

### Honest remainder
- Exact clipper scope: single-contour simple polygons (multi-contour and
  self-intersecting operands fall back to raster). Curves flatten at 16
  segs/curve — result contours are polylines, not re-fit beziers.
- SVG image export ignores focal/flip placement (preserveAspectRatio has
  no focal-point equivalent; needs viewBox cropping math).
- PDF tile mode approximated as crop; PDF images uncompressed (big files).
- Text export parity still the largest visual delta (0.17 RMSE).

## Session 33 (v0.28 era — crates 0.20.0 / x-components 0.26.0) — .SKETCH IMPORTER (user-pasted draft, audited + landed)

User pasted a draft `.sketch` importer and asked to "audit and add this".
The draft referenced infra that didn't exist (`crate::json`, `crate::zipfile`)
— both built; the importer itself had 6 real bugs, all fixed with
regression tests:

### Audit findings (draft bugs → fixes)
1. **Opacity never read** — draft called `.num()` on the `contextSettings`
   OBJECT (always None → everything opaque). Fixed: `.get("opacity").num()`.
   Regression test: 0.35 opacity imports as 0.35.
2. **Artboard/group/symbol positions dropped** — `Node::frame/group/
   component` constructors position at (0,0); draft never set x/y for
   those classes. Fixed + test (artboard at 100,50; nested group 25,35).
3. **Text overrides not render-effective** — draft stored raw "Save";
   our IR requires the `"text:"` prefix keyed by target layer id. Fixed;
   test drives `build_render_tree` and asserts the override text reaches
   a Glyphs command.
4. **Gradient anchors unscaled** (caught LIVE, not by unit tests: first
   import rendered the hero rect as a solid purple block) — Sketch
   anchors are normalized 0..1; our Paint uses node-local pixels. Fixed:
   scale by frame w/h. Screenshot before/after shows orange→purple
   gradient restored.
5. **shapeGroup children invisible** — Sketch styles live on the
   shapeGroup; child shapePaths have none, so combined shapes imported
   fully transparent. Fixed: group fill propagates to fill-less child
   vectors + test.
6. **Pages imported 0×0** — breaks zoom-to-fit/thumbnails/export. Fixed:
   page auto-sizes to content envelope (+margin, 800×600 min) + test.
   Also: cleaned the confusing `"artboard" | "symbolMaster" if class ==
   "artboard"` match arm, dead `let _ = target_id` block, unused-var
   warnings; text nodes default to black (not transparent) fill; empty
   sketch (no pages) is an Err not a silent empty doc; counter bump on
   hidden layers removed (ids come from do_objectID anyway).

### Infrastructure built (draft assumed, didn't exist)
- `x-format/src/json.rs` — the JSON parser EXTRACTED from deserialize.rs
  (shared, not duplicated; deserialize.rs now imports it).
- `x-format/src/zipfile.rs` — minimal ZIP reader: EOCD scan, central
  directory walk, stored + deflate (miniz_oxide 0.8, already in tree via
  png). Zip64/encryption → clean Err. 3 tests incl. real deflate entry.

### App integration
- Ctrl+I imports `import.sketch` → appends pages; help overlay updated.
- LIVE: generated a deflate-compressed .sketch (python zipfile, like real
  Sketch output): artboard w/ gradient hero + 60%-opacity dot + text +
  10-point star + Pill symbol master + instance w/ override → status
  "IMPORTED 1 SKETCH PAGE(S)", PAGE-A in pages panel, full layer tree in
  LAYERS, canvas renders all shapes, instance shows "IMPORTED" (override
  applied through the component registry), master renders "OK".
  Ctrl+S: everything persists to .x (verified: linear fill, artboard at
  40,40, `{'pill-label': 'text:IMPORTED'}`).

### Tests: 196 workspace green (was 183: +10 sketch, +3 zipfile).

### Honest remainder
- Angular gradients → linear fallback; radial radius from anchor distance
  (Sketch's ellipse ratio ignored).
- Bitmap layers reference asset by name only (no embedded-image decode
  from the zip's images/ dir yet — Assets loader wants PNG on disk).
- Per-character text styling, shadows/blurs, boolean groups, resizing
  constraints: dropped (documented in module header).
- Radial gradient center should also scale by frame — same normalized
  space; scaled now, but Sketch's separate ellipse aspect is not.

## Session 34 (v0.29 era — crates 0.21.0 / x-components 0.27.0) — TEXT EXPORT PARITY

The declared "top visual gap" (text RMSE ~0.17 vs ≤0.02 for other fixtures): closed.

### Architecture: ONE text-geometry pipeline for all three sinks
- `x-text::glyph_outlines()` — shaping+wrap+align now returns positioned
  `OutlineGlyph{path, transform, color}`s; `encode_rich_text` is a thin
  Vello adapter over it (canvas unchanged by construction).
- `x-text::node_text_outlines()` — THE canonical Text-node→glyphs mapping
  (0.72em factor, 1.2 line height, font binding resolution). Canvas sink,
  PDF exporter, SVG exporter all call this one function, so text geometry
  cannot drift between sinks.
- PDF: `export_pdf_full(..., fonts)` emits each glyph as filled bezier
  path ops (composed node-world × glyph-local transforms into the flipped
  CTM). No more Helvetica Tj (kept as fallback when fonts absent).
- SVG: `export_svg_full(..., text_outliner)` — outliner is an INJECTED
  callback because x-format must not depend on x-text (dependency
  direction is test-enforced); `arco_native::svg_text_outliner(&fonts)`
  is the canonical implementation in the facade. No more
  `<text font-family="monospace">` guessing.

### Bug found by the parity work
- `emit_path` (PDF) converted quadratic beziers by using the quad control
  for BOTH cubic controls — visibly fattened TrueType glyph curves. Fixed
  with the exact quad→cubic elevation (c1=p0+2/3(q−p0), c2=p+2/3(q−p)),
  which needed current-point tracking in the emitter.

### Measured (visual regression, RMSE vs GPU canvas)
- text SVG: 0.168 → **0.0038** (44×)
- text PDF: 0.183 → **0.018** (10×; also enabled gs TextAlphaBits=4 — the
  residual was ghostscript's unantialiased rasterization, which also
  dropped booleans/components/effects/vectors PDF numbers ~3×)
- all 8 fixtures ≤0.02 except gradients-PDF 0.119 (PDF sink still
  flattens gradients to first stop) and images 0.10-0.11 (crop/tile
  approximations) — both pre-existing, documented.

### Tests: 200 workspace green (was 196) — new text_export_parity.rs:
1. all_three_sinks_share_glyph_geometry (same count, real curves, fi
   ligature survives INTO the export path)
2. svg_export_emits_outlines_not_font_tags (no <text>, no font-family,
   ≥1 path/glyph)
3. pdf_export_emits_outlines_not_helvetica (no Tj with fonts, fallback
   keeps Tj without)
4. wrapping_matches_between_canvas_and_exports (narrow box wraps: same
   glyphs, taller block, two baseline bands)

### Live proof
- App Ctrl+E: export.svg has per-glyph `<path d="M 1.54 9.42 …">`,
  ZERO font-family/text tags; rasterized crop shows "X NATIVE" identical
  to canvas. Ctrl+Shift+E: export.pdf content stream has 0 Tj ops, 12
  curve ops, 10 fills.

### Honest remainder
- SVG/PDF glyph outlines mean larger files and no text selectability in
  exported PDFs (design-tool tradeoff; a ToUnicode-mapped embedded-font
  mode would restore selectability — future work).
- Gradient-fill TEXT exports as solid (first stop) in both exporters —
  same gradient flattening as shapes in PDF.
- PDF gradients + image crop/tile approximations unchanged.

## Session 35 (v0.30 era — crates 0.22.0 / x-components 0.28.0) — IMPORT IR + INTEROPERABILITY

Review: "Import Fidelity + Interoperability … with a common intermediate
representation. Don't let Sketch → Node, SVG → Node, Figma → Node each
develop completely different semantics."

### Built: the Import IR layer (x-format/src/import_ir.rs)
- `ImportDoc { pages: Vec<ImportNode>, assets, source }`,
  `ImportNode { id?, kind, x/y/w/h, rotation, fill?, stroke?, opacity, visible, children }`,
  `ImportKind` (Frame/Group/Rect/Ellipse/Line/Text/Path/Image/Component/Instance).
- `lower(ImportDoc) -> Document` — THE single place for shared import
  semantics: id sanitize + global dedupe (collision → "-2" suffix),
  generated ids for missing ones, kind-default fills (text=BLACK never
  transparent), opacity clamp [0,1], NaN/∞ scrub of all geometry, page
  auto-size to content envelope (+40px, 800×600 min), instance text
  overrides encoded render-effective ("text:" prefix), one rotation
  convention. 4 unit tests.

### Refactored to emit IR (parse-only importers)
- Sketch: convert_layer now returns ImportNode; "text:" encoding moved
  OUT of the importer into lower(); shapeGroup fill propagation stays in
  the importer (source-format quirk, documented rationale); embedded
  images/*.png now carried as ImportDoc.assets. All 10 existing sketch
  tests pass UNCHANGED — proof the refactor preserved semantics.
- SVG: parse_children builds ImportNodes; local id counter deleted
  (lower() owns ids now); public import_svg -> Node kept via
  single-page lowering. All existing svg tests pass unchanged.

### New importers (both IR-native from day one)
- **Figma** (figma.rs): REST-API JSON (`GET /v1/files/:key` shape — the
  format Figma officially exports; binary .fig is proprietary and NOT
  attempted, documented in the module header). Covers CANVAS/FRAME/
  GROUP/COMPONENT/INSTANCE (componentId→name via file components map),
  RECTANGLE+cornerRadius, ELLIPSE, LINE, TEXT.characters, VECTOR
  fillGeometry (parsed with the SHARED svg d-parser — one parser),
  solid + linear/radial gradient fills (normalized handles scaled to
  pixels — the exact bug class the Sketch importer hit in s33, avoided
  by design here), absolute→parent-relative coordinate conversion,
  content envelope shifted to origin. 2 tests.
- **PNG** (png_import.rs): IHDR-only parse (ONE pixel decoder stays in
  x-render Assets), Image node at natural size + asset bytes in the IR.
  2 tests.

### Conformance suite (import_conformance.rs, 5 tests)
- One shared `assert_conformant()` contract over sketch/figma/svg/png:
  unique sanitized ids, black text, sized pages, clamped opacity, finite
  geometry, byte-stable .x round trip, non-empty render IR.
- `same_scene_same_semantics_across_importers`: the SAME logical scene
  (red 100×50 rect at (10,20) + duplicate-id rect + text) authored in
  Sketch zip, Figma JSON, and SVG — asserts identical fills, sizes, id
  dedup behavior, and text defaults across all three. This test is the
  review directive made executable.

### App
- Ctrl+I now imports the first of import.{sketch,figma.json,svg,png};
  PNG also registers pixels with the live asset store. Help updated.
- LIVE: figma json → "IMPORTED 1 FIGMA PAGE(S)", page 0:1 renders
  gradient card + 70% ellipse + "FROM FIGMA" + triangle vector
  (screenshot s35_figma_imported.png); png → "IMPORTED 1 PNG PAGE(S)";
  Ctrl+S persists all three page sources into one .x (verified: linear
  fill, opacity 0.7, image node).

### Tests: 213 workspace green (was 200: +4 IR, +2 figma, +2 png, +5 conformance).

### Honest remainder
- PDF import: not built (next matrix slot; plan: content-stream subset,
  re-import our own exports first).
- Figma: effects/auto-layout/constraints/image fills not mapped (module
  header documents); .fig binary out of scope by design.
- Sketch embedded-image assets carried in IR but the app shell only
  registers import.png pixels so far (sketch/figma image bytes → Assets
  wiring is a small follow-up).
- Import IR has no diagnostics channel yet (skipped-node counts would
  make fidelity measurable per file).

## Session 36 (v0.31 era — crates 0.23.0 / x-components 0.29.0) — ASSET MANAGER

Review: "Build a proper asset manager … Then .x can reference asset://abc123
instead of relying on filenames."

### Built: content-addressed AssetStore (x-core/src/assets.rs)
- `AssetRecord { id: "asset://<fnv1a128-of-bytes>", hash, mime, kind
  (Image/Font/Svg/Other), dimensions, bytes, source (Embedded/External),
  name (display only — NEVER identity) }`.
- Mime sniffed from magic bytes (png/jpeg/gif/webp/ttf/otf/woff2/svg),
  extensions never trusted. Dimensions from header-only probes (PNG IHDR,
  JPEG SOF scan, GIF screen descriptor) — pixel DECODING stays in
  x-render (one decoder in the codebase).
- `register()` dedups by content hash: same bytes -> same id, one copy,
  regardless of filename. `retain_used()` = GC. 5 unit tests + shared
  b64 module (encoder moved out of svg_export, decoder added, 2 tests).

### Wired through the whole pipeline
- `Document.assets: AssetStore`; .x serializes EMBEDDED records
  (base64) — External stays machine-local by design. Deserializer
  re-derives ids from content (tampered records can't hijack ids).
  Byte-stable round trip WITH assets asserted.
- Import IR `lower()` registers ImportDoc.assets and rewrites Image refs
  to asset:// ids — importers stay parse-only; the semantics live in the
  ONE shared lowering, per the interop architecture.
- Sketch importer now reads the REAL bitmap reference (layer.image._ref
  = "images/<sha>.png") instead of the layer display name — the exact
  gap the review named.
- x-render `Assets::sync_store()` decodes store records into the GPU
  cache under their asset:// ids (idempotent); `load_png_bytes` split
  out of the file loader.
- App: store field on App; Ctrl+S carries it; Ctrl+O adopts + decodes;
  Ctrl+I merges imported stores and reports "N embedded asset(s)".

### End-to-end tests (asset_manager.rs, 3) — hand-built VALID png
(zlib stored-blocks + real crc32/adler32) so the decode path is real:
1. sketch embedded bitmap -> asset:// uri + store record (mime, dims,
   Embedded source, zip-stem name)
2. **portability**: import -> save .x -> reload IN A VACUUM (no files) ->
   asset resolves, byte-stable, render IR references the uri, VelloSink
   renders from the synced cache
3. **dedup**: two bitmaps w/ identical bytes under different names ->
   ONE record, both nodes share the id

### LIVE (screenshots s36_asset_store.png + reload crop)
- Sketch zip w/ embedded 48×32 png referenced twice under two names →
  Ctrl+I → both images render; Ctrl+S → document.x holds exactly ONE
  asset record (3796 b64 chars), both nodes point at
  asset://2e248a4b…; **deleted import.sketch**, Ctrl+O → images still
  render purely from the .x store. Portability demonstrated literally.

### Tests: 223 workspace green (was 213: +5 store, +2 b64, +3 e2e; png
import test updated to assert asset:// as the new correct behavior).

### Honest remainder
- Fonts/SVG assets: store accepts them (kind+mime) but nothing registers
  fonts yet (Google-font cache still file-based).
- Only image/png decodes in sync_store (jpeg/gif/webp sniffed+stored but
  the render cache has no decoder for them — needs a jpeg decoder dep or
  honest "unsupported" placeholder).
- Legacy filename refs still work (assets/ dir scan) — no migration
  pass rewriting old documents to asset:// yet.
- REPLACE inspector button cycles the render cache (which now includes
  store ids) but has no store-aware picker UI showing name/mime/dims.
- fnv1a128 is fine for dedup identity, not cryptographic integrity.

## Session 37 (v0.32 era — crates 0.24.0 / x-components 0.30.0) — BOOLEANS 2.0 + PDF QUALITY

Review round 3. Status check first: TEXT EXPORT PARITY was already
delivered in v0.29 (the review quoted the pre-v0.29 0.17-0.18 numbers) —
re-verified this session: text SVG 0.0038 / PDF 0.018, one glyph pipeline
(node_text_outlines) across all three sinks. COLOR EMOJI explicitly
deprioritized by the review — not touched.

### 1. Exact vector booleans 2.0 (bezier_clip.rs — EXTENDS, not replaces)
- Review target pipeline implemented: Bezier → bezier intersections
  (recursive subdivision, bbox pruning, flatness cutoff, param-space
  tracking) → topology (Greiner–Hormann over CURVE CHAINS, de Casteljau
  splits at cut params — exact) → **Bezier output**.
- Lines ride as exact 1/3-mark degenerate cubics so the algorithm is
  uniform; output emits LineTo for numerically-straight segs, CurveTo
  for real curves. Flattening only ever used for point CLASSIFICATION,
  never for output geometry.
- Backend ladder: BezierExact (new DEFAULT) → Exact polygon → Raster,
  same stable facade; perturb-retry on degeneracy at the bezier tier too.
- Tests (7 new): circle∪rect area within 0.2% + curves preserved;
  circle∩circle lens vs ANALYTIC formula within 0.5%; 4 successive
  subtracts keep arcs as genuine curves (the review's degradation case);
  pure-line input -> pure-line output; containment/disjoint return
  UNTOUCHED cubics (donut: 8/8 segments still curves, area vs π·(R²−r²)
  within 0.1%); path round trip; end-to-end via boolean_selected incl.
  second-generation boolean still curved.
- LIVE: grad-rect ∪ circle → BOOL-0 saved with 8 CurveTo + 4 LineTo
  (old default: ~130 LineTo, zero curves). Screenshot s37.

### 2. PDF image/gradient quality (all three review items)
- Gradients: real /ShadingType 2 (axial) & 3 (radial) dicts; 2-stop via
  FunctionType 2, multi-stop via stitching FunctionType 3; path-clipped
  `sh` paint; transforms applied to coords, radii scaled. gradients PDF
  RMSE 0.119 → **0.0075** (16×).
- Tile: real repeat loop matching the canvas sink (was crop approx).
- Images: /Filter /FlateDecode (miniz_oxide zlib) — was uncompressed;
  images PDF RMSE 0.109 → **0.037** (fit-mode parity also improved).
- 3 tests: shading dicts + stitching fn present, radial Type 3, tile
  emits 16 Do ops for a 4×4 grid + FlateDecode marker.

### Regression table after this session (RMSE vs GPU canvas):
| fixture | SVG | PDF | | fixture | SVG | PDF |
|---|---|---|---|---|---|---|
| booleans | 0.0026 | 0.0056 | | images | 0.1008* | 0.0371 |
| components | 0.0013 | 0.0035 | | masks | 0.0045 | 0.0201 |
| effects | 0.0009 | 0.0028 | | text | 0.0038 | 0.0183 |
| gradients | 0.0034 | 0.0075 | | vectors | 0.0018 | 0.0046 |
(*images SVG = preserveAspectRatio crop/tile approximation — now the
single largest remaining export delta.)

### Tests: 233 workspace green (was 223: +7 bezier/facade, +3 pdf quality).

### Honest remainder
- Bezier clipper scope: single-contour non-self-intersecting operands
  (multi-contour falls to polygon tier). Tangency/overlap-arc topology
  falls back rather than resolving exactly.
- Curve-curve intersections are subdivision-refined (~1e-6), not
  closed-form; params dedup at 1e-4 which could merge very close twins.
- SVG image fit modes (0.10) now the biggest export gap; PDF sweep
  gradients unsupported (fall back to flatten); color emoji unchanged.

## Session 38 (v0.33 era — crates 0.25.0 / x-components 0.31.0) — CANONICAL IMAGE TRANSFORM + STYLES UI COMPLETION

### 1. Canonical image transform model (review's #1 export fidelity item)
- New x-core/image_transform.rs: `resolve_image_placement(fit, placement,
  w, h, iw, ih) -> ResolvedImage { draws: Vec<Affine> }` — THE single
  mapping from fit/focal/zoom/flip/tiling to image-pixel→node-local
  affines. 5 unit tests (fill stretch, fit letterbox, crop+focal pinning,
  tile grid + zoom halving the grid, flip mirroring).
- All three sinks now consume it and ONLY compose their outer transform:
  * Vello sink: 45 lines of per-fit math → 8 lines (clip + draw loop).
  * SVG export: preserveAspectRatio GUESS replaced by exact
    `<image transform="matrix(…)">` per draw inside a clipPath.
  * PDF export: per-fit CTM math replaced by draw-affine × pixel→unit
    conversion; tiling falls out of the shared model automatically.
- **images-SVG RMSE 0.1008 → 0.0229** (4.4×) — the review's number-one
  gap closed without a single sink-specific special case. PDF stays
  0.037 (remaining delta = ghostscript raster interpolation).

### 2. Design Styles UI — the P0 product work
Engine additions (x-core, tested in style_management_rename_detach_usage):
- `style_usage(node, name)` usage counts; `detach_style(node, key)`
  (values kept); `rename_style(styles, pages, from, to)` — registry +
  EVERY consumer rebound atomically, collision/missing-source refused.
App workflow (all live-verified):
- Grouped browser (PAINT/TEXT/EFFECT) now has: SEARCH box
  (Focus::StyleSearch, live filter — "elev" filters to ELEV/CARD),
  usage count on every chip ("PRIMARY 2"), selected-style highlight.
- Click chip = apply+link; Shift+click = redefine-from-selection →
  all consumers update (live: "2 CONSUMER(S) UPDATED", card+dot flipped
  together); Ctrl+click = select for management.
- Management row (REN/DUP/DEL/DET): RENAME with inline buffer
  (live: "RENAMED BRAND/BLUE -> PRIMARY (2 CONSUMER(S) REBOUND)",
  document.x shows both bindings now 'Primary'); DUPLICATE ("Primary
  copy", collision-safe naming); DELETE detaches all consumers first
  (values kept), red-bordered button; DETACH unlinks selected layer only.
- Create was already there (+P/+T/+FX). Persistence verified in .x.

### Tests: 239 workspace green (was 233: +5 image transform, +1 style management).

### Honest remainder
- "Local vs library" styles: NOT built — no multi-document library
  concept exists yet; all styles are document-local (documented, needs
  a library-file design first).
- Rename UI is a status-line buffer, not an inline text field on the
  chip; management targets one style at a time (Ctrl+click).
- images-SVG residual 0.023 = rsvg bilinear vs vello nearest sampling
  on upscaled checkers; images-PDF 0.037 similar. True zero needs
  matching resample filters, out of exporter scope.
- DEL has no undo (bindings detach through direct mutation, not the
  command log) — style ops should become undoable commands eventually.

## Session 39 (v0.34 era — crates 0.26.0 / x-components 0.32.0) — LIBRARIES + ASSET BROWSER

### 1. Versioned library system (the review's deliberate architecture)
Engine (x-core/library.rs, 3 tests):
- `Library { library_id, name, version, styles, variables, components,
  assets }` — the review's exact model; `LibraryRef` parses
  `library://<lib>/style/<name>` (names may contain slashes:
  "Primary/500"); consumers bind the URI, never copy the object
  (test: doc.styles stays EMPTY, binding holds the library uri).
- `LibraryDependency { library_id, resolved_version, source_path }` —
  documents PIN the version they were designed against.
- Versioning-first: `resolve_library_styles` reads only the pinned
  snapshot — a newer .xlib on disk CANNOT change a document
  (tested explicitly). Missing library freezes values, never blanks.
- `diff_library` = "Review changes" (StyleAdded/Removed/Modified,
  VariableChanged, ComponentAdded/Removed); `accept_update` = "Accept"
  (repin + snapshot swap + consumer re-resolution). The review's
  "Update available → Review → Accept" is literally the API.
Persistence (x-format, 2 xlib + 2 lifecycle tests):
- `.xlib` artifact: save_xlib/load_xlib, byte-stable, REUSES the .x
  style/node/asset encoders (extracted style_json/parse_style_v — one
  serialization dialect, no drift).
- .x now carries `"libraries":[{dep + inline snapshot}]` — documents
  stay SELF-CONTAINED: reload + render identically with no .xlib
  anywhere (lifecycle test proves it), byte-stable round trip.
- library_lifecycle.rs: full chain xlib→link→pin→v2 appears (no silent
  change!)→diff→accept→repin persists.

### 2. Asset browser (the review's designer-facing P0)
- Shift+A overlay over the content-addressed store: REAL decoded
  thumbnails (GPU cache images drawn clipped into tiles), captions
  name+dims+usage-count (cross-page), selected-tile highlight.
- SEARCH (name or mime, live filter), PLACE (new image node at natural
  size, canvas), REPLACE (selected image layer swaps to selected
  asset), RENAME (display name only — identity stays content-derived),
  DEL UNUSED (cross-page usage sweep via collect_asset_ids/retain_used).
- Store helpers in x-core: rename/collect_asset_ids/asset_usage.
- Shared layout fn (asset_layout) for painter + hit-test — same
  anti-drift pattern as styles_layout.
- LIVE: imported sketch w/ 2 embedded pngs → browser shows both thumbs
  w/ "AAA 64X48 1X" captions; tile click → status "aaa | image/png |
  64x48 | used 1x"; PLACE → "PLACED IMAGE-1 ON CANVAS"; RENAME →
  "ASSET RENAMED"; DEL UNUSED correctly kept assets used on ANOTHER
  page; Ctrl+S: asset:// refs + renamed display name persist.

### Tests: 246 workspace green (was 239: +3 library, +2 xlib, +2 lifecycle).

### Honest remainder
- No UI yet for the library flow itself (link/review/accept panel) —
  engine + persistence complete, app surface is the next wave; the
  styles browser doesn't yet LIST library styles alongside local ones.
- Library components/variables/assets resolve structurally but only
  styles have consumer re-resolution; component instances from
  libraries need registry merging.
- Asset browser: no drag-onto-canvas (PLACE puts at fixed spot), no
  sort toggles (fixed name sort), no grid scrolling (first ~8 visible),
  rename prefills buffer (live test typed onto the prefix — worth an
  inline select-all).
- .xlib assets/components not yet auto-registered into consumer
  documents' stores on link.

## Session 40 (v0.35 era — crates 0.27.0 / x-components 0.33.0) — LIBRARY INTEGRITY + LIBRARY UI + LIBRARY COMPONENTS

### 1. Library integrity verification (review's pre-beta item)
- `LibraryDependency.snapshot_hash` = fnv1a128 over the snapshot's
  CANONICAL .xlib serialization (`library_hash()` — save-time and
  load-time use the same function, so any mutation shifts it).
- `verify_dependency` -> IntegrityStatus::{Verified, LegacyUnhashed
  (pre-integrity docs accepted w/ flag), Corrupt{expected,actual},
  MissingSnapshot}; `verify_document_libraries` = load-time sweep.
- Persisted in .x. 2 new tests: hand-edited style caught as Corrupt;
  full .x round trip verifies clean, tampered raw .x text (color hex
  swapped) flagged, truncated snapshot never verifies.
- LIVE: edited document.x by hand (#6633ff -> #111111), reopened →
  stderr "library integrity: brand-system: Corrupt { expected: 0198…,
  actual: fa7d… }". Real corruption caught in the real app.

### 2. Library UI — complete workflow, strict engine client
- 4th inspector tab LIBS (tabs renarrowed to fit): per-library card
  matching the review sketch — name, "v1 • LINKED" (+" • INTEGRITY!"
  in red when unverified), STYLES list, VARIABLES count, COMPONENTS
  chips, ASSETS count. LINK .XLIB + CHECK UPD buttons.
- CHECK UPD re-reads dep.source_path; newer version stages
  (idx, newer, diff_library(pinned, newer)) — the app NEVER diffs
  itself. Blue "UPDATE v2 (4 CHANGES) — REVIEW" banner.
- Review overlay: CURRENT v1 / AVAILABLE v2, per-change rows (+ green,
  - red, ~ yellow) with OLD→NEW color swatches for modified paint
  styles, ACCEPT (calls engine accept_update + rehashes the pin) /
  CANCEL (pinned version untouched).
- LIVE: linked v1 (3 styles, 2 vars, Button+Card), bumped .xlib to v2
  (Primary #3366ff→#6633ff, +Danger, radius 12→16, +Input) → "UPDATE
  AVAILABLE … (4 CHANGES)" → overlay showed +COMP INPUT/+DANGER/
  ~PRIMARY with blue→purple swatches/~VAR RADIUS → ACCEPT →
  "ACCEPTED V2: 4 CHANGE(S)" → saved .x: dep v2 + new hash + snapshot
  Primary #6633ff + components [Button, Card, Input].
- Bug found & fixed live: GET STARTED empty-state card painted over
  VARS/LIBS tabs (was only gated on selection, not tab).

### 3. Library components placed as instances (no cloning)
- `place_library_component`: ONE hidden master per (library, component)
  enters the page registry (stable id libmaster-<lib>-<name>); each
  placement is a Node::instance REFERENCING it — same dependency
  semantics as styles, per review. Click ◆BUTTON chip → live instance
  rendered from the library master ("PLACED BUTTON INSTANCE FROM
  BRAND-SYSTEM", 120×40 blue button on canvas).

### Tests: 248 workspace green (was 246: +2 integrity; lifecycle updated for snapshot_hash).

### Honest remainder
- Registry master is a one-time snapshot: accept_update re-resolves
  STYLE bindings but does not yet swap updated component masters into
  documents (needs master-refresh in accept path).
- LIBS tab hit-testing mirrors painter geometry manually (styles/asset
  rows not clickable yet — chips only); no shared layout fn like
  styles_layout — worth the same refactor.
- Corruption warning appears in status+stderr; no blocking dialog, and
  bindings still resolve against the (corrupt) snapshot rather than
  freezing — freeze-on-Corrupt is the stricter follow-up.
- LINK reads a fixed ./library.xlib path (no file picker).

## Session 41 (v0.36 era — crates 0.28.0 / x-components 0.34.0) — BETA CHECKLIST WAVE 1

### Designer Complete Beta (all 8 items)
- **Component update propagation**: accept_update now also runs
  refresh_library_masters — registry masters (libmaster-<lib>-<name>)
  swap to the accepted version's children/size; instances re-render.
  Test: v2 recolors Button bg -> master carries it post-accept.
- **Freeze-on-corrupt**: freeze_unverified strips unverified snapshots
  from the resolution map; bindings keep last-applied values (test:
  corrupt verdict -> resolve count 0, value intact). Wired into startup
  sweep — status shows "… — FROZEN".
- **Shared LIBS layout**: libs_layout() single geometry source; painter
  + click_inspector both consume it (manual mirror deleted).
- **Asset browser**: wheel SCROLLING (row-paged), SORT chips
  (name/size desc/usage desc w/ active highlight), DRAG-TO-CANVAS
  (press tile, release on canvas -> image node at cursor world pos —
  live: "DROPPED IMAGE-1 AT CURSOR"), RENAME polish (empty buffer =
  select-all semantics; empty commit keeps old name).

### Production Reliability (all 7 items; reliability.rs, 5 tests)
- atomic_write: temp + fsync + rename (no partial writes, no temp
  litter — tested); Ctrl+S now rotates backups -> atomic publish ->
  clear autosave ("SAVED V2 (1 PAGES, ATOMIC, 1 BACKUP(S))" live).
- autosave: every 30s while dirty (undo-depth dirty tracking), atomic,
  cleared on clean save (live: "autosave: 7957 bytes" in log).
- crash recovery: open_with_recovery chain exact -> autosave ->
  lenient -> backups; LIVE: staged doc w/ 1 child + autosave w/ 2 ->
  app opened with UNSAVED-WORK present.
- rolling backups .bak1..3 (recovery HISTORY, rotation tested).
- corruption recovery: garbage main file falls back to backup (test).
- legacy integrity upgrade: unhashed deps get hashes from
  first-open-trusted snapshots; LegacyUnhashed -> Verified (test).
- recent files: MRU under ~/.cache/x-native/recent.txt, atomic.

### Performance (2 of 6)
- bench_scale bin: rect farm + MIXED workload (rect/ellipse/text/
  gradient/instance) at 1k/10k/50k/100k. Numbers (release, softpipe):
  rects 1k=3.9ms ✓ / 10k=35ms / 100k=146ms; mixed 1k=17ms / 10k=138ms /
  100k=1.59s — encode dominates mixed (text shaping per Glyphs cmd).
  HONEST: 10k+ misses 16ms; virtualization/caching is REQUIRED, not
  optional, for the 100k claim. Baselines now exist to measure against.
- Frame-time instrumentation: rolling 64-frame times, Ctrl+Shift+F HUD
  (avg/max ms, fps, red/green 16.7ms sparkline) — live-verified.

### Interoperability (1 of 5)
- Import diagnostics: ImportDoc.diagnostics + lower_with_report ->
  ImportReport { nodes_imported, assets_imported, diagnostics };
  sketch importer records skipped layer classes.

### Tests: 255 workspace green (was 248: +2 library, +5 reliability).

### Honest deferrals (next waves)
- Performance: memory profiling, asset/layer virtualization (the real
  fix for the benchmark numbers above), damage-rect scissoring.
- Interop: SVG import round 2 (transforms/gradients in), Figma
  ImportDoc assets, clipboard, import preview UI; diagnostics not yet
  SHOWN in-app after Ctrl+I (report plumbed, UI pending).
- Library resolver abstraction (single resolve entry for
  local/library/asset refs) sketched but not landed this wave.

## Session 42 (v0.37 era — crates 0.29.0 / x-components 0.35.0) — PERFORMANCE WAVE (review's #1 priority)

### 1. ShapedTextCache (the identified target: shaping-per-command)
- x-text/cache.rs: TextLayoutKey = { text, font binding (family+weight+
  style+fallback via binding), size bits, width bits, letter-spacing,
  line-height, color, font_epoch } — everything shaping-relevant, and
  review's exact example enforced: "Hello"/16px vs 24px = 2 entries.
  POSITION IS NOT IN THE KEY: output glyphs are block-local, world
  transform composes in the sink — moving text is a hit BY CONSTRUCTION.
- Arc'd ShapedBlock {glyph runs, positions, fallback decisions baked,
  metrics}; global process cache; FontManager.epoch() flushes on font
  loads; 4096-entry cap.
- Invalidation tests exactly per review: unchanged=HIT(ptr_eq),
  text change=miss, size change=miss, font change=miss, width
  change=miss AND rewrap asserted (narrow height > 1.5x), position
  change=HIT, epoch bump=miss. 6 tests.
- VelloSink Glyphs path iterates the Arc'd block ZERO-CLONE.

### 2. Incremental Render IR (reuse, don't rebuild)
- The app now renders through the (previously unused!) SceneCache:
  build_render_tree each frame (cheap walk) but re-ENCODE only when
  command fingerprints changed. 2 new tests: identical frame -> encode
  SKIPPED (encode_count unchanged); moving 1 of 500 nodes -> damage set
  <= 4 rects, not a world redraw.
- LIVE HUD proof: steady frame shows "IR 0.0 ENCODE 0.0 (CACHED) …
  IR CACHE HIT".

### 3. Phase instrumentation (evidence-driven HUD)
- HUD (Ctrl+Shift+F) now shows per-phase ms: IR / ENCODE (+CACHED
  flag) / CHROME / OTHER + text-cache hit/miss counters + fps +
  16.7ms-threshold sparkline.

### 4. Virtualization
- LAYERS panel: only the visible window of rows is cloned/painted
  ("… N MORE ABOVE/BELOW" markers); rebuild_layer_rows memoized by
  (undo_depth, filter, child count) fingerprint — no full-tree walk on
  unchanged frames.
- ASSET THUMBNAILS: browser decodes ONLY visible tiles on demand
  (store bytes -> GPU cache per id when scrolled into view); far
  assets stay raw bytes.

### 5. Benchmarks vs the review's acceptance criteria (warm = steady state)
| workload | target | BEFORE | AFTER (warm) |
|---|---|---|---|
| mixed 1k | <=16.7ms | 17ms | **1.4ms** ✓ |
| mixed 10k | <=33ms | 138ms | **14.0ms** ✓ (9.9x) |
| mixed 100k | <=100ms | 1.59s | 215ms ✗ (7.4x better, still over) |
| rects 1k/10k | — | 3.9/35ms | 1.3/12.3ms ✓ |
- bench_scale now measures cold vs warm and prints cache stats +
  pass/fail against the criteria. Mixed encode collapse is exactly the
  predicted shaping win (8800 hits / 4 misses at 10k).

### Tests: 263 workspace green (was 255: +6 cache, +2 incremental).

### Honest remainder
- 100k: warm 215ms > 100ms stress budget. Remaining cost is IR-walk
  (90ms — needs dirty-subtree IR reuse, not just encode skip) and
  encode of 120k paths when it DOES re-encode (needs viewport culling
  in the sink). Documented as the next perf step; 100k remains the
  stress benchmark per review, not the UX target.
- Memory profiling still not done (no allocator instrumentation).
- SceneCache damage rects still don't reach GPU scissoring (Vello 0.1
  has no partial-present hook) — cache saves CPU encode, not raster.
- Thumbnail preload is visible-window-only; "near viewport" preload
  ring not implemented.

## Session 43 (v0.38 era — crates 0.30.0 / x-components 0.36.0) — PERF WAVE 2 + IMPORT UX + GOLDEN CI

### 1. Dirty-subtree IR reuse (FrameCache, 5 tests)
- Replaces SceneCache in the app. Fast subtree-hash walk (geometry,
  paints, effects, bindings, overrides, kind payloads, variables) —
  NO lowering to detect change. Unchanged doc hash -> cached Scene
  (zero lower, zero encode). Changed -> children bucketed 512/segment;
  only dirty buckets re-lower + re-ENCODE (per-bucket cached Scenes,
  composition = ~n/512 Scene::appends). Registry hash keys instance
  buckets (component edit re-lowers consumers — tested); top-level
  masks / root blend fall back to full path (order-dependent clipping —
  tested). Perf bugs found while building: O(n^2) root-clone-per-child
  (priming hung 100k), then per-child Scene::append overhead (100k
  compose > encode) -> bucketing fixed both.
- INTERACTION benchmark (drag 1 node/frame, warm cache):
  1k=1.3ms ✓, 10k=14.2ms wc 24.5 ✓ (was 28.3), 100k=237ms ✗
  (hash 37ms + n/512 appends dominate; honest gap).

### 2. Memory profiling
- Counting global allocator in bench_scale: live/peak MB per tier +
  text-cache resident bytes + eviction count. Data: peak 642-670MB at
  100k (segment scenes + doc clones — now measurable, next target).

### 3. Cache budgets/eviction
- ShapedTextCache: 64MB byte budget (approx path-element accounting) +
  4096-entry cap; either bound -> flush + eviction counter; memory()
  API surfaced in HUD ("CACHE 0.0MB (0 EVICT)").

### 4. Command latency profiling
- Every ctrl-command and canvas click timed; HUD line "LAST CMD
  ctrl+d 0.42MS" (red >16.7ms). Live-verified.

### 5. Import Report UI + 6. Import Preview
- Ctrl+I now STAGES: parse -> lower_with_report -> preview overlay
  with real IR page thumbnails (thumbnail_scene), node/asset counts,
  FIDELITY WARNINGS list (sketch skip diagnostics), ACCEPT/CANCEL.
  Nothing lands before accept; cancel = zero mutation.
- LIVE: sketch with 2 unsupported layer classes -> overlay shows
  thumbnail + "SKETCH: SKIPPED UNSUPPORTED LAYER CLASS 'SLICE' /
  'ARTBOARDSHADOW'" -> accept -> "2 DIAGNOSTIC(S) LOGGED".
- import_sketch_with_report API; figma/svg/png get count-based reports.

### 7. Golden-project CI
- golden_project.rs: one document exercising auto-layout+vars,
  components+overrides, gradients+effects, masks+images, styles+
  bindings, library dep+snapshot+hash, curve-preserving boolean.
  Gates: byte-stable .x, PINNED render-IR shape (13 cmds,
  kind-hash 0xcc7a…, drift = loud failure demanding deliberate
  re-pin), byte-exact undo chain. ci.sh = suite + golden + benches.

### Tests: 271 workspace green (was 263: +5 FrameCache, +3 golden).

### Honest deferrals (unchanged from the list: 8-12)
- Viewport culling/spatial index for render (would fix 100k drag).
- SVG import round 2, clipboard, Figma ImportDoc + figma-specific
  diagnostics (report plumbing exists; figma reports are count-only).
- 100k drag at 237ms: next lever is culling (only ~1-5% of a 100k doc
  is on screen) — bucket appends + hash walk both shrink with it.

## Session 44 (v0.39 era — crates 0.31.0 / x-components 0.37.0) — PERF WAVE 3: CULLING + MEMORY BREAKDOWN + EVICTION

Review items 1+2 status first: dirty-subtree IR reuse LANDED in v0.38
(FrameCache: unchanged doc = zero lower/encode; changed = re-lower only
dirty 512-buckets) — re-verified via tests + benchmarks below.

### 1. Viewport culling (the new work; 4 tests)
- `subtree_bounds()`: conservative world AABB per top child (rotation-
  expanded, blur/shadow-inflated — the "cannot affect the viewport"
  contract, tested: a shadow bleeding INTO the viewport is NOT culled).
- `FrameCache::render_viewport(…, Some(world_rect))`: invisible children
  skipped from lowering AND encoding; visibility mask baked into doc +
  bucket hashes (static viewport = full hit; pan re-encodes only
  visibility-changed buckets — tested). Bounds memoized by subtree hash
  (drag recomputes ONE child's bounds).
- App passes canvas→world viewport (+25% margin, camera-inverse);
  HUD shows CULLED n (green when active). LIVE: panned demo content
  off-screen → "CULLED 3".
- Numbers (app-like 1500×1000 viewport):
  mixed 10k drag: 10.2ms → **6.9ms** (9252/10004 culled)
  mixed 100k drag: 187ms → **110ms** (99252 culled; hash walk 36ms is
  now ~the whole cost — honest floor below).

### 2. Memory profiling — per-subsystem breakdown (review's exact list)
- Document::memory_breakdown(): pages/styles/variables/assets/library-
  snapshots bytes. Assets::memory_bytes() = decoded RGBA (GPU-side).
  FrameCache::memory_bytes() = segment-scene estimate.
  Editor::history_bytes() = undo/redo (ReplaceNode snapshots dominate).
  ShapedTextCache::memory() (v0.38) completes the list.
- All seven surfaced in the HUD: "MEM DOC … ASSETS … LIBS … | TXT …
  SEG … GPU … UNDO … MB" — live-verified.

### 3. Cache eviction (text ✅ v0.38; NEW: thumbnails/GPU)
- Assets::evict_except(keep): drops decoded asset:// images not in the
  keep set; content-addressed store retains raw bytes so eviction is
  always safe (re-decode on demand).
- Wired: closing the asset browser (Shift+A) evicts thumbnail decodes
  not referenced by any page — status reports "N MB thumbnails evicted".

### Tests: 275 workspace green (was 271: +4 culling).
### bench_results_v039.txt committed.

### Honest remainder
- 100k drag floor is the O(n) subtree-hash walk (~36ms) + doc clone in
  bench prime; going below needs editor-emitted dirty node IDs (skip
  hashing entirely) — a Document-model change, deferred deliberately.
- FrameCache::memory_bytes is an estimate (vello 0.1 Scene has no size
  API); GPU texture memory ≈ decoded RGBA (Images are CPU blobs handed
  to vello per frame — true VRAM accounting needs wgpu allocator hooks).
- Segment cache has no byte budget yet (text+GPU do); disk filled during
  this session (24GB target/) — cleaned incremental artifacts, ci.sh
  unaffected.

## Session 45 (v0.40 era — crates 0.32.0 / x-components 0.38.0) — UI OVERHAUL (user's mockup)

User supplied a full product mockup (dark chrome, #3366FF accent, two-row
header, sectioned inspector, bottom page thumbnails). Restyled the app:

### 1. REAL TYPOGRAPHY IN ALL CHROME (the big one)
- `label()` now renders SHAPED text through node_text_outlines +
  ShapedTextCache (thread_local FontManager; FontManager is not Sync
  due to its RefCell outline cache — found at compile time). The blocky
  vector font is retired to fallback-only. `ui_measure()` gives real
  text metrics for layout. Every panel instantly looks like the mockup.
- 151 label() call sites upgraded by changing ONE function.

### 2. Two-row header
- Row 1: X logo mark (accent), product name, document tab ("Brand
  Dashboard") with accent underline + dirty dot, "+" new-tab hint.
- Row 2: File/Edit/View/Object/Help menu labels, CENTERED tool row
  (moved from the floating bottom bar — bottom_bar_rect() now maps to
  the header, so all tool click/tooltip code worked unchanged), zoom
  pill cluster, accent ▶ Present pill (click = enter_present).

### 3. Bottom page-thumbnail strip + status bar
- THUMBS_H strip: live page thumbnails through the REAL render IR
  (active page renders from the live editor tree — drawing a rect
  updates their thumbnail immediately, verified), accent border +
  name pill on the active page, "+ New Page" cell (click = create).
- Status bar: green ready dot + status/edit-buffer line, right side
  X/Y/W/H of the selection + zoom % (mockup's footer).

### 4. Left panel + inspector restyle
- Icon-tab row (Layers/Assets/Components/Library — Layers active),
  mockup-style rounded search field at top (drives the existing layer
  filter; verified live: "dot" filters). Pages list compacted.
- Inspector: filled dark field boxes (X/Y/W/H) with dim labels —
  same rects as before so click-to-edit worked unchanged (verified:
  typed X=200, saved, document.x shows x=200); section headers
  moved to mockup case (Fill/Constraints/Styles/…).
- New theme: near-black panels (#17181c/#1e2025), #3366FF accent,
  canvas #26282e.

### Tests: 275 workspace green (unchanged — pure UI wave, geometry
shared with handlers so no behavior drift).

### Honest remainder
- Left icon tabs are visual: Assets/Components/Library click-through
  to their panels not wired yet (Shift+A browser + LIBS tab still the
  real UIs); menu labels are visual (actions live in shortcuts/help).
- Inspector still lacks mockup's collapsible sections/eye toggles per
  fill row; Export section not present.
- Thumbnail strip renders every page every frame (fine at <10 pages;
  needs the FrameCache treatment for many-page docs).
- Multi-tab documents are visual only (single doc per window).

## Session 46 (v0.41 era — crates 0.33.0 / x-components 0.39.0) — REAL MENUS + REAL LEFT TABS + EXPORT SECTION

The two biggest "visual only" gaps from session 45's honest remainder are
now real, live-verified features.

### 1. Real dropdown menus (File / Edit / View / Object / Help)
- MENUS table in theme.rs: 31 items with shortcut hints, each carrying an
  action tag ("file.save", "obj.union", ...). menu_title_rects() +
  menu_layout() give ONE geometry for painter and click handler (the
  shared-layout pattern that has prevented drift 3 times now).
- run_menu_tag() dispatches into the SAME methods the keyboard shortcuts
  use — Ctrl+S/O/I/E bodies were extracted out of run.rs into App::
  save_document/open_document/start_import/export_{svg,png,pdf}_now, so
  the menu can never diverge from the shortcut behavior (run.rs match
  arms are now one-liners calling the same methods).
- Behavior: click title opens, click item runs + closes, click another
  title switches, Esc or outside-click closes, hover = accent row.
- LIVE: File menu screenshot; File>Save wrote document.x (v2, atomic);
  Object>Union on card+dot produced bool-0 VECTOR (screenshot);
  Edit>Undo restored both; View>Zoom to Fit went 60%→47%;
  File>Import... opened the import-preview overlay for import.png.

### 2. Left icon tabs are REAL panels (Layers / Assets / Components / Library)
- App::left_tab + left_tab_rects()/left_panel_layout() (shared geometry).
- ASSETS tab: 2-column grid of document assets with real decoded
  thumbnails; click selects + arms drag-to-canvas (mouse_up extended:
  drop from the left tab creates the image node at the cursor — same
  code path as the Shift+A browser drop). LIVE: dragged the imported
  gradient PNG onto the canvas → image-1 96x64 bound to asset://3edb....
- COMPONENTS tab: document components (click = stamp, existing stamping
  flow) + linked-library components (click = place_library_component).
  LIVE: stamped Component1-2 INST; placed LibButton from Brand Kit v1
  → lib-inst-2 + libmaster-brand-kit-LibButton in the saved doc.
- LIBRARY tab: linked-library summary cards (name, version, style/comp
  counts, integrity flag) + OPEN LIBRARY MANAGER jump to the LIBS
  inspector tab. LIVE: linked library.xlib (Brand Kit v1) and the tab
  showed "Brand Kit v1 / 2 style(s), 1 comp(s)".
- Bug found LIVE: sorted_assets() returns (name, id) and the first
  Assets-tab build destructured (id, _) — tile keyed by display name
  showed NO PREVIEW. Fixed to (_name, id); re-verified with thumbnail.

### 3. Export section in the inspector (mockup's bottom-right section)
- export_layout(): PNG / SVG / PDF buttons above the thumbnails strip,
  hover accent; dispatches through run_menu_tag into the real exporters.
- LIVE: all three clicked; export.png (1600x1000 RGBA, real GPU render),
  export.svg (valid, gradient defs), export.pdf (v1.4, pdftoppm renders).

### Tests: 275 workspace green. Versions bumped: crates 0.33.0,
### x-components 0.39.0.

### Honest remainder
- Menus lack disabled-state rendering (e.g. Undo with empty history is
  clickable and no-ops via editor guard) and keyboard navigation
  (arrows/Enter); x-ui Menu widget is still only the right-click menu.
- Assets tab has no scroll (grid clips at panel bottom; fine <14 assets)
  and no search field (Shift+A browser remains the full manager).
- Components tab shows first 10 per library, no thumbnails on rows.
- Inspector sections still not collapsible; per-fill-row eye toggles
  absent; Export section is page-level only (no per-node export).
- Multi-document tabs still visual; 100k stress unchanged (196ms warm,
  floor = O(n) hash walk, editor dirty-IDs still the deferred lever).

## Session 47 (v0.41.1) — MOCKUP FIDELITY PASS ("design is not matched")

User compared the running app against the uploaded mockup and rejected
the delta. Side-by-side diff produced this fix list; every item is now
implemented and live-verified:

### Header (mockup row 1 + 2)
- Menu bar now matches: File Edit View Object ARRANGE PLUGINS Help
  (Arrange = real align actions via editor::align; Plugins placeholder).
- Right cluster rebuilt at FULL header width (was stopping at the
  inspector edge): zoom pill (single rounded field, -/+ halves),
  ▷ Prototype ghost button (click → Prototype tab), ▶ Present accent
  pill, avatar circle at the far right. ONE `header_rects()` feeds both
  painter and click handler.
- Tab strip: caret next to X-NATIVE, window controls (— ▢ ✕) top-right.

### Inspector — mockup's labeled sections (the big delta)
- New IY_* y-map in theme.rs shared by painter AND click_inspector.
- Position: header + X/Y/W/H filled boxes (2-col, mockup style) +
  ROT/OP boxes with opacity -/+ inside the OP field.
- Fill: section header + swatch/hex/opacity row with per-row EYE
  toggle (hides the node — live-verified: canvas, layer row and page
  thumbnail all update), palette as ONE compact row, GR chip.
- Stroke: swatch + hex + width with -/+ steppers + INSIDE tag
  (mockup's E5E7EB/1/Inside row shape). Swatch click cycles palette.
- Effects: header + "+" (adds Drop Shadow) + one row per effect with
  circle glyph + per-row eye (removes that effect) — mockup's
  Drop Shadow/Blur rows. Live: added 2nd shadow, removed it by eye.
- Inspector tabs restyled to text + accent underline (Design Proto
  Vars Libs) instead of colored blocks.
- Export section pinned above the thumbnails strip (separator + PNG/
  SVG/PDF buttons) — no longer collides with the styles browser.

### Left panel
- Icon tabs now ICON above LABEL in 4 equal columns (LTAB_H=54) —
  layers stack / image / component diamonds / library book, accent
  underline on the active tab (mockup's left rail look).
- Search field / Pages / Layers shifted below via shared LSEARCH/LPAGES
  constants (click handlers use the same ones).

### Misc
- Rulers ON by default (mockup shows them).
- Old inspector anchors all re-pointed to the IY map: component/image/
  auto-layout at IY_SEC, constraints IY_CONSTRAINTS, styles IY_STYLES,
  font browser IY_FONT.

### Live verification (fresh doc, xdotool)
- W field: click → BackSpace×3 → "300" → Enter → W=300 (found UX wart:
  buffer pre-fills so bare typing appends — logged below).
- Stroke +: 0→2. Effects +: second Drop Shadow row; row eye removed it.
- GR chip → gradient fill; palette row recolor; fill-row eye hid the
  card (s47_eye_toggle.png).
- Arrange ▸ Align Middle centered dot to card (s47_arrange_menu_align).
- Zoom pill -/+ at the new right-cluster position works.

### Tests: 275 workspace green.

### Honest remainder
- Field edit buffer pre-fills the old value (select-all-on-focus
  semantics missing — typing appends until you Backspace).
- Inspector sections still not collapsible (mockup has −/+ carets);
  no Auto Layout "+" chip row for non-frames; stroke INSIDE is a
  static tag (no alignment options).
- Hex values are read-only text (no hex input field).
- Ruler numbers overlap the canvas edge slightly at some zooms.
- Avatar/window controls are visual only (no window integration).

## Session 47b — "design is not matched" round 2: THE DOCUMENT ITSELF

Diagnosis: rounds 1-2 restyled the chrome, but the biggest visual delta
was the CONTENT — the mockup shows a white "Desktop - 1440" landing page
with pages Dashboard/Analytics/Users/Settings/Mobile; our canvas still
had 5 loose demo shapes. Fixed:

### 1. demo_document() rebuilt to recreate the mockup
- Page "Dashboard" contains frame "Desktop - 1440" (white, 1440x1024)
  with the mockup's structure as plain editable nodes:
  - Header group: logo mark + "Brand", nav (Product/Solutions/
    Resources/Pricing), accent "Get Started" button.
  - Hero Section group: "New" badge pill, "Create stunning designs
    together" (accent word), subcopy, "Get Started Free" CTA w/ soft
    brand shadow, "Watch Demo", soft indigo blobs, color-chips card
    (Primary #3366FF / Secondary / Success), chart card (+12.5%, 3.2k,
    polyline vector), avatar strip pill (+24).
  - Features group: 3 bordered cards (Design System / Components /
    Collaboration) with icon tiles and 2-line copy.
- Pages Analytics/Users/Settings/Mobile: dark dashboard mock layouts
  (sidebar/topbar/cards/hero + accent bar) so the bottom thumbnail
  strip reads exactly like the mockup's page rail.
- Layers tree now mirrors the mockup's (Header / Hero Section /
  Features with named children); row name limit 12→18 chars
  ("Desktop - 1440" no longer truncates).

### 2. Frame name labels on canvas
- Top-level frames get the Figma-style floating "◇ Frame Name" label
  above their top-left corner (accent when selected) — mockup's
  "◇ Desktop - 1440".

### 3. Minimap off by default
- The mockup has no minimap; it's now a View ▸ Minimap toggle
  (painter + click handler both gated).

### Live verification
- Fresh doc: Dashboard page shows the full landing page; thumbnail
  strip = Dashboard/Analytics/Users/Settings/Mobile/+New Page.
- Page switch to Analytics (dark layout) and back — live thumbnails.
- Esc-to-parent selected Hero Section GROUP: inspector shows
  Position 0/0 1440x480, empty fill hex 00000000, effects NONE.
- 275 workspace tests green.

### Honest remainder
- Demo text uses the single shaping pipeline's default font at fixed
  sizes; mockup's exact typographic weights (bold hero) need per-node
  weight binding (font browser can apply Google weights manually).
- Feature icon tiles are plain rounded rects (no glyph art inside).
- Chart card has no area fill under the polyline; avatar overlaps are
  flat circles (no ring strokes).
- Frame label is display-only (double-click-to-rename not wired).

## Session 48 (v0.41.3) — "clone the design for interface" round 3

(Environment reset mid-session — toolchain reinstalled, full rebuild.)

Remaining deltas vs the mockup, all closed this round:

### Layers panel = mockup's tree
- Kind text-labels replaced by TYPE GLYPHS: T for text, rounded square
  for rects, square for frames/groups, circle for ellipses, picture
  glyph for images, purple diamond for components/instances, ~ for
  vectors; expand carets on containers. Rows read "▾ ▢ Header",
  "T Nav Product" exactly like the mockup.
- Search field got the magnifier glyph; Pages header a "+".

### Inspector = mockup's exact section list + tabs
- Tab strip is now Design | Prototype | INSPECT (mockup's three).
  Vars reachable via View ▸ Variables; Libs via left Library rail
  (small indicator shows when either is active).
- Design tab order now MATCHES the mockup: Position (name rides the
  header, ∠ rotation box + kind dropdown box) → Auto Layout (header
  always present; +/FRAMES ONLY for non-frames) → Appearance (opacity
  % w/ -/+ AND corner radius w/ -/+ — new live control) → Fill →
  Stroke → Effects → Styles (compact chips, no group headers) →
  Export. No overlaps at 800px height.
- Text nodes: Font section (search + 5 rows + weights) takes the
  styles/constraints slot, like the mockup's type panel.
- Constraints moved to the INSPECT tab, which now shows a read-only
  spec block (X/Y/W/H/ROT, FILL hex, STROKE, OPACITY, EFFECTS) +
  the constraint pin grid — a real inspect/handoff panel.

### Live verification
- Opacity stepper: 100→90% (Fill row % followed); undo restored.
- NEW radius stepper: CTA Header 9→13, button visibly rounder.
- Inspect tab: spec block for CTA Header (X 1240 Y 24 W 130 H 38,
  FILL #3366ff), h-pin L→R applied via pin grid.
- Text selection shows Font section clear of Export (5/80 scroll).
- 275 workspace tests green after all changes.

### Honest remainder
- Inspect tab spec is display-only (no copy-to-clipboard — no
  clipboard integration yet, deferred with the interop items).
- Layer tree carets are decorative (tree always fully expanded;
  collapse state not in the walk).
- Appearance radius stepper targets Rect nodes only (frames have no
  radius field in the model).
- Vars/Libs tabs are now "hidden" behind menu/left-rail entry points —
  power-user paths, matching the mockup's minimal 3-tab strip.

## Session 49 (v0.42-beta — crates 0.34.0 / x-components 0.40.0) — BETA POLISH PASS

User: "make polished interface for this, we are close to beta". This
session removed the known UX warts and added the interaction polish a
beta tester touches first. (Cargo cache corrupted mid-session again —
rm -rf ~/.cargo/registry/src, rebuilt.)

### 1. Field editing done right (killed the session-47 wart)
- Numeric fields now use SELECT-ALL semantics: focusing shows the old
  value highlighted in an accent wash; the first keystroke REPLACES it;
  Enter with nothing typed keeps the old value; Esc cancels.
- TAB commits and hops to the next field (X→Y→W→H→X) with the same
  select-all state — rapid keyboard-only geometry editing.
- LIVE: focused W (130 shown selected), typed 200, Tab → W committed
  (canvas + "200 X 38" badge updated), H focused with 38 selected.

### 2. Disabled menu states (real-app behavior)
- menu_item_enabled(): Undo grays with empty history; Duplicate/Delete/
  Front/Back/Mask need a selection; Group/booleans/Arrange need 2+;
  Plugins placeholder always disabled. Disabled items render dim, take
  no hover highlight, and clicking them keeps the menu OPEN with an
  explanatory status line.
- LIVE: Object menu fully grayed with nothing selected; same menu
  fully lit with 2 layers selected; both screenshots.

### 3. Hover states everywhere
- Tool slots in the header, page rows in the left panel, page cells in
  the thumbnail strip (accent ring), numeric field boxes, Present pill
  (lighter accent), menu rows (already) — chrome now responds to the
  pointer like a finished product.

### 4. Scroll affordance
- Layers panel got a proportional scrollbar (thumb size = visible
  fraction, position tracks layers_scroll) — visible whenever the tree
  overflows; pairs with the existing wheel scrolling.

### Tests: 275 green. Versions: crates 0.34.0, x-components 0.40.0.

### Honest remainder (the beta-testing gap list)
- No true text cursor/caret in fields (append/backspace only, no
  mid-string editing or arrow-key caret movement).
- Scrollbar is display-only (not draggable; wheel scrolls).
- Hover states repaint on the NEXT frame after cursor move (event-
  driven redraw; imperceptible in practice).
- Inspector still not collapsible; no per-fill-row multi-fills.
- Beta-blocking candidates for next session: window title dirty flag,
  About/version in Help menu, crash-recovery banner polish, first-run
  onboarding hints.

## Session 50 (v0.42.1) — RIGHT-SIDE INSPECTOR CLEANUP ("right side has so many issues")

(Snapshot rollback recovered: sessions 48-49 file changes survived,
commits/tags were re-created as 1e7e453 / v0.42-beta. Toolchain
reinstalled again.)

User called out the right panel specifically. Fixed the concrete issues
visible in the last screenshots:

### Real glyphs replace ASCII placeholders (helpers.rs additions)
- draw_eye(): almond+iris eye, slashed when off — used by the Fill row
  visibility toggle and each Effects row (was "O"/"-" letters).
- draw_stepper(): bordered -/+ buttons with centered stroke glyphs and
  hover wash — used by Appearance opacity, corner radius, stroke width,
  Effects add, Auto Layout hint (was bare "-"/"+" text floating).
- draw_align_icon(): real bar+object alignment icons for the 6-button
  row (was "|<", "><", ">|", "T", "M", "B" ASCII).
- draw_section_sep(): hairline separators above Appearance / Auto
  Layout / Fill / Stroke / Effects / Styles / Font — the sections now
  read as sections instead of one run-on column.

### Collision + tone fixes
- Stroke row rebuilt: width value right-aligned, steppers at fixed
  slots, "Inside" is a small pill with a dropdown caret (was INSIDE
  text colliding with the + stepper).
- "NONE — + ADDS DROP SHADOW" → "None — click + to add a drop shadow";
  "FRAMES ONLY" → "Available on frames"; "GET STARTED"/"N LAYERS
  SELECTED" → sentence case. The shouting is gone.
- Effects + moved into a proper stepper button aligned with the header.

### Click zones re-synced to the new geometry (stroke -/+ shifted left
### for the Inside pill; opacity/radius/effects zones match steppers).

### Live verification (fresh doc, CTA Header selected)
- Stroke + clicked twice: 0→2 (value updated, canvas stroke visible).
- Effects +: "Drop Shadow" row appeared with real eye; eye click
  removed it. Fill-row eye: node hidden (slashed red eye, canvas +
  thumbnail updated), clicked again → shown.
- 275 workspace tests green.

### Honest remainder
- Inside pill is visual (no stroke-alignment model field yet).
- Section separators are static (sections still not collapsible).
- Align icons: no distribute variants (6 align ops only).

## Session 51 (v0.43-beta) — FIGMA-PARITY WAVE 1: clipboard, pages, dashboard lifecycle

User pasted a full Figma-parity requirements doc (19 sections + 9
acceptance tests). Audit-first: most editor core already existed
(multi-select/marquee/drag, undo/redo, text double-click editing,
radius, path tool, SVG/PNG/PDF export, import preview). This wave built
what was genuinely missing and re-verified the rest.

### Built this wave
1. CLIPBOARD (req 6): Editor::cut() + clipboard_len() in x-editor;
   Ctrl+C/X/V bound; Edit menu Cut/Copy/Paste with enablement
   (paste grays until clipboard has content); context menu gained
   CUT/PASTE (dispatch re-synced). Copy/paste preserves all node
   properties (same serialization path as duplicate) and stays
   editable — never flattened.
2. PAGES (req 9): double-click page (strip cell or left row) -> inline
   rename with Enter/Esc (new Focus::PageRename routed through the
   shared focus system); File menu Rename/Duplicate/Delete Page;
   duplicate re-suffixes all node ids; delete guards the last page;
   names persist through save/reload (test).
3. COLLAPSIBLE PAGES PANEL (req 10): chevron toggles the strip to a
   22px slim bar ("Pages (5) — Landing"); canvas_rect expands; state
   persists in .xprefs; View ▸ Pages Panel menu item.
4. DASHBOARD -> FILE -> EDITOR (req 19): new Screen::Dashboard with
   file cards (real IR thumbnail, name, page count, modified age),
   + New File (creates files/untitled-N.x), click card -> open_file()
   loads that document (per-file doc_path replaces the DOC_PATH
   constant in save/open/autosave). X logo = Home: auto-saves,
   refreshes cards. Brand Dashboard is seeded as document.x on first
   run — a real persistent file, not UI state.

### Acceptance tests verified LIVE (screenshots)
- A multi-select: marquee over 2 feature cards -> 11 layers selected,
  dragged together, Shift+Arrow moved together.
- D copy/paste: CTA Header -> Ctrl+C/V -> "CTA Header-copy" selected,
  editable, offset. E cut/paste: Ctrl+X removed it, Ctrl+V restored.
- F pages: double-click "Dashboard" cell -> typed "Landing" -> Enter;
  saved doc shows pages [Landing, Analytics, Users, Settings, Mobile].
- I viewport: collapsed strip -> canvas expanded -> state persisted.
- Dashboard lifecycle: Home -> open card -> edit -> X logo -> card
  thumbnail shows the edits, "just now"; reopen -> edits present
  (document.x: CTA Header-copy-copy persisted).

### Already-working items re-verified (no changes needed)
- B text: double-click -> inline edit -> Enter commits (since P0 era).
- C radius: Appearance stepper immediate + persists (session 48).
- G path tool: pen/anchors/handles/undo live-verified in v0.24-0.32.
- H export/import: SVG/PNG/PDF exporters + SVG import preview overlay.

### Tests: 278 (275 + clipboard_pages.rs: copy/paste multiselect
### properties+editability, cut round-trip w/ undo, page-rename
### save/reload persistence).

### Honest remainder (Figma-parity gaps, priority order)
- Clipboard is internal-only (no OS clipboard / cross-app SVG paste).
- Text editing: no caret/mid-string editing/selection ranges; no
  per-node font size/line-height/letter-spacing fields in inspector
  (font family+weight only). No auto-width/auto-height text modes.
- Paste targets the page root, not the hovered frame; no
  paste-over-selection / paste-here.
- Dashboard: no rename/duplicate/delete/search on cards yet (cards
  open only); no file context menu.
- No independent corner radii; no distribute-spacing; no sections;
  page reorder not implemented; .fig import NOT supported (honest:
  proprietary format — native .x format documented instead).
- Right-click rename/context menu on pages/layers rows absent.

## Session 52 (v0.44-beta) — FIGMA-PARITY WAVE 2: the 5-item remainder list

User asked for exactly the 5 gaps from wave 1's honest remainder.
(Snapshot rollback again — sessions 42-43 re-committed as e418966.)

### 1. OS clipboard bridge (was: internal-only)
- os_clipboard_set/get in main.rs (xclip shell-out, X11; graceful None
  when unavailable — honest: desktop-Linux path, Wayland needs wl-copy).
- Ctrl+C/X/V INSIDE text editing now hit the OS clipboard: copy/cut
  push the buffer out; paste inserts external text at the caret.
- Edit ▸ Copy as SVG: selection is wrapped in a bounds-sized frame,
  exported through export_svg_full, placed on the OS clipboard —
  cross-app SVG interop. VERIFIED: xclip -o returned real 130x38 SVG
  markup of the CTA Header.

### 2. Text caret + typography fields (was: append-only, no fields)
- Focus::TextNode gained `caret`; chars INSERT at the caret; Backspace/
  Delete are caret-aware (char-boundary safe); Left/Right/Home/End move
  it; a caret line renders in-canvas at the measured x-position.
  VERIFIED: typed TEXT, Home→Right×2... final saved doc "TEXTHELLO"
  (typed + OS-paste at caret).
- ONE-pipeline typography: node_text_outlines_styled(ls, lh) +
  TextLayoutKey::new_styled (the ls/lh key bits existed since v0.37 —
  now used). Glyphs IR command carries letter_spacing/line_height from
  node bindings; canvas sink + cache consume them; SVG/PDF inherit via
  the same shaping entry.
- Inspector Font section: Size / Sp / Lh stepper boxes (click upper
  half = +, lower = −). VERIFIED: Sp 0→2.0 visibly widened "TEXYXT".
- New regression test: spacing widens layout, line-height raises block.

### 3. Paste-into-frame (was: page root only)
- clipboard_paste picks the top-level frame under the CURSOR as the
  paste parent (Figma behavior); falls back to page root; status says
  "pasted N object(s) into Frame".

### 4. Dashboard card actions (was: open-only)
- Right-click card → OPEN/RENAME/DUPLICATE/DELETE context menu
  (ctx_menu widget reused; painted on the dashboard scene too).
- VERIFIED live: DUPLICATE created files/copy-1.x ("Brand Dashboard
  copy" card), RENAME → "Marketing Site" (metadata write-through,
  card + editor doc-tab both show it), DELETE guards document.x.
- Search box (Focus::DashSearch) filters cards live — VERIFIED "mark"
  → only Marketing Site. Double-click card name = rename.
- Editor doc tab now shows the real file name (was hardcoded).

### 5. Distribute-spacing (+ the honest .fig stance)
- Arrange ▸ Distribute Horizontal/Vertical: sorts by axis, equalizes
  gaps between 3+ selected layers (enablement at 3+).
- .fig: still NOT supported, still not faked. Native .x remains the
  documented format.

### Tests: 279 green (278 + typography geometry test).

### Bugs found live this session
- dash_layout edit originally targeted app.rs but the fn lives in
  chrome.rs — silent no-op until the search box didn't paint; fixed
  in the right file (and the lesson: grep before patch).
- Editor doc-tab was hardcoded "Brand Dashboard" — renamed files
  looked wrong in the editor; now reads dash_files metadata.

### Honest remainder
- Independent corner radii still absent (model has one radius field).
- Page reorder still absent.
- Text selection RANGES still absent (caret only, no shift-select);
  Ctrl+A in text = caret-to-end placeholder.
- OS clipboard object-paste (SVG in → nodes) not wired: SVG import
  exists via Ctrl+I but not from the clipboard.
- Wayland clipboard needs wl-copy/wl-paste variant.

## Session 53 (v0.45-beta) — FIGMA-PARITY WAVE 3: ranges, corners, reorder, SVG-in, Wayland

User asked for exactly wave 2's honest-remainder list. (Snapshot
rollback again — waves recommitted as 8a2feef first.)

### 1. Text selection RANGES (was: caret only)
- Focus::TextNode gained sel_anchor: Option<usize>. Shift+Left/Right/
  Home/End extend the range from the anchor; plain arrows clear it.
- Ctrl+A = REAL select-all (anchor 0, caret end). Typing/Space/paste
  REPLACE the range; Backspace/Delete delete it. Ctrl+C/X copy/cut
  the RANGE (whole buffer if none) to the OS clipboard.
- Canvas paints the range as an accent wash between the two measured
  x-positions + the caret line.
- LIVE: Ctrl+A over "TEXT" -> typed "Replaced" (replaced wholesale);
  Shift+Left×3 highlighted the tail; Backspace removed it.

### 2. Independent corner radii (was: single radius)
- Model support existed (corner_radii: Option<[f64;4]> + IR + ser/de
  since the mask-matrix era) — this wave added the missing UI + edit
  path: adjust_corner(Some(k), delta) promotes uniform radius to
  per-corner on first edit; uniform stepper clears overrides back.
- Inspector Appearance gained TL/TR/BR/BL mini-boxes (top half +2,
  bottom −2) with a "mixed" badge when corners diverge.
- LIVE: CTA Header BR 9→21 (canvas visibly asymmetric); saved doc
  shows "corners":[9,9,21,9]. New regression test: render + persist.

### 3. Page reorder (was: fixed order)
- reorder_page(dir): swaps with neighbor, follows the page, marks
  dirty. File ▸ Move Page Left/Right with edge enablement.
- LIVE: Dashboard moved to slot 2 — Pages list + thumbnail rail both
  reordered; saved doc pages array starts with Analytics.

### 4. Clipboard SVG-in (was: out only)
- Edit ▸ Paste SVG from Clipboard: os_clipboard_get -> import_svg ->
  id-resuffixed editable node tree inserted at the cursor's world
  point. Refuses politely when the clipboard has no <svg>.
- LIVE: external file xclip'd from the shell -> pasted as 8 editable
  nodes (green rounded rect + red circle visible on canvas, in the
  layers tree, and persisted). New regression test: export_svg_full ->
  import_svg round-trip stays an editable tree.

### 5. Wayland clipboard (was: xclip only)
- os_clipboard_set/get now try wl-copy/wl-paste FIRST, fall back to
  xclip; graceful None when neither exists. (Honest: verified on X11;
  Wayland path is best-effort until we have a Wayland CI target.)

### Tests: 281 green (279 + corner radii persistence + SVG round-trip).
### One test assertion fixed during the session (transparent frame
### emits no fill command — assert the rect's FillPath instead).

### Honest remainder
- Range selection is keyboard-driven; mouse drag-to-select inside a
  text block not wired (needs per-glyph hit mapping from the shaper).
- Corner boxes are steppers, not typed fields; no on-canvas corner
  handles (Figma's draggable dots).
- Page reorder is menu-driven; thumbnail drag-reorder not wired.
- Pasted SVG lands as one group at cursor; no paste-in-place variant.
- Wayland path untested in this environment (no compositor).

## Session 54 (Mac parity wave 1) — native files, recovery safety, X identity

### Mac-native daily workflow
- Added native open, Save As, import and SVG/PNG/PDF export panels through
  `rfd`. Designers now choose real files and destinations instead of placing
  specially named files in the process working directory.
- Open and ⌘O share the same native picker; ⇧⌘S was added for Save As.
- Import keeps the existing non-destructive preview/accept workflow and now
  accepts the selected Sketch, Figma JSON, SVG or PNG file.
- macOS clipboard now uses `pbcopy` / `pbpaste` before the existing Wayland
  and X11 fallbacks.

### Reliability fix
- Periodic autosave previously called `autosave(DOC_PATH, ...)`, so every
  dashboard document wrote recovery data beside `document.x`. It now uses
  `app.doc_path`, keeping crash recovery attached to the active file.

### Interface direction
- Replaced the borrowed bright-blue interaction language with X Designer's
  graphite + violet system across chrome, retained widgets, selection washes,
  menus and focus states.
- Increased the Layers panel to 252 px and Inspector to 260 px for calmer
  hierarchy and less property-label collision.
- Replaced visible Ctrl/Alt shortcut copy with native ⌘/⌥/⇧ notation.

### Validation boundary
- Source and archive structure were inspected, but this environment has no
  Rust toolchain or macOS display. Compile, live dialogs, Retina/trackpad and
  signed-app validation remain required on the Mac test machine.

## Session 55 — tool parity wave 1 (capability before chrome)

Direction changed: stop platform-specific prioritization. Complete editor
tools and their Figma-reference behaviors first; production UI follows after
the tool contract is stable.

### Creation contract
- One shared `creation_rect` drives both live preview and committed nodes.
- Shift constrains shapes/frames and snaps lines to 45-degree increments.
- Alt creates from the gesture center; Alt+Shift composes correctly.
- Horizontal and vertical lines pass a length guard instead of the old 3x3 guard.

### Shape tool options
- Rectangle default corner radius is editable before drawing.
- Polygon exposes 3–60 sides.
- Star exposes 3–60 points and 5–95% inner radius.
- Right inspector shows tool-specific controls and modifier hints.

### Transform parity
- Added editable, undoable Flip Horizontal / Flip Vertical for every node type.
- Added Arrange menu entries and Shift+H / Shift+V shortcuts.

### Cleanup
- Removed the duplicate Pen match arm and separated its toolbar label from Polygon.

### Next tool waves
1. Multi-fill/stroke/effect stacks and gradient stop editor.
2. Text resize modes, paragraph controls and mouse hit mapping.
3. Auto Layout wrap/min-max/absolute/baseline and canvas reorder.
4. Component property authoring and full instance override controls.
5. Prototype overlays, scroll/fixed behavior, triggers and flow starts.

## Session 56 — visual stack architecture (next wave, no build)

Per direction, no intermediate ZIP/build was produced. This work changes the
document/render architecture instead of adding inspector-only controls.

### Canonical layered visuals
- Added ordered `PaintLayer`, `StrokeLayer`, and `EffectLayer` models with
  visibility, opacity and blend metadata.
- Added explicit `visual_stacks_materialized` migration state. Legacy files
  fall back to their single fill/stroke/effects; materialized files can safely
  represent zero layers without the legacy paint reappearing.
- Layer add/remove/reorder/toggle operations use whole-node replacement and
  remain atomic undo/redo steps.

### Render and cache
- Render IR lowers every visible fill and stroke in order with stable per-layer
  keys, opacity and blend layers.
- Drop-shadow effect layers are emitted before paint layers.
- Frame-cache fingerprints include all layer paint/state data and distinguish
  legacy fallback from an intentionally empty visual stack.
- Viewport bounds use the active effect stack.

### File and export contract
- `.x` serialization persists all three stacks, including empty stacks,
  visibility, opacity, blend and effect state.
- Loader migrates old documents without changing their rendering.
- SVG export now emits ordered fill/stroke layers for rectangles, ellipses,
  vectors and lines. PDF/canvas share the Render IR layer output.

### Inspector behavior
- Fill, stroke and effect headers expose layer count/selection, add, reorder
  up/down and remove.
- Fill/stroke blend modes cycle through Normal, Multiply, Screen, Overlay,
  Darken and Lighten.
- Fill opacity is editable per layer; visibility applies to the selected layer,
  not the entire node.
- Gradient stop selection is retained; palette changes edit the selected stop
  instead of collapsing the gradient to a solid.

### Regression coverage added
- Visual-stack file roundtrip with hidden layers, opacity and blend state.
- Render-IR command coverage for two fills, one stroke and one effect.

### Still open inside visual fidelity
- On-canvas gradient handles and arbitrary stop add/remove/drag.
- Inner-shadow and blur GPU implementation.
- Multi-fill shaped text and full blend-mode preservation in SVG.
## Session 57 — direct gradient editing and visual parity (no build)

- Added on-canvas linear/radial gradient geometry and stop handles.
- Gradient stops can be selected and dragged, added by double-clicking the gradient axis, and removed with Alt-click while preserving a valid two-stop gradient.
- Gradient drags merge into one undoable document transaction.
- Stroke alignment is now an editable per-layer property; GPU strokes consume cap, join, dash, offset, and miter options from the document model.
- SVG text now exports every visible fill layer, including gradients, opacity, and per-fill blend modes.
- Inspector gradient stop selection now supports the complete stop list instead of a fixed two-stop display.

## Session 58 — GPU effects, text paint parity and inspector refinement (no build)

- Added normalized multi-ring Gaussian GPU-vector compositing on the pinned Vello backend.
- Drop shadows now consume blur radius instead of rendering as hard offset fills.
- Inner shadows render above paint, clipped to the source geometry, with offset and Gaussian falloff.
- Layer blur now covers shape fills, strokes, text glyph outlines and images.
- Background blur replays already-rendered background commands through Gaussian taps clipped to the affected shape.
- Text render commands now carry a full brush instead of a fallback color, enabling true linear/radial gradient multi-fill text on canvas.
- PDF text outlines now clip native gradient shadings per glyph instead of collapsing gradients to their first stop.
- Expanded blend parity from six modes to the complete 16-mode SVG/Vello set, including independently persisted effect-layer blend modes.
- Effects inspector now exposes four visible effect rows, type cycling, radius controls, visibility and stack ordering without overlapping the styles section.
