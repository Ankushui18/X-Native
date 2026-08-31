# X Designer — Product and Figma-Parity Audit

Audit date: 2026-08-29  
Scope: static review of the supplied Rust workspace and its regression fixtures. The current environment does not include the Rust toolchain or a macOS display, so this pass does not claim a compiled or live-Mac verification.

## Executive assessment

X Designer is already a real native editor, not an interface prototype. The strongest work is below the UI: its document model, undoable editor operations, render pipeline, import/export path, recovery system, and regression coverage form a credible foundation.

It is not at 100% Figma parity. The repository's older roadmap and README overstate and understate different areas, while the session log records newer behavior. A reliable parity claim must be tied to scenario tests, not a feature count.

The right goal is workflow parity for professional design work while preserving X Designer's own product identity. Pixel-copying Figma's chrome would add legal/product risk and would not solve the missing workflows.

## Current capability map

| Area | Current state | Assessment |
| --- | --- | --- |
| Canvas/navigation | Pan, zoom-to-cursor, fit/100%, culling, minimap, rulers/guides, outline mode | Strong beta |
| Selection/editing | Deep select, drill-in, marquee, move/resize/rotate, snapping, align/distribute, group, z-order | Strong beta |
| Vector work | Paths, anchors/handles, boolean operations, masks | Beta; advanced vector UX still behind |
| Text | Real shaping pipeline, font discovery, caret/ranges, spacing and line-height, export outlines | Useful; rich text and full IME remain gaps |
| Layout | Constraints and recursive auto layout | Useful; wrap and deeper responsive behavior need scenario coverage |
| Design systems | Components, instances, variants, variables/modes, styles, libraries | Broad foundation; management and override UX need hardening |
| Visual fidelity | Ordered paint/effect stacks, on-canvas gradients, four classic effects, 16 blend modes, corner radii | Strong beta; texture/noise/glass and advanced strokes remain gaps |
| Files/reliability | Multi-page `.x`, autosave, backups/recovery, dashboard lifecycle | Strong, with one multi-document autosave bug fixed in this pass |
| Interop/export | `.x`, Sketch and Figma REST JSON import/export plus SVG/PNG/PDF | Useful editable interchange; proprietary native `.fig` parity is not present |
| Prototype | Links, navigation, smart animate, presentation | Early beta; overlays, scroll behavior, flows and richer triggers are missing |
| Collaboration/platform | Named checkpoints and dev CSS exist | Multiplayer, comments, plugin runtime, production packaging absent |

## Highest-priority gaps against Figma workflows

### P0 — Mac-native trust

1. ✅ Native open/save/import/export panels are implemented; verify them in the signed Mac build.
2. Validate text input with macOS IME, dead keys, emoji, and international keyboard layouts.
3. Add trackpad magnify/inertial pan behavior and verify Retina scaling across displays.
4. Add unsaved-close confirmation, app menu integration, recent documents, and crash-recovery presentation.
5. Package, sign, notarize, and test a universal Apple Silicon/Intel build.

This pass adds native file panels, `pbcopy`/`pbpaste`, Command notation, and fixes autosave so it writes beside the active document rather than always targeting the default document.

### P0 — Daily design loop

1. Typed independent corner fields and on-canvas radius handles. (fields complete; canvas radius handles remain)
2. Mouse-based text range selection with shaped-glyph hit mapping.
3. ✅ Multiple fills/strokes/effects with reorder, visibility, blend, and per-paint opacity.
4. ✅ Gradient handles and arbitrary stop editing directly on canvas.
5. Paste-in-place, paste-over-selection, and reliable cross-app object clipboard semantics.
6. Drag reorder for pages, layers, auto-layout children, and component properties.

### P1 — Responsive systems

1. Auto-layout wrap, min/max sizing, absolute children, baseline alignment, and robust nested fill/hug behavior.
2. Component properties: boolean, text, instance swap, preferred values, nested expose, reset/detach.
3. Variable collections with scopes, modes, aliases, bulk editing, rename/delete safety, and missing-library resolution.
4. Layout grids and responsive constraints surfaced coherently in the inspector.

### P1 — Prototype and handoff

1. Overlay/open/close/back/scroll-to actions and hover/press/drag/delay triggers.
2. Fixed/sticky elements and frame overflow scrolling.
3. Flow starting points, device frames, shareable presentation links.
4. Inspect/dev mode with measurements, tokens, assets, CSS/Swift/Compose output, and copy controls.

### P2 — Team/product platform

1. Comments and annotations.
2. Multiplayer presence and conflict-safe document sync.
3. Version history beyond local named checkpoints.
4. Sandboxed plugin API and extension management.
5. Accessibility tree, keyboard navigation, focus visibility, and configurable UI scaling.

## Interface direction

The existing layout is familiar and efficient, but it contains explicit Figma mimicry: Figma-blue selection, Figma-labelled comments, and copied structural assumptions. The UI should instead follow these rules:

- X identity: graphite surfaces with a violet action/selection color.
- Canvas-first: quiet chrome, fewer permanent borders, progressive disclosure for advanced properties.
- Native Mac language: Command/Option/Shift glyphs, proper menu hierarchy, trackpad-first navigation.
- Inspector clarity: 260 px panel, semantic sections, consistent 22–28 px controls, clear mixed states.
- One component system: menus, tooltips, popovers, fields, steppers and confirmation dialogs should all consume the same theme tokens.
- Status by meaning: violet for selection/action, green for success, amber for recovery/warnings, red for destructive/error states.

The theme and panel-density changes in this pass begin that separation without copying Figma's visual identity.

## Definition of “100%”

Do not use “all features implemented” as the finish line. Use a versioned acceptance suite:

1. Build a 50–100 scenario corpus covering creation, editing, responsive layouts, components, variables, prototypes, import/export, recovery, and 10k/100k-layer performance.
2. Record expected document state, rendered output, undo/redo state, and keyboard/mouse behavior for each scenario.
3. Run the same user workflow in Figma only as a behavioral reference.
4. Score each scenario on correctness, visual fidelity, latency, and recoverability.
5. Claim parity only for the named workflow set and platform build.

Recommended release gates: zero data-loss bugs; 100% pass on P0 scenarios; at least 95% visual similarity on controlled fixtures; p95 interaction latency below 16 ms for common operations; successful recovery after forced termination; keyboard-only completion of core file and editing flows.

## Next implementation sequence

1. Mac-native shell: dialogs, clipboard validation, close/save lifecycle, trackpad/Retina, packaging.
2. Inspector/editor completeness: on-canvas radius handles, text hit mapping and advanced stroke popovers.
3. Auto layout/components/variables acceptance corpus and fixes.
4. Prototype scrolling/overlays/flows and presentation handoff.
5. Collaboration, comments, plugins, accessibility.
