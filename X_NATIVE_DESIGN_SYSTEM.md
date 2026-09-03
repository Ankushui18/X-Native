# X-Native Design System — v2 "Ink & Ember"

> **Authoritative.** This document supersedes the v1 "Graphite & Violet"
> spec in its entirety (v1 is retired; its violet accent, surface ramp, and
> icon conventions no longer exist in the codebase). `theme.rs` is the
> machine-readable source of truth for every token printed here; if the two
> disagree, `theme.rs` wins and this doc must be updated in the same commit.
> Historical note: the pre-v1 "Arco" exploration is also retired — permanently.

## 1. Identity: why Ink & Ember

X-Native is **not** a Figma clone and not a Sketch clone. The 2026 field:

| | Figma (UI3) | Sketch | X-Native v2 |
|---|---|---|---|
| Surface | neutral dark, ~5 grays | light, minimal | **cool ink ramp** (7 steps, blue-leaning) |
| Accent | blue `#0D99FF` (and UI3's growing multi-accent set) | red/pink, sparse | **one warm Ember**, everywhere discipline is needed |
| Density | roomy, 36–44px rows | airy | **28px rows as a feature** — pros want layers on screen |
| Icons | in-house set, mixed weights | crisp, light strokes | **one stroke-only set, 24-grid, 1.8 weight** |
| Personality | friendly-neutral | quiet-craft | **calm ink, one spark** |

The differentiator is a single warm accent on a cool ink canvas. Every
competitor uses a cool accent on a neutral or cool canvas; warm-on-cool gives
instant brand recognition, and because there is exactly **one** accent, the
eye learns "Ember means I can act here." Discipline is the design:
Ember appears on exactly three things — **selection, the primary action,
and the active underline**. Everything else is neutral.

The palette was chosen against WCAG, not taste: every text token in this
document ships with its measured contrast ratio (§3).

## 2. Geometry

```rust
TAB_H:       30   // document tab strip
HDR_H:       42   // header row 2 (menus · tool dock · zoom · actions)
TOP_H:       72   // = TAB_H + HDR_H (chrome above the canvas)
BOTTOM_BAR_H 28   // (legacy name) tool dock band, now inside the header
STATUS_H:    24   // bottom status bar
THUMBS_H:    88   // page thumbnail strip
LAYERS_W:    264  // left panel
RAIL_W:      44   // left icon rail (rail tabs are square: LTAB_H = RAIL_W)
INSPECTOR_W: 288  // right panel
ROW_H:       28   // list rows (layers, pages, assets) — density is a feature
PAD:         16   // standard panel padding
```

Density is intentional: 28px rows put ~40 layer rows on screen where Figma's
UI3 rows put ~26. Rulers, when visible, are 16px strips inset in the canvas.

## 3. Color — Ink ramp + Ember

### Surfaces (the ink ramp, cool-leaning)

| Token | Value | Role |
|---|---|---|
| `C_BASE` | `#0E1117` | window backdrop / dashboard base |
| `C_CANVAS` | `#131720` | artboard area |
| `C_PANEL` | `#181D27` | header, tabs, panels |
| `C_PANEL2` | `#1F2531` | raised: menus, popovers, modals |
| `C_FIELD` | `#11151D` | recessed wells: inputs, search, thumbnails |
| `C_HOVER` | `#262D3B` | hover fill |
| `C_PRESSED` | `#2C3444` | pressed fill / active tool slot |

The ramp is strictly ordered: `FIELD < PANEL < PANEL2 < HOVER < PRESSED`
in lightness, all hue-shifted toward blue (~222°) so the chrome reads *cool*.

### Lines

| Token | Value | Use |
|---|---|---|
| `C_EDGE` | `#232A38` | subtle hairlines |
| `C_EDGE_2` | `#303949` | strong dividers |
| `C_HOVERLN` | `#EAECF1 @ 55%` | neutral hover outline (canvas) |

### Text (contrast measured against `C_PANEL` `#181D27`)

| Token | Value | Contrast | Role |
|---|---|---|---|
| `C_TEXT` | `#EAECF1` | 13.6:1 | primary |
| `C_DIM` | `#A8AFBF` | 7.6:1 | secondary, section headers |
| `C_FAINT` | `#7E8698` | 4.7:1 | tertiary (readable floor) |
| `C_OFF` | `#4A5160` | 2.0:1 | disabled only — never informational |
| `C_ACCENT_TXT` | `#FFA259` | 8.9:1 | Ember *as text* / focus ring |
| `C_ON_ACCENT` | `#1A1206` | 6.9:1 | ink-on-Ember (text on accent fills) |

All interactive text meets or exceeds WCAG AA (4.5:1). `C_ACCENT`
(`#F97B22`) itself is 4.6:1 on `C_PANEL` — it is used as a **fill/stroke
token**, and when Ember must carry small text we use `C_ACCENT_TXT` instead.

### Ember (the only accent)

| Token | Value | Role |
|---|---|---|
| `C_ACCENT` | `#F97B22` | base — selection stroke, primary buttons, underlines |
| `C_ACCENT_HOV` | `#FF9049` | hover state of accent fills |
| `C_ACCENT_PRS` | `#E16A12` | pressed state |
| `C_ACCENT_TXT` | `#FFA259` | accent as text / focus ring |
| `C_ON_ACCENT` | `#1A1206` | content on accent fills (dark-on-light, not white) |
| `C_SELECTED` | `#F97B22 @ 14%` | selected rows/tiles wash |
| `C_FOCUS_RING` | `#FFA259 @ 70%` | keyboard focus ring |

**Accent discipline — the three allowed uses:** (1) selection & active
state marks (outline, underline, wash); (2) exactly one primary action per
surface; (3) focus. Menu hovers are neutral (`C_HOVER`), secondary buttons
are neutral, the active tool slot is `C_PRESSED` with an Ember glyph —
**not** an all-accent block. White text is reserved for on-accent and
on-canvas emphasis; on dark chrome the strongest text is `C_TEXT`.

### Semantic

`C_OK #3ECF8E` · `C_WARN #F5B83D` · `C_DANGER #F2545B` · `C_INFO #5CA8FF`
· `C_SNAP #FF4D6D` (snap guides/rulers) · `C_GRID #EAECF1 @ 14%` ·
`C_SHADOW`/`C_SHADOW2`/`C_SCRIM` (black @ 40/50/50%).

### Canvas interaction colors

Selection stroke = `C_SELECT` (`= C_ACCENT`). Hover outline = `C_HOVERLN`
(neutral — deliberately distinct from the Ember selection). Vector-edit
handle lines = Ember @ 67%; anchors are white-filled, Ember-stroked.
Smart guides = red `#FF3B30`; skew/guide lines = cyan `#00BCD4 @ 70%`.
These canvas-tool colors are semantic, not brand, and stay literal.

### Paper (light theme — planned, not yet implemented)

The ramp inverts while Ember stays: `Paper 0 #FFFFFF`, `Paper 1 #F7F8FA`,
`Paper 2 #EEF0F4`, hover `#E4E7EC`, pressed `#D9DDE3`, field `#EDEFF3`,
edges `#E2E5EA`/`#C7CCD4`, text `#16181D`, dim `#5A6170`, faint `#8A919E`,
off `#B6BCC7`. Ember text on Paper uses `#D9640A` (4.6:1 on white); fills
keep `#F97B22` with `C_ON_ACCENT` content. Ships only when every token in
this document has a Paper counterpart — no partial themes.

## 4. Radii

| Token | Value | Use |
|---|---|---|
| `R_XS` | 3 | chips, tiny affordances |
| `R_SM` | 6 | buttons, inputs, list-row hovers |
| `R_MD` | 10 | menus, popovers, tooltips, palette, HUD, minimap, tool dock |
| `R_LG` | 14 | modals, dialogs, large cards |
| `R_XL` | 20 | hero/dashboard cards |

Compat aliases `RADIUS_SM/MD/LG` map to `R_SM/R_MD/R_LG`.

## 5. Typography

- **UI font:** system UI stack via the text pipeline; monospace for values
  is supplied by the same stack (no separate brand font yet).
- **Scale:** 20 / 16 / 14 / 13 / 12 / 11 / 10 / 9 / 8 px. 11px is the
  default for chrome labels (menus, tabs, buttons, zoom); 9–10px for
  micro-labels (rail captions, status, shortcut hints); 8px reserved for
  all-caps section headers set in `C_DIM`.
- `label()` takes the **top** of the glyph box, not a baseline — center
  text in a band by `y = band_y0 + (band_h - size) / 2`.
- Section headers: 11px caps in `C_DIM`, never `C_ACCENT`.

## 6. Iconography — one library, `icons.rs`

Every chrome glyph — toolbar, layer list, left rail, status bar, menus,
window controls — comes from the single in-binary library `icons.rs`:

- **24×24 design grid**, rendered at 9–18px (`icons::paint(scene, icon,
  cx, cy, size, color)` — centered, scaled `size/24`).
- **Stroke only** — 1.8-unit stroke (floored at 1px device), round caps
  and joins, no fills in chrome. One `BezPath` per icon, subpaths for
  open/closed parts; circles built from four kappa arcs so everything
  strokes identically.
- Lucide-inspired open-form geometry (45° cuts, optically balanced
  circles), but self-contained: zero assets, no font dependency.
- Mappings live beside the glyphs: `tool_icon(Tool)` covers **all 17
  tools**; `kind_icon(&NodeKind)` covers all 13 node kinds (Frame, Group,
  Section, Rect, Ellipse, Arc, Line, Text, Image, Vector, Component,
  Instance, Slice).
- The set ships complete (~70 variants) like an icon package; unused
  variants are allowed so future chrome never reintroduces ad-hoc paths.
- **No text-glyph icons** ("O" for eye, "*" for lock, "T" for text layers
  are retired) and no per-surface hand-drawn marks.

## 7. Chrome anatomy

1. **Tab strip (30px, `C_PANEL`):** monochrome X mark + wordmark in
   `C_TEXT`; active document tab = no background, full-contrast text,
   **2px Ember underline inset 8px, flush to the strip's bottom edge**;
   dirty dot at tab end; `+` in library stroke weight; window controls
   from the library (Minus / Minimize / Close, 11px, `C_DIM`).
2. **Header row 2 (42px, `C_PANEL2`):** menus at 11px with neutral hover;
   center **tool dock** — a recessed `C_FIELD` pill (`R_MD`, hairline
   `C_EDGE`, soft shadow) holding all 17 tools at **40px pitch, 34px
   slots**; active tool = `C_PRESSED` slot + Ember glyph, hover = neutral;
   zoom pill; Share / Prototype as ghost buttons; **Present** is the one
   accent pill (ink-on-Ember content).
3. **Left rail (44px):** square icon tabs — 18px library glyph above a
   9px micro-label; active = Ember glyph + `C_SELECTED` wash + Ember
   underline at the rail's foot; hover = white @ 4%.
4. **Left panel (264px):** `C_FIELD` search with library magnifier,
   `C_FOCUS_RING` focus; Pages with real thumbnails in `C_FIELD` wells;
   Layers rows at 28px — `C_SELECTED`/`C_HOVER` row fills, library caret +
   kind glyph, Eye/EyeOff/Lock affordances (hover-revealed, `C_WARN` when
   engaged); proportional scrollbar.
5. **Inspector (288px):** Design/Prototype tabs use the same text +
   Ember-underline language as the document strip; recessed fields with
   `C_FOCUS_RING`; export rows as neutral chips (hover = `C_HOVER`);
   the **Export button is the panel's single accent action**.
6. **Status bar (24px):** `C_OK` ready dot, 10px text, live focus/
   edit line; selection geometry + zoom right-aligned.
7. **Popovers/modals:** `C_PANEL2` on `C_SCRIM`, `R_MD`/`R_LG`, `C_EDGE`
   hairline, `C_SHADOW`/`C_SHADOW2` drop shadows; menu rows hover in
   neutral `C_HOVER` (never accent), disabled rows in `C_OFF`.

## 8. Interaction states

| State | Fill | Content |
|---|---|---|
| rest | `C_FIELD` or panel | `C_TEXT` / `C_DIM` |
| hover | `C_HOVER` | `C_TEXT` |
| active/pressed | `C_PRESSED` | `C_TEXT` (+ Ember glyph for the active tool) |
| selected | `C_SELECTED` wash | `C_TEXT`/white |
| focus | unchanged + `C_FOCUS_RING` stroke | unchanged |
| primary action | `C_ACCENT` → `C_ACCENT_HOV` → `C_ACCENT_PRS` | `C_ON_ACCENT` |
| disabled | unchanged | `C_OFF` |

## 9. Accessibility

- All interactive text ≥ 4.5:1 (§3 table); focus rings ≥ 3:1 (`C_FOCUS_RING`
  is 8.9:1 tinted); state is never carried by color alone (lock/eye glyphs,
  dirty dot, text emphasis).
- Full keyboard model: tool single-keys and ⇧-variants, ⌘· menus, ⌘.
  hide-interface, arrow/nudge, ⌥⌘K components, ⇧R rulers, ⌘G/⌥⌘G grouping.
- The canvas is the only element allowed pure white text; chrome maximum is
  `C_TEXT` so selection/active states keep the highest local contrast.

## 10. Motion

Chrome motion is currently implicit (hover/active color steps, no
transitions in the vello UI layer). Policy when animation lands: 120ms for
state feedback, 200ms for popover/panel reveals, ease-out; no motion on
text; respect reduced-motion (drop to instant state swaps).

## 11. Differentiators from Figma / Sketch (v2 recap)

1. One warm accent on a cool ink field vs everyone's cool-on-neutral.
2. 28px density as a pro feature, measured against UI3's roomier rows.
3. A single stroke-only icon system across every surface, 1.8 weight.
4. WCAG-measured tokens published as code (`theme.rs`) and prose (here).
5. Active-tab/tool language that favors neutral surfaces + Ember marks
   over Figma-style tinted blocks — quieter chrome, louder content.

## 12. Governance

- Tokens change in `theme.rs` **and** this doc in the same commit.
- No new hex literal in `chrome.rs`/`helpers.rs` outside: canvas tool
  colors (§3), avatar/content imagery, and black scrims/shadows via
  tokens. New chrome color = new token.
- New glyph = new `Icon` variant + `paint` arm; no ad-hoc paths.
- Gates: 33 test suites, clippy clean at default level, rustfmt clean.
