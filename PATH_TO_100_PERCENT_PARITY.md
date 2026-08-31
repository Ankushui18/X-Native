# X-Native — Path to 100% Figma Parity

## Executive Summary

**Current State (v0.5):** ~65-70% core feature parity with Figma for single-user visual design workflows.

**Critical Gaps:** The remaining 30-35% includes essential daily-designer workflows that prevent professional adoption.

**Recommended Strategy:** Prioritize by **workflow completion** rather than feature count. Focus on P0 gaps that block real design work.

---

## Priority Matrix: What Blocks 100% Parity

### 🔴 P0 — Daily Design Loop Blockers (Must-Have for Professional Use)

| Feature | Figma Capability | X-Native Status | Effort | Impact |
|---------|------------------|-----------------|--------|--------|
| **Text Selection & Editing** | Full mouse range selection, IME, rich text spans | Partial: caret/ranges exist, no mouse hit-mapping | High | Critical |
| **Auto Layout Complete** | Wrap, min/max, baseline, absolute children, nested solving | Partial: basic horizontal/vertical only | High | Critical |
| **Component Properties** | Boolean, text, instance-swap, preferred values, nested expose | Partial: basic overrides only | Medium | Critical |
| **Paste Behavior** | Paste-in-place, paste-over-selection, cross-app clipboard | Missing | Low | Critical |
| **Corner Radius Handles** | Independent per-corner + on-canvas drag handles | Partial: fields exist, no canvas handles | Medium | High |
| **Vector Networks** | Branching paths, pencil tool, width profiles | Missing: basic pen/boolean only | High | High |
| **Export Completeness** | JPG, slices, multi-scale, batch export, suffixes | Partial: PNG/SVG/PDF only | Medium | High |

### 🟡 P1 — Professional Workflow Gaps

| Feature | Figma Capability | X-Native Status | Effort | Impact |
|---------|------------------|-----------------|--------|--------|
| **Advanced Prototyping** | Overlays, scroll-to, hover/press/drag triggers, fixed layers | Partial: basic navigation only | High | High |
| **Variable Management** | Scopes, bulk editor, rename/delete safety, missing-library resolution | Partial: basic variables/modes | Medium | High |
| **Style Management** | Grid styles, remote publish/update, conflict UI | Partial: local styles only | Medium | Medium |
| **Dev Mode Depth** | Full inspect UI, CSS/Swift/Compose panels, measurements | Partial: CSS foundations only | Medium | Medium |
| **Accessibility** | Full accessibility tree, focus navigation, contrast checks | Partial: basic support | Medium | Medium |
| **Import Formats** | Native .fig files, video, GIF | Partial: .x, Sketch, Figma JSON, PNG | High | Medium |

### 🟢 P2 — Platform/Collaboration (Differentiators, Not Blockers)

| Feature | Figma Capability | X-Native Status | Effort | Impact |
|---------|------------------|-----------------|--------|--------|
| **Multiplayer** | Real-time collaboration, presence cursors | Missing | Very High | High (but not for solo use) |
| **Comments** | Threaded comments, annotations | Missing | Medium | Medium |
| **Plugins** | Sandboxed plugin runtime, distribution | Missing | Very High | High (ecosystem play) |
| **Version History** | Beyond local checkpoints | Missing | High | Medium |
| **Native .fig Import** | Proprietary format parsing | Missing | Very High | Medium |

---

## Detailed Implementation Roadmap

### Phase 1: Close Daily Design Loop (8-12 weeks)

#### Week 1-2: Text Mouse Selection
- [ ] Implement shaped-glyph hit-testing for mouse→text-range mapping
- [ ] Add double-click word select, triple-click line select
- [ ] Test IME input with macOS/i18n keyboards
- **Acceptance:** Can select/edit text ranges with mouse like Figma

#### Week 3-4: Auto Layout Completion
- [ ] Add wrap behavior for horizontal/vertical layouts
- [ ] Implement min/max sizing constraints
- [ ] Add baseline alignment option
- [ ] Support absolute-positioned children within auto-layout frames
- [ ] Handle nested auto-layout solving correctly
- **Acceptance:** All Figma auto-layout test cases pass

