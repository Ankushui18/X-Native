# X-Native Design System — FINAL v45 — HTML Source of Truth

**Source of truth:** `prototypes/v45-final-editor-28px.html` (editor) and `prototypes/dashboard-v2.html` (dashboard)
**Library:** Lucide icons (stroke 1.75px rounded) + Tailwind utility pattern — same in HTML and Rust native
**No external product naming — fully native tokens**

---

## 1. Core Palette — Exact from HTML

| Token | Hex | Usage in HTML |
|-------|-----|---------------|
| `C_BG` / `bg` | `#090909` | Window backdrop, body, pill-tabs bg |
| `C_CANVAS` | `#060606` | Canvas void, main dashboard bg |
| `C_PANEL` | `#111111` | Side panels, title bar h-10/h-9, cards |
| `C_FIELD` | `#1A1A1A` | Inputs 28px, toolbar bg, search field 480x32 |
| `C_FIELD_2` | `#222222` | Active pill, active flow btn, hug tag |
| `C_LINE` | `#1F1F1F` | Dividers, card border, input border |
| `C_LINE_2` | `#2A2A2A` | Field hover border, active border, toolbar border |
| `C_TEXT` | `#FFFFFF` | Primary text, active icons, white frame |
| `C_MUTED` | `#999999` | Secondary labels, team names, 12% |
| `C_DIM` | `#777777` | Tertiary, icons, captions, 9px labels |
| `C_FAINT` | `#3A3A3A` | Disabled, placeholder #555555 in search |
| `C_ACCENT_GREEN` | `#1BCB55` | Logo X green fill — custom SVG 28x28 |
| `C_AVATAR` | `#FFEB3B` | Avatar S 28px circle |
| `C_TEAM_L` | `#5B7CFF` | Team Liquor Delivery L badge |
| `C_TEAM_D` | `#FF7A45` | Team Design System D badge |
| `C_DRAFT_DOT` | `#2ECC71` | Drafts personal dot |
| `C_MD_BADGE` | `#519ABA` | Markdown file badge |

---

## 2. Typography — HTML exact

- **Family:** Inter 400/500/600 + JetBrains Mono 400 (mono for W/H/X/Y/100%)
- **Sizes:**
  - 9px tracking 0.12em uppercase — section headers DRAFTS/PAGES/PAGE 3/GUIDES/EXPORT
  - 10px — pill tabs LAYERS/ASSETS/TOKENS DESIGN/PROTOTYPE/INSPECT, opacity/radius labels, hug tags
  - 11px — file name, layer names, input values, Fill/Stroke values, Manrope 14
  - 12px — Welcome, Recents, search, new file button
  - 14px — X-Native title
  - 24px — Welcome back headline
- **Weights:** 400 regular, 500 medium, 600 semibold for pills

---

## 3. Layout — Editor (v45)

| Region | Size | HTML Class | Rust `theme.rs` |
|--------|------|------------|-----------------|
| Title bar | 36px (h-9) | `h-9 panel border-b line` | `TITLE_H = 40.0` (40 for breathing, HTML 36 visual) |
| Logo | 28x28 rx4 | `svg 28x28 rect #1A1A1A + path #1BCB55 + white` | `paint_title_final` 28px at (6, (TITLE_H-28)/2) |
| File tabs | 132px w, 100% h flush | `file-tab active bg #1A1A1A` no floating gap | flush rect, active `C_FIELD` + top white 1.5px |
| New tab + | 32px next to X | `w-8 h-full + icon` | `+` next to close X, expanding to right |
| Left panel | 280px resizable 200-480 | `w-[280px] min 200 max 480` resizer 6px | `left_w 280 clamp 200-480` |
| Right panel | 340px resizable 240-520 | `w-[340px] min 240 max 520` | `right_w 320 clamp 240-480` |
| Canvas | flex remainder #060606 | `canvas bg #060606` | `C_CANVAS` |
| Frame | 375x420 default white | `bg-white rounded 8 shadow-2xl` | `520x340 scaled * zoom` |
| Bottom toolbar | 260x36 centered | `h-10 px-1.5 bg #1A1A1A/90 backdrop-blur border #2A2A2A rounded 12` | `bar_w 260 bar_h 36` |
| Status | 24px | `STATUS_H 24` | `STATUS_H` |

**Left Panel Details (HTML truth):**
- DRAFTS icon box + more-horizontal
- File name directly below Draft — editable on hover/click pencil icon appears `group-hover:opacity-100`
- Pill tabs LAYERS/ASSETS/TOKENS — `pill-tabs h-30 bg #090909 border #1F1F1F rounded 8 p-2 gap-2` active `#222222 border #2A2A2A`
- PAGES header + Page 3 card `#1A1A1A border #2A2A2A`
- PAGE 3 header + search
- Tree: icons by type — Frame=Board icon, Rectangle=Square, Group=Grid 4 squares, Text=Type, Ellipse=Circle, Vector=PenTool, chevron-right/down for expand

**Right Panel Details (HTML truth) — FINAL v45:**
- Top: avatar S #FFEB3B + 12% + message/history/play icons — unified with pill tabs, no border between
- Pill tabs DESIGN/PROTOTYPE/INSPECT same as left — active #222222
- **Size+Position combined — no heading:**
  - Frame dropdown `Frame ▼` 120px field #1A1A1A + % 100% 90px + eye + lock
  - W 375 + H 420 side by side 28px field + lock
  - X 0 + Y 60
  - Rotation 0
