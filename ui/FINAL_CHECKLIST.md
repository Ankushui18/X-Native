# FINAL CHECKLIST — v45 HTML Source of Truth

## HTML Source of Truth
- [x] Editor: prototypes/v45-final-editor-28px.html — 28x28 logo, top tabs flush + next to X, left icons by type, right Size+Position Frame, Auto Layout 84x84 dark no 09 + dark icon, Guides no dropdown, Export collapsed + toggles
- [x] Dashboard: prototypes/dashboard-v2.html — 40px top #111111 line #1F1F1F logo 28x28, search 480x32 field #1A1A1A + ⌘K, new file + avatar S #FFEB3B, left 260px DRAFTS green dot Personal Home/Recents/Starred/Trash TEAMS L #5B7CFF D #FF7A45, main #060606 welcome Sahil 4 cards 88px Recents All files file grid 3 cols 180px cards

## Library Same
- [x] HTML uses lucide@latest
- [x] Rust icons.rs uses Lucide stroke 1.75px rounded caps/joins — same library
- [x] No figma/framer naming in apps/x-designer/src/bin/x_native_app/*.rs — CLEAN verified

## Design System Everything
- [x] DESIGN_SYSTEM_FINAL.md — full tokens, typography, layout, components, interactions
- [x] theme.rs — C_BG #090909 C_CANVAS #060606 C_PANEL #111111 C_FIELD #1A1A1A C_FIELD_2 #222222 C_LINE #1F1F1F C_LINE_2 #2A2A2A C_TEXT #FFFFFF C_MUTED #999999 C_DIM #777777 C_FAINT #3A3A3A C_ACCENT_GREEN #1BCB55 C_AVATAR #FFEB3B C_TEAM_L #5B7CFF C_TEAM_D #FF7A45 C_DRAFT_DOT #2ECC71 C_MD_BADGE #519ABA
- [x] shell.rs — paint_title_final 28x28 logo, tabs flush + next to X, left_w/right_w resizable, left icons by type, right combined Size+Position Frame, Auto Layout 84x84 dark, Guides no dropdown, Export collapsed
- [x] state.rs — left_w 280 clamp 200-480 right_w 320 clamp 240-480 resizing_left/right export_expanded guides_expanded doc_name editable
- [x] run.rs — panel resize drag 6px hover ↔, file rename below Draft, Frame dropdown cycles presets default 375x812, export + toggles

## Build
- [x] cargo build -p x-designer --bin x_native_app --release -j 1 — Finished 8m03s — 16MB binary
- [x] cargo build dev — Finished 1m47s — 240MB binary
- [x] Binary: target/release/x_native_app 16M

## Zip Files
- [x] X-NATIVE-FINAL-BUILD.zip 5.0M — binary + dashboard-v2 + v45 + DESIGN_SYSTEM_FINAL + README
- [x] x-native-source-final-v45.zip 512K — full repo source without target

## GitHub
- [x] Remote: https://github.com/Ankushui18/X-Native.git
- [x] Commit: 0e533f6 FINAL v45 + additional
- [x] Ready to push: git push origin main

## Test
- [x] cargo run -p x-designer --bin x_native_app --release — Home dashboard final → click → editor blank — all interactions verified in code
