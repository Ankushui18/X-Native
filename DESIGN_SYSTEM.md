# X-Native Design System — “Graphite & Signal”

**Version:** 1.0 (greenfield — no inheritance from prior X-Native UI)  
**Intent:** Professional native desktop tool. Calm, dense, discoverable.

---

## Principles

1. **Canvas first** — chrome is quiet; the artboard is the hero  
2. **One accent** — Signal teal for selection, focus, primary actions only  
3. **Progressive disclosure** — inspector shows what the selection needs  
4. **Native density** — 24–28px rows, 11–12px type, 8–12px padding  
5. **Token-only chrome** — no raw hex outside `theme.rs`  

---

## Surfaces (dark, default)

| Token | Hex | Use |
|-------|-----|-----|
| `surface.base` | `#0C0E12` | Window backdrop |
| `surface.canvas` | `#12151C` | Infinite canvas void |
| `surface.panel` | `#161A22` | Side panels, header |
| `surface.raised` | `#1C222D` | Menus, popovers, fields |
| `surface.hover` | `#252B38` | Row/tool hover |
| `surface.active` | `#2C3444` | Pressed / selected tool |

## Borders

| Token | Hex | Use |
|-------|-----|-----|
| `border.subtle` | `#2A3140` | Dividers |
| `border.strong` | `#3A4458` | Focus wells, inputs |

## Text

| Token | Hex | Use |
|-------|-----|-----|
| `text.primary` | `#E8EBF2` | Primary labels |
| `text.secondary` | `#9AA3B5` | Secondary / hints |
| `text.faint` | `#6B7385` | Captions, disabled-ish |
| `text.onAccent` | `#041016` | Text on Signal buttons |

## Accent — Signal

| Token | Hex | Use |
|-------|-----|-----|
| `accent.default` | `#2DD4BF` | Primary actions, selection |
| `accent.hover` | `#5EEAD4` | Hover |
| `accent.muted` | `#2DD4BF33` | Soft selection fill |
| `accent.danger` | `#F87171` | Destructive |

## Spacing scale

`4, 8, 12, 16, 20, 24, 32`  
Panel padding: **12**  
Section gap: **16**  
Row height: **26**  

## Typography

| Role | Size | Weight |
|------|------|--------|
| Caption | 11 | Regular |
| Body | 12 | Regular |
| Label | 12 | Medium |
| Section | 11 | Medium, uppercase tracking |
| Title | 14 | Medium |

Prefer system UI font stack (SF Pro / Segoe UI / Inter fallback).

## Radii

| Token | Value |
|-------|-------|
| `radius.sm` | 4 |
| `radius.md` | 6 |
| `radius.lg` | 8 |
| `radius.pill` | 999 |

## Shadows

Minimal. One soft elevation for menus/modals only:

`0 8px 24px rgba(0,0,0,0.45)`

## Layout chrome (default desktop)

| Region | Width / height |
|--------|----------------|
| Title / menu strip | 36px |
| Tool rail | 48px |
| Left panel (Layers) | 240px |
| Right panel (Inspector) | 260px |
| Status bar | 24px |
| Canvas | Flex remainder |

## Icons

Stroke-only, 1.5px, 16×16 optical size in 20×20 hit targets.  
No filled skeuomorphism. Consistent corner rounding on paths.

## Motion

- Hover: instant or ≤80ms  
- Panels: ≤120ms ease-out  
- Respect `prefers-reduced-motion`  

## Accessibility

- Focus ring: 2px `accent.default`  
- Minimum hit target: 24px  
- UI scale token: 1.0 default, 1.25 / 1.5 supported  

---

## What we explicitly avoid

- Figma purple / blue toolbar clone  
- Sketch yellow accent clone  
- Old X-Native Ember orange or Azure blue systems  
- Huge rounded SaaS cards  
- Permanent export grids on empty canvas  
- Listing the page node inside Layers  
