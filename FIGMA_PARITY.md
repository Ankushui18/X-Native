# X Designer vs Figma Design — Current Feature Matrix

Audit date: 2026-08-30  
Method: static source inspection of the supplied Rust workspace, compared with current Figma Help Center documentation. No build was produced in this wave and the environment has no Rust toolchain, so runtime verification remains a release gate.

Status legend: **Complete** = implemented across model/editor/render/file path; **Partial** = usable implementation with known workflow gaps; **Missing** = not implemented.

## X Designer feature inventory

### Workspace and navigation

- Native dashboard and recent-document lifecycle
- Multi-page documents with create, switch and rename
- Infinite-canvas pan, zoom-to-cursor, fit and 100% views
- Rulers, guides, outline view, minimap and presentation mode
- Layer search, hierarchy, visibility, lock, z-order and drill-in
- Offline-first `.x` files, autosave, recovery and checkpoints

### Creation and editing

- Move, Hand, Scale, Frame, Rectangle, Ellipse, Line, Polygon, Star, Text and Pen tools
- Frame presets and configurable rectangle, polygon and star defaults
- Click, deep-select, Shift toggle, marquee and select-all
- Multi-selection move, resize, rotate, align and distribute
- Shift constraints, 15-degree rotation snap, 45-degree line snap and Alt-from-center creation
- Smart guides, edge/center snapping and keyboard nudging
- Copy, cut, paste, duplicate, Alt-drag duplicate, delete, group and ungroup
- Z-order controls, flip horizontal/vertical and parent drill navigation
- Atomic undo/redo for document and visual-stack changes
- Inline layer/frame rename with reference-safe ID rewriting

### Vector and geometry

- Editable vector paths, anchors and Bézier handles
- Pen path creation, close path, and move/convert/delete anchors
- Union, subtract, intersect and exclude boolean operations
- Masks and clip paths
- Independent corner radii and standard corner radius
- Transform-aware hit testing for rotated shapes and ellipses
- Stroke width, inside/center/outside alignment, cap, join, dash, offset and miter model/render/export

### Paint, effects and compositing

- Ordered multiple fill, stroke and effect layers
- Per-layer visibility, opacity, ordering and blend mode
- Solid, variable, linear-gradient and radial-gradient paints
- On-canvas linear/radial gradient geometry handles
- Arbitrary gradient stop select/add/remove/drag and color editing
- GPU-vector blurred drop shadow and clipped inner shadow
- Shape, stroke, text and image layer blur
- Clipped background replay blur
- Normal, Darken, Multiply, Color Burn, Lighten, Screen, Color Dodge, Overlay, Soft Light, Hard Light, Difference, Exclusion, Hue, Saturation, Color and Luminosity modes
- Canvas/SVG multi-fill text; true gradient glyph brushes on canvas and PDF shading clips

### Text

- Font discovery and load-on-demand font management
- Shaped glyph outlines, font family/weight binding and fallback
- Inline text editing, caret, range selection, letter spacing and line height
- Multi-fill text with per-fill opacity, blend and gradients
- SVG/PDF text-outline export parity

### Responsive layout and design systems

- Constraints: left/right/center/stretch/scale and top/bottom/center/stretch/scale
- Recursive auto layout with horizontal/vertical direction, gap, padding, alignment, space-between and hug/fill foundations
- Components, instances, detach, swap and variant discovery
- Typed instance overrides for text, fill, visibility, opacity and component swap
- Color/number/string/boolean variables, aliases and light/dark modes
- Paint, text and effect styles plus local library snapshots/dependencies
- Asset, component and library-management foundations

### Files, import and export

- Versioned `.x` serialization and migration
- Native `.x`, SVG, Sketch, Figma REST JSON and PNG import paths
- Native `.x`, editable Sketch package, editable Figma REST JSON, PNG, SVG and PDF export
- Embedded images with fill/crop/fit/tiling placement
- SVG gradients, masks, images, text outlines, stroke properties and supported blend modes
- PDF vector paths, embedded images, shaped text and gradient text shadings

### Prototype, developer and performance

