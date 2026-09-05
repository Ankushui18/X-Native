# X-Native FINAL v45 — HTML Source of Truth → Rust Native

## HTML Source of Truth
- Editor: v45-final-editor-28px.html — 28x28 logo rect #1A1A1A + green #1BCB55 + white X, top tabs flush + next to X expanding right, left icons by type Group=Grid Rectangle=Square Text=Type Ellipse=Circle Vector=PenTool, right Size+Position Frame dropdown, Auto Layout 84x84 dark no 09 + dark icon, Guides Square 16 no dropdown, Export collapsed + toggles PNG 1x Suffix EXPORT 1 ELEMENT h-7 small
- Dashboard: dashboard-v2.html — top 40px #111111 line #1F1F1F logo 28x28, search 480x32 field #1A1A1A + ⌘K, new file + avatar S #FFEB3B, left 260px DRAFTS green dot Personal Home/Recents/Starred/Trash TEAMS L #5B7CFF D #FF7A45, main #060606 welcome Sahil 4 cards 88px file grid 3 cols 180px cards

## Rust Files — Converted from HTML — No figma/framer naming
- theme.rs — full design system tokens C_BG #090909 C_CANVAS #060606 C_PANEL #111111 C_FIELD #1A1A1A C_FIELD_2 #222222 C_LINE #1F1F1F C_LINE_2 #2A2A2A C_TEXT #FFFFFF C_MUTED #999999 C_DIM #777777 C_FAINT #3A3A3A C_ACCENT_GREEN #1BCB55 C_AVATAR #FFEB3B C_TEAM_L #5B7CFF C_TEAM_D #FF7A45 C_DRAFT_DOT #2ECC71 C_MD_BADGE #519ABA
- shell.rs — FINAL v45 editor + dashboard — paint_title_final 28x28 logo, tabs flush + next to X, left_w/right_w resizable 200-480/240-480, left icons by type, right combined Size+Position Frame, Auto Layout 84x84 dark, Guides no dropdown, Export collapsed
- chrome.rs — Native chrome — same tokens, top bar 36px #111111 line #1F1F1F, left 280px resizable, right 340px resizable, canvas #060606 frame 375x420 white, bottom toolbar 260x36 #1A1A1A/90 backdrop-blur, export collapsed toggle
- icons.rs — Lucide stroke 1.75px rounded — same library as HTML — Grid/Square/Type/Circle/PenTool/Board/Search/Plus/X etc
- state.rs — left_w right_w resizing_left/right export_expanded guides_expanded doc_name editable
- run.rs — panel resize drag 6px hover ↔, file rename below Draft, Frame dropdown cycles presets default 375x812 auto converts, export + toggles
- paint.rs — primitive chrome painting Vello Scene — fill_rect fill_rrect stroke_rect hline vline measure label_bar

## Build
cargo build -p x-designer --bin x_native_app --release -j 1
Binary: 16M target/release/x_native_app

## Zip
X-NATIVE-FINAL-BUILD.zip 5M — binary + HTML source + RS files + design system

## GitHub Push & Test
git remote -v # https://github.com/Ankushui18/X-Native.git
git add .
git commit -m "FINAL v45 — HTML source to RS chrome.rs shell.rs theme.rs icons.rs state.rs run.rs — 28x28 logo, resizable panels, export collapsed, guides no dropdown, full design system Lucide same lib, no figma/framer naming"
git push origin main
cargo run -p x-designer --bin x_native_app --release
