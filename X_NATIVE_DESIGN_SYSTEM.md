# X-Native Professional UI Design System

## 🎨 Brand Identity: Graphite & Violet

### Core Philosophy
X-Native is **NOT** a Figma clone. We are a professional, native-first design tool with our own distinct identity inspired by modern tools like Framer, while maintaining professional ergonomics.

### Visual Language Principles
1. **Distinctive Color Palette**: Deep graphite surfaces with violet accent (#7C5CFC)
2. **Professional Spacing**: Consistent 8px grid system
3. **Modern Typography**: Inter for UI, JetBrains Mono for code/values
4. **Subtle Depth**: Layered surfaces with minimal borders
5. **Focus on Content**: Canvas-first design, minimal chrome distraction

---

## 🎨 Color System

### Primary Palette
```rust
// Backgrounds (Graphite Series)
C_BG_PRIMARY:   #0D0E12  // Main app background
C_BG_SECONDARY: #141519  // Panel backgrounds  
C_BG_TERTIARY:  #1B1D23  // Elevated surfaces
C_BG_FIELD:     #202229  // Input fields

// Semantic Colors
C_ACCENT:       #7C5CFC  // Primary violet (brand color)
C_ACCENT_HOVER: #8B6EFF  // Hover state
C_ACCENT_ACTIVE:#6A4CE0  // Active/pressed

// Text Hierarchy
C_TEXT_PRIMARY:   #F2F3F7  // Primary text
C_TEXT_SECONDARY: #9A9EAA  // Secondary text
C_TEXT_DISABLED:  #6B6F7A  // Disabled text

// Functional Colors
C_SUCCESS: #10B981  // Green - success states
C_WARNING: #F59E0B  // Amber - warnings
C_ERROR:   #EF4444  // Red - errors
C_INFO:    #3B82F6  // Blue - information

// Canvas & Interaction
C_CANVAS:      #22242A  // Canvas background
C_HOVER:       #2A2D36  // Hover states
C_SELECTION:   rgba(124, 92, 252, 0.2)  // Selection overlay
C_FOCUS_RING:  #A996FF  // Accessibility focus ring
C_BORDER:      #2D3039  // Subtle dividers
```

### Contrast Requirements
- All text must meet WCAG AA minimum (4.5:1 for normal text)
- Focus indicators must be highly visible (3:1 minimum)
- No reliance on color alone for information

---

## 📐 Layout System

### Spacing Scale (8px Grid)
```
4px   - Tight spacing (icon padding)
8px   - Base unit (component padding)
12px  - Comfortable spacing
16px  - Section spacing
24px  - Panel margins
32px  - Large gaps
48px  - Major sections
```

### Panel Dimensions
```rust
// Left Panel (Layers/Assets)
LAYERS_PANEL_W: 252px

// Right Panel (Inspector)
INSPECTOR_W: 280px  // Slightly wider for better readability

// Top Bar
TOP_BAR_H: 48px     // Streamlined from 72px

// Bottom Bar  
BOTTOM_BAR_H: 32px  // Minimal status bar

// Tab Height
TAB_H: 36px         // Comfortable click target
```

### Corner Radius System
```
2px  - Small elements (buttons, inputs)
4px  - Medium elements (cards, panels)
8px  - Large elements (modals, overlays)
12px - Extra large (floating HUDs)
```

---

## ✨ Typography

### Font Families
```rust
UI_FONT: "Inter", -apple-system, BlinkMacSystemFont, sans-serif
CODE_FONT: "JetBrains Mono", "Fira Code", monospace
CANVAS_FONT: "Inter", system-ui  // For design text preview
```

### Type Scale
```
11px - Tiny (labels, hints)
12px - Small (secondary text)
13px - Base (body text)
14px - Medium (emphasis)
16px - Large (section headers)
18px - XL (panel titles)
24px - 2XL (modal titles)
```

### Font Weights
```
400 - Regular (body text)
500 - Medium (buttons, labels)
600 - Semi-bold (headers)
700 - Bold (emphasis)
```

---

## 🎯 Component Guidelines

### Buttons
```rust
// Primary Button
background: C_ACCENT
text: WHITE
padding: 8px 16px
radius: 6px
height: 32px

// Secondary Button  
background: C_BG_TERTIARY
text: C_TEXT_PRIMARY
border: 1px solid C_BORDER
padding: 8px 16px
radius: 6px
height: 32px

// Ghost Button
background: transparent
text: C_TEXT_PRIMARY
hover: C_HOVER
padding: 8px 12px
radius: 6px
```

### Input Fields
```rust
background: C_BG_FIELD
border: 1px solid C_BORDER
border_radius: 6px
height: 32px
padding: 6px 10px
font: 13px JetBrains Mono
focus_border: C_ACCENT
focus_ring: 2px solid rgba(124, 92, 252, 0.4)
```

### Panels
```rust
background: C_BG_SECONDARY
border: 1px solid C_BORDER
border_radius: 8px
shadow: 0 4px 12px rgba(0, 0, 0, 0.3)
```

### Tabs
```rust
inactive_bg: transparent
inactive_text: C_TEXT_SECONDARY
active_bg: C_BG_TERTIARY
active_text: C_TEXT_PRIMARY
indicator: 2px solid C_ACCENT (bottom)
height: 36px
padding: 0 16px
```

---

## 🚀 Key UI Innovations

### 1. Adaptive Context System
Panels adapt to current task:
- **Layout Mode**: Auto-layout controls prominent
- **Vector Mode**: Pen tool options visible
- **Text Mode**: Typography panel expands
- **Component Mode**: Properties show variants
- **Focus Mode**: All chrome hidden

### 2. Command Palette (Cmd+K)
Spotlight-style search for:
- Layers and components
- Actions and tools
- Settings and preferences
- Keyboard shortcuts

### 3. Floating Property HUD
Inline editing without losing canvas focus:
- Shows common properties for selection
- Drag-to-scrub numeric values
- Quick color picker
- Dismissable with Esc

### 4. Spatial Navigator
Mini-map showing:
- Element positions spatially
- Color-coded by type
- Click to zoom/select
- Shows overlap visually

### 5. Focus Mode (Cmd+Shift+F)
Distraction-free workflow:
- Hide all panels
- Canvas fills 95% screen
- Only essential tools visible
- Notifications suppressed

---

## ♿ Accessibility Standards

### Keyboard Navigation
```
Tab/Shift+Tab  - Navigate widgets
Enter/Space    - Activate
Arrow Keys     - Navigate within widgets
Esc            - Close/Cancel
Cmd+K          - Command palette
Cmd+/          - Shortcuts overlay
```

### Screen Reader Support
- All interactive elements have semantic roles
- Logical focus order
- Live regions for dynamic updates
- Descriptive labels for icons

### High Contrast Mode
Toggle switches to WCAG AAA compliant theme:
- Pure black background (#000000)
- White text (#FFFFFF)  
- Yellow focus rings (#FFFF00)
- No color-only information

### Reduced Motion
Respects OS setting:
- Disables non-essential animations
- Instant transitions option
- No auto-playing content

---

## 🎭 Motion Design

### Animation Durations
```
60ms  - Micro-interactions (hover states)
80ms  - Selection feedback
120ms - Small transitions (toggle, checkbox)
150ms - Panel expand/collapse
200ms - Modal transitions
250ms - Page transitions
```

### Easing Functions
```rust
ease_out: cubic-bezier(0.16, 1, 0.3, 1)    // Most UI motions
ease_in_out: cubic-bezier(0.65, 0, 0.35, 1) // Balanced transitions
spring: cubic-bezier(0.34, 1.56, 0.64, 1)  // Playful interactions
```

### Animation Policy
```rust
Full      - All animations (default)
Reduced   - Essential motion only
None      - Instant changes (accessibility)
```

---

## 📊 Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| First Paint | <100ms | App launch to UI ready |
| Panel Response | <16ms | Click to visual feedback |
| Canvas FPS | 120+ | With 1000+ elements |
| Search Results | <50ms | Cmd+K typing latency |
| Theme Switch | <100ms | Full theme transition |

---

## 🛠 Implementation Checklist

### Phase 1: Foundation ✅
- [x] Define color system
- [x] Establish spacing scale
- [x] Set typography guidelines
- [ ] Implement theme provider
- [ ] Create component primitives

### Phase 2: Core Components
- [ ] Update button styles
- [ ] Redesign input fields
- [ ] Modernize panels
- [ ] Improve tabs
- [ ] Add tooltips

### Phase 3: Layout Improvements
- [ ] Streamline top bar (48px)
- [ ] Widen inspector (280px)
- [ ] Add command palette
- [ ] Implement focus mode
- [ ] Create floating HUD

### Phase 4: Polish
- [ ] Add micro-animations
- [ ] Implement keyboard nav overlay
- [ ] Add performance metrics display
- [ ] Accessibility audit
- [ ] User testing iterations

---

## 🎯 Success Metrics

### Usability Goals
- Reduce clicks per common task from 3-5 to 1-2
- Achieve basic proficiency in <1 day (vs 2-3 for Figma)
- Score 95+ on accessibility audits
- Maintain 120+ FPS with complex documents

### User Satisfaction
- Net Promoter Score >50
- Task completion rate >95%
- Error rate <2%
- Time-on-task reduction >30%

---

## 💡 Differentiators from Figma

1. **Native Performance**: Rust + GPU = butter smooth at any scale
2. **True Offline**: Full functionality without internet
3. **Local-First**: Your data stays yours
4. **Open Format**: Documented .x files, backward compatible
5. **Better Accessibility**: Built-in from day one, not afterthought
6. **Custom Themes**: User-created themes easily shareable
7. **Spatial Navigation**: See your design structure visually
8. **Focus Modes**: Distraction-free workflows built-in

---

*Last Updated: 2025*
*Version: 1.0*
*X-Native Design Team*