- **Auto Layout:**
  - Header Auto layout + plus dark icon `#1A1A1A border #1F1F1F` not blue
  - Flow label 10px dim + arrow-up-right
  - 4 flow buttons 28px #1A1A1A border #1F1F1F active #222222 — no 09
  - Resizing W 37 Hug + H 75 Hug + maximize
  - Alignment 84x84 dark card #1A1A1A border #1F1F1F rounded 12 cross lines #2A2A2A dots #777777 active white halo 20% white
  - Gap 5 field 28px + Gap icon
  - Padding 0 0 + layout-grid
  - Clip content checkbox 16px border #2A2A2A
- **Appearance:** Opacity 100% + Radius 0 side by side
- **Typography:** Manrope Regular 14 Line 20 -0.16px — full controls
- **Fill above Stroke:** Fill FFFFFF 100% with white swatch 14px + eye + minus
- **Stroke:** 000000 Outside 1 black swatch + % + eye + minus + Position Outside + Weight 1
- **Effects:** + icon only header
- **Guides:** No dropdown icon — header GUIDES chevron-down + plus + Square 16 field 28px — Square icon dark style
- **Export:** Styled like other tabs — header Export + plus dark icon — settings only visible when + clicked (currently hidden shows "No exports — click + to add") — when expanded shows PNG 1x Suffix + EXPORT 1 ELEMENT h-7 small

---

## 4. Layout — Dashboard (v2)

**Top Bar 40px #111111 line #1F1F1F:**
- Logo 28x28 same as editor + X-Native 14px semibold
- Centered search 480x32 field #1A1A1A border #1F1F1F rounded 10 search icon + placeholder + ⌘K chip
- New file button 80x32 field border + plus + avatar S 32px #FFEB3B

**Left Sidebar 260px #111111 border #1F1F1F:**
- DRAFTS section same as editor
- Personal with green dot
- Nav Home active #1A1A1A border #2A2A2A + Recents/Starred/Trash dim
- TEAMS header + Liquor Delivery L #5B7CFF 12 count + Design System D #FF7A45
- Upgrade card field border line rounded 10 sparkles #1BCB55/20

**Main #060606:**
- Welcome back Sahil 24px semibold + 13px muted 12 files edited
- Grid/List toggle
- Quick actions 4 cards 88px panel border line rounded 12 hover #141414 border #2A2A2A — first card white bg + black plus, others #1A1A1A
- Recents header + All files dropdown field + search/more
- File grid 3 cols gap 12 — cards panel border line rounded 12 overflow hidden — preview 140px white or #1A1A1A etc + star on hover — title 12px medium + team • edited 11px dim + members avatars #FFEB3B -ml
- Drafts list panel border line rounded 12 — rows h-12 px-4 gap-3 border-b line hover #1A1A1A

---

## 5. Icon Library — Same in HTML and Rust

HTML uses `lucide@latest` — Rust uses custom `icons.rs` drawing same lucide stroke 1.75px rounded caps/joins:

- Cursor, Board/Frame, Type, Square, Circle, PenTool, Search, Plus, Minus, Home, ChevronDown/Right/Left, Grid, Play, X, Check, Sparkles, ArrowLeftRight/UpDown, Maximize, Lock, Eye, More, AlignLeft/Center/Right/Justify, Layers, File, Drafts, Box, etc.
- **Left panel icons by type:** Group→Grid (4 squares), Rectangle→Square, Text→Type, Ellipse→Circle, Vector→PenTool, Frame→Board

---

## 6. Interactions — HTML truth implemented in Rust

- **Panel resize:** mouse near edge 6px shows ↔ status, drag clamps left 200-480 right 240-480, resizer 6px hover rgba(255,255,255,0.08)
- **File rename:** hover pencil appears, click → input field #1A1A1A border #2A2A2A rounded 6
- **Top tabs:** continue expanding to right, + next to X, close X 16px rounded 4 hover #2A2A2A
- **Frame dropdown:** Frame replaces Normal, default 375x420, click cycles Desktop 1440x900 Laptop 1280x800 Tablet 768x1024 Mobile 375x812
- **Auto Layout:** Flow buttons active #222222, 9-dot grid purple selected halo — in Rust dark version white halo
- **Export:** collapsed by default, + toggles
- **Guides:** no dropdown icon, only Square 16

---

## 7. Build — No external naming

All code references use `X-Native`, `C_BG`, `C_PANEL`, `C_FIELD`, `C_LINE`, `C_TEXT`, `Icon::Grid` etc. — no `figma` or `framer` strings in `apps/x-designer/src/bin/x_native_app/`.

```
cargo build -p x-designer --bin x_native_app --release
```

Binary: `target/release/x_native_app` 16MB

---

## 8. Zip & GitHub

- Zip: `x-native-final-v45.zip` contains binary + prototypes + design system + README
- GitHub: repo `Ankushui18/X-Native` — push main, test via `cargo run -p x-designer --bin x_native_app`

---

**This file is the complete design system — HTML is source of truth.**