#### Week 5-6: Component Properties
- [ ] Boolean property type with UI toggle
- [ ] Text property type with editable field
- [ ] Instance-swap property with component picker
- [ ] Preferred values configuration
- [ ] Nested instance property exposure
- [ ] Reset-all overrides action
- **Acceptance:** Can author and consume component properties like Figma

#### Week 7-8: Clipboard & Paste
- [ ] Implement paste-in-place (same coordinates)
- [ ] Implement paste-over-selection (replace selected objects)
- [ ] Cross-app clipboard serialization (SVG/PNG metadata)
- [ ] Fix Alt-drag duplicate edge cases
- **Acceptance:** Paste behaves identically to Figma in all scenarios

#### Week 9-10: Corner Radius UX
- [ ] Add on-canvas radius drag handles (uniform)
- [ ] Add per-corner handle visibility toggle
- [ ] Link independent corner fields to canvas handles
- **Acceptance:** Can adjust all corner radii via canvas or inspector

#### Week 11-12: Export Completeness
- [ ] Add JPG export with quality slider
- [ ] Implement slice-based export regions
- [ ] Add multi-scale export (@1x, @2x, @3x presets)
- [ ] Batch export multiple nodes/pages
- [ ] Add filename suffix customization
- **Acceptance:** All Figma export workflows supported

---

### Phase 2: Professional Workflows (8-10 weeks)

#### Week 13-15: Advanced Prototyping
- [ ] Overlay interactions (open/close with background blur)
- [ ] Scroll-to actions
- [ ] Hover, press, drag, delay triggers
- [ ] Fixed/sticky layer behavior in scrolling frames
- [ ] Flow starting points
- [ ] Device frame presets
- **Acceptance:** Can prototype multi-screen flows with all interaction types

#### Week 16-17: Variable Management
- [ ] Variable scopes (frame/component/global)
- [ ] Bulk variable editor UI
- [ ] Rename/delete with reference safety
- [ ] Missing library variable resolution UI
- [ ] Variable aliasing improvements
- **Acceptance:** Can manage complex design token systems

#### Week 18-19: Vector Networks
- [ ] Branching vector paths (multiple edges per vertex)
- [ ] Pencil tool (freehand drawing with simplification)
- [ ] Width profiles (pressure-sensitive strokes)
- [ ] Brush tool with texture options
- **Acceptance:** Can create and edit complex vector illustrations

#### Week 20-21: Dev Mode & Handoff
- [ ] Full inspect panel with measurements
- [ ] CSS copy (complete property set)
- [ ] Swift/UIKit export
- [ ] Jetpack Compose export
- [ ] Asset extraction panel
- [ ] Token value display (variables → code)
- **Acceptance:** Developers can extract all needed assets/code

#### Week 22: Accessibility
- [ ] Full accessibility tree generation
- [ ] Keyboard focus navigation
- [ ] Focus visibility indicators
- [ ] Contrast checking tools
- [ ] Screen reader label assignment
- **Acceptance:** Meets WCAG 2.1 AA for tool itself + export accessible designs

---

### Phase 3: Platform & Polish (10-14 weeks)

#### Week 23-26: Mac-Native Trust
- [ ] Universal binary (Apple Silicon + Intel)
- [ ] Code signing and notarization
- [ ] Native file dialogs (open/save/import/export)
- [ ] Trackpad gestures (magnify, inertial pan)
- [ ] Retina scaling across displays
- [ ] Unsaved-close confirmation
- [ ] Crash recovery presentation
- [ ] App menu integration (Help, About, Preferences)
- **Acceptance:** Feels like a native Mac app, passes Gatekeeper

#### Week 27-30: Collaboration Foundations
- [ ] Comment threads with anchoring
- [ ] Multiplayer presence (cursors, selection highlights)
- [ ] CRDT or OT document sync engine
- [ ] Conflict resolution UI
- [ ] Shared history timeline
- **Acceptance:** 2+ users can collaborate in real-time

#### Week 31-34: Plugin System
- [ ] WASM sandbox runtime
- [ ] Plugin API surface (document, selection, UI)
- [ ] Permission model
- [ ] Plugin manager UI
- [ ] Distribution mechanism
- **Acceptance:** Can run third-party plugins safely

