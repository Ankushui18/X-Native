# Screen / layout specification — Phase 1

## Home

- Centered card on Graphite base
- Actions: Blank canvas, Desktop, Mobile, Open file
- Click → Editor with empty page

## Editor chrome

```
┌─────────────────────────────────────────────────────────────┐
│ Title 36px   mark · document · zoom chip                    │
├────┬──────────┬─────────────────────────────┬───────────────┤
│Tool│ Left 240 │         Canvas (flex)       │ Inspector 260 │
│48  │ Pages    │     artboard + content      │ Page or props │
│    │ Layers   │                             │               │
├────┴──────────┴─────────────────────────────┴───────────────┤
│ Status 24px · message · ⌘K                                  │
└─────────────────────────────────────────────────────────────┘
```

## Interaction rules

- Page root is never a layer row
- Empty page → “No layers on this page”
- Empty selection → Page properties only
- Space / H → Hand tool
- ⌘/Ctrl+K → command palette
- F on canvas → place white frame (Phase 1 create path)

## Not in Phase 1

- Full property editors, Auto Layout UX, assets browsers, prototype wires, export panel