- Click navigation, back stack, page destinations and transition metadata
- Smart animate for matched layers and presentation playback
- Per-node CSS export and inspect foundations
- Spatial-grid hit testing, viewport culling, frame cache and stable render-command keys
- Large deterministic stress-scene infrastructure

## Comparison with current Figma Design

| Capability | X Designer | Gap compared with Figma |
|---|---|---|
| Canvas navigation | **Complete** | Trackpad/Retina runtime acceptance still required |
| Selection and transforms | **Complete** | Paste-over-selection and richer transform origins remain |
| Basic shape tools | **Complete** | Ellipse arc controls and repeat transforms remain |
| Pen/vector editing | **Partial** | Branching vector networks, Pencil/Brush and width profiles remain |
| Boolean operations and masks | **Complete** | Flatten, outline-stroke polish and luminance controls remain |
| Fill/stroke stacks | **Complete** | Image/gradient strokes, width profiles and final arrow geometry remain |
| Gradient editing | **Complete** | Angular/diamond gradient workflows are not included |
| Four classic effects | **Complete** | Figma's newer texture, noise and glass effects are missing |
| Blend modes | **Complete** | Layer, fill, stroke and effect blend state is modeled, rendered and serialized |
| Text rendering | **Partial** | Mixed rich-text ranges, lists, IME, OpenType and text-on-path remain |
| Auto layout | **Partial** | Wrap, min/max, baseline, absolute children and grid auto layout remain |
| Constraints/resizing | **Partial** | Mixed auto-layout/constraint edge cases need scenario coverage |
| Components/variants | **Partial** | Full property authoring, preferred values and reset-all overrides remain |
| Variables | **Partial** | Scopes, bulk editor, rename/delete safety and missing-library resolution remain |
| Styles/libraries | **Partial** | Grid styles, remote publish/update and conflict UI remain |
| Prototyping | **Partial** | Overlays, scroll-to, hover/press/drag/delay, fixed layers and flow starts remain |
| Presentation | **Partial** | Device frames, scroll containers and shareable sessions remain |
| Import | **Partial** | `.x`, Sketch and Figma REST JSON are wired; proprietary native `.fig`, video and GIF workflows remain |
| Export | **Partial** | `.x`, Sketch package and Figma REST JSON are wired; JPG, slices, multi-scale presets, suffixes and batch export remain |
| Dev Mode/handoff | **Partial** | Full inspect UI and CSS/Swift/Compose panels remain |
| Collaboration | **Missing** | Comments, multiplayer presence, branching and shared history require server architecture |
| Plugins/widgets | **Missing** | Sandboxed plugin runtime, permissions and distribution are absent |
| Accessibility | **Partial** | Full accessibility tree, focus navigation, contrast and annotations remain |
| Offline ownership | **X advantage** | Preserve local files, recovery and offline editing as product differentiation |

## Honest parity assessment

X Designer covers most of the core single-user visual-design loop: creation, selection, transforms, vectors, layered paints, gradients, classic effects, text shaping, layout foundations, components, variables, local files and export.

It is not 100% Figma parity. The largest gaps are collaboration/platform features, advanced auto layout, rich text, full component-property authoring, modern texture/noise/glass effects, advanced prototyping, native `.fig` import, batch export and Dev Mode depth.

“100%” should only be claimed against a versioned workflow corpus with rendered-output, document-state, undo/redo, latency and recovery assertions—not from a raw feature count.

## Current Figma references

- Properties panel: https://help.figma.com/hc/en-us/articles/360039832014
- Effects: https://help.figma.com/hc/en-us/articles/360041488473
- Blend modes: https://help.figma.com/hc/en-us/articles/360040667874
- Strokes: https://help.figma.com/hc/en-us/articles/360049283914
- Variable modes: https://help.figma.com/hc/en-us/articles/15343816063383
- Components: https://help.figma.com/hc/en-us/articles/360038662654
- Export formats: https://help.figma.com/hc/en-us/articles/13402894554519
- Import formats: https://help.figma.com/hc/en-us/articles/360041003114
