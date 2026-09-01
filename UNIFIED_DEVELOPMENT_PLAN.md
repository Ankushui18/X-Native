# X-Native2 Unified Development Plan

## Philosophy: Functional Parity + Distinct Identity (NOT Clone-Then-Fix)

We study Figma's workflows to understand professional designer needs, but implement everything with X-Native's own visual identity from day one. This avoids legal risk while building competitive features.

---

## Phase 1+2 Combined: Immediate Actions

### 1. UI Audit & Cleanup (Week 1)
**Goal:** Ensure zero copyright exposure while maintaining professional ergonomics

- [x] Theme already uses graphite/violet (not Figma blue) ✅
- [ ] Replace any "Figma-style" comments in code with neutral language
- [ ] Audit icon set - ensure all icons are original or properly licensed (Lucide/Heroicons)
- [ ] Remove "92% parity" claims from documentation - replace with honest status
- [ ] Add legal disclaimer: "X-Native is independent software, not affiliated with Figma/Adobe"

### 2. Command Palette - Cmd+K (Week 1-2)
**Inspired by:** Spotlight/Alfred pattern (industry standard, not Figma-specific)

```rust
// Implementation priority:
- Search layers, components, actions, shortcuts
- Fuzzy matching
- Keyboard navigable
- Recent items prioritized
```

### 3. Focus Mode - Distraction-Free Editing (Week 2)
**Your innovation:** All panels fade, canvas fills 95% screen

```rust
// Toggle: Cmd+Shift+F
- Hide all chrome except active tool options
- Suppress notifications
- Optional: isolate current layer
```

### 4. Auto-Layout Completeness (Week 2-3)
**Functional parity without copying:**

- [ ] Wrap behavior for overflowing children
- [ ] Min/max sizing constraints
- [ ] Baseline alignment
- [ ] Absolute positioning within auto-layout frames

### 5. Inline Property HUD (Week 3)
**Better than Figma:** Edit without losing canvas focus

```
Selected element shows floating controls:
┌─────────────────────────────┐
│ W: 320  H: 240  ↻ 45°       │
│ Radius: [●━━━○] 12px        │
│ Fill: [■] #7C5CFC  [Opacity]│
└─────────────────────────────┘
```

### 6. Spatial Navigator Enhancement (Week 3-4)
**Your differentiator:** 2D map vs Figma's flat list

- [ ] Mini-map showing entire canvas layout
- [ ] Color-coded by node type (frames/text/vectors/components)
- [ ] Click-to-select + zoom-to-fit
- [ ] Shows overlap/z-index visually

### 7. Rich Text Engine Improvements (Week 4-5)
**Critical gap from PRODUCT_AUDIT.md:**

- [ ] IME support for international input
- [ ] OpenType features (ligatures, stylistic sets)
- [ ] Better hit-testing for mouse selection
- [ ] Text range formatting UI

### 8. Component Properties UI (Week 5-6)
**Professional workflow requirement:**

- [ ] Boolean property toggles
- [ ] Text override fields
- [ ] Instance swap dropdowns
- [ ] Nested instance expose/reset
- [ ] Preferred values UI

---

## Legal Safeguards (Ongoing)

### DO Study:
- ✅ Professional workflows (how designers actually work)
- ✅ Feature specifications (what capabilities users expect)
- ✅ UX patterns that are industry standards (pan/zoom, layers panel concept)
- ✅ Performance benchmarks (what feels "fast enough")

### DO NOT Copy:
- ❌ Exact color values from Figma's palette
- ❌ Figma's icon designs (use Lucide, Heroicons, or custom)
- ❌ Proprietary file formats (.fig binary)
- ❌ Trademarked terms ("Dev Mode", "Auto Layout" is borderline - consider "Responsive Layout")
- ❌ Exact panel layouts and spacing
- ❌ Figma's specific micro-interactions/animations

### Documentation Changes Required:
1. Replace "Figma parity" language with "professional workflow support"
2. Remove percentage claims (92%, 100%) - use scenario-based testing instead
3. Add clear independence disclaimers
4. Reference open standards, not reverse-engineering

---

## Success Metrics (From UI_UX_REVOLUTION.md)

| Metric | Current | Goal | Measurement |
|--------|---------|------|-------------|
| Time to first edit | ~2s | <0.5s | Cold launch benchmark |
| Panel clicks per task | 3-5 | 1-2 | Usability testing |
| FPS with 1000 elements | ~60 | 120+ | Performance overlay |
| Accessibility score | Unknown | 95+ | AccessKit audit |
| Learning curve | 2-3 days | 1 day | User onboarding tests |

---

## Next Sprint Priorities

### Week 1: Foundation
1. Audit all UI assets for licensing
2. Implement Cmd+K command palette
3. Add Focus Mode toggle
4. Update documentation language

### Week 2-3: Navigation & Editing
1. Build spatial navigator mini-map
2. Implement inline property HUD
3. Start auto-layout wrap support

### Week 4-6: Professional Features
1. Complete rich text engine
2. Component properties authoring UI
3. Auto-layout min/max sizing

---

## Key Principle

**We're not building "Figma but offline." We're building the future of design tools that happens to work offline.**

Leverage your actual advantages:
- Native Rust performance (web tools can't match)
- True offline-first architecture
- Local file ownership
- Open .x format
- GPU acceleration via Vello
- No subscription model

These are your differentiators - not how closely you mimic Figma's interface.