#### Week 35-36: Performance Hardening
- [ ] 100K+ node scene performance (<16ms frame time)
- [ ] Incremental re-encoding (dirty subtrees only)
- [ ] Tiled rendering cache for static content
- [ ] Background image decode/font loading
- [ ] Memory profiling and leak fixes
- **Acceptance:** Smooth performance at scale

---

## Definition of "100% Parity"

**Do NOT claim 100% based on feature checklists.** Instead:

### Parity Criteria v1.0

1. **Workflow Corpus:** 100 real-world design scenarios documented
   - Creation, editing, layout, components, variables, prototyping
   - Import/export, recovery, performance stress tests

2. **Behavioral Matching:** For each scenario:
   - ✅ Same final document state
   - ✅ Same rendered output (≤2% RMSE visual difference)
   - ✅ Same undo/redo behavior
   - ✅ Similar latency (<20ms for common ops)

3. **Platform Acceptance:**
   - ✅ Zero data-loss bugs
   - ✅ 100% pass on P0 scenarios
   - ✅ Successful recovery after forced termination
   - ✅ Keyboard-only completion of core flows

4. **User Validation:**
   - ✅ 10+ professional designers complete daily work without Figma
   - ✅ No workflow blockers reported in 2-week trial

---

## What NOT to Build (Avoid Figma Traps)

❌ **Don't copy Figma's UI exactly** — Legal risk + loses X-Native identity
- Use graphite/violet theme, not Figma blue
- Fewer permanent borders, progressive disclosure
- Native Mac language (Command/Option glyphs)

❌ **Don't chase every Figma feature** — Focus on core design workflows
- Video/GIF import (low value, high complexity)
- Advanced animation timelines (different product category)
- Whiteboarding tools (separate market)

❌ **Don't build collaboration before single-player is perfect**
- Multiplayer amplifies bugs, doesn't fix them
- Get the daily loop flawless first

---

## Success Metrics

### Leading Indicators (Track Weekly)
- [ ] P0 scenario pass rate (target: 100%)
- [ ] Visual regression RMSE (target: <0.05)
- [ ] Frame time p95 (target: <16ms)
- [ ] Undo chain depth tested (target: 100+ ops)

### Lagging Indicators (Track Monthly)
- [ ] Designer retention (target: 80% week-2 retention)
- [ ] Figma replacement rate (target: 50% of tasks)
- [ ] Bug report velocity (target: decreasing trend)
- [ ] Feature request themes (identify gaps)

---

## Immediate Next Steps (This Week)

1. **Audit current P0 gaps** — Run through the 7 P0 features above, document exact failures
2. **Build scenario corpus** — Start with 20 critical workflows, expand to 100
3. **Set up visual regression CI** — Automated RMSE comparison vs Figma exports
4. **Prioritize text selection** — Highest impact, blocks most text workflows
5. **Freeze new features** — No new capabilities until P0 gaps closed

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Text shaping complexity underestimated | High | High | Partner with `parley`/`cosmic-text` teams, don't build from scratch |
| Auto-layout solver becomes constraint nightmare | Medium | High | Study existing implementations (Yoga, Cassowary), don't invent new algorithm |
| Component properties scope creep | High | Medium | Define MVP property types, defer advanced features to v1.1 |
| Performance degrades with features | Medium | High | Budget 20% of each sprint for perf debt, benchmark continuously |
| Legal issues from Figma similarity | Low | Very High | Audit UI for trademark/copyright issues early, differentiate visually |

---

## Conclusion

**Realistic Timeline:** 6-9 months to 100% workflow parity for single-user professional design work.

**Key Insight:** Don't chase "all features." Chase "all workflows." A designer should be able to open X-Native and complete their daily work without thinking about missing features.

**X-Native Advantages to Preserve:**
- ✅ Offline-first ownership
- ✅ Native performance
- ✅ Portable .x files
- ✅ No subscription lock-in

These differentiators matter more than matching Figma feature-for-feature. Build the best **offline-native design tool**, not the best **Figma clone**.
