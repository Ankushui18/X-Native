//! X-Native Shell — FINAL v45 — Native Design System — HTML Source of Truth
//! Editor source: prototypes/v45-final-editor-28px.html — 28x28 logo, top tabs flush, + next to X, left icons by type, right Size+Position Frame dropdown, Auto Layout 84x84 dark no 09 + dark icon, Guides no dropdown, Export collapsed + toggles, Typography full, Fill above Stroke
//! Dashboard source: prototypes/dashboard-v2.html — dark palette #090909 #111111 #1A1A1A #1F1F1F #2A2A2A, 260px sidebar, 40px top, 3-col grid 180px cards
//! Library: Lucide icons stroke 1.75px rounded — same as HTML — no external product naming
//! Design system: DESIGN_SYSTEM_FINAL.md

use vello::kurbo::Rect;
use vello::peniko::Color;
use vello::Scene;

use crate::icons::{draw_icon, draw_dropdown_chevron, Icon};
use crate::paint::{fill_rect, fill_rrect, hline, label_bar, stroke_rect, vline};
use crate::state::{AppState, InspectorTab, LeftTab, Screen, Tool};
use crate::theme::*;

pub struct Regions {
    pub title: Rect,
    pub left: Rect,
    pub right: Rect,
    pub canvas: Rect,
    pub status: Rect,
}

pub fn regions_for(app: &AppState) -> Regions {
    Regions {
        title: Rect::new(0.0, 0.0, app.win_w, TITLE_H),
        left: Rect::new(0.0, TITLE_H, app.left_w, app.win_h - STATUS_H),
        right: Rect::new(app.win_w - app.right_w, TITLE_H, app.win_w, app.win_h - STATUS_H),
        canvas: Rect::new(app.left_w, TITLE_H, app.win_w - app.right_w, app.win_h - STATUS_H),
        status: Rect::new(0.0, app.win_h - STATUS_H, app.win_w, app.win_h),
    }
}
pub fn regions(_w: f64, _h: f64) -> Regions {
    // legacy shim — callers that don't have app should use regions_for
    Regions {
        title: Rect::new(0.0, 0.0, _w, TITLE_H),
        left: Rect::new(0.0, TITLE_H, LEFT_W, _h - STATUS_H),
        right: Rect::new(_w - RIGHT_W, TITLE_H, _w, _h - STATUS_H),
        canvas: Rect::new(LEFT_W, TITLE_H, _w - RIGHT_W, _h - STATUS_H),
        status: Rect::new(0.0, _h - STATUS_H, _w, _h),
    }
}

pub fn paint_shell(scene: &mut Scene, app: &AppState) {
    let r = regions_for(app);
    fill_rect(scene, Rect::new(0.0, 0.0, app.win_w, app.win_h), C_BG);
    match app.screen {
        Screen::Home => paint_home(scene, app),
        Screen::Editor => {
            paint_title_final(scene, app, &r);
            paint_left_final(scene, app, &r);
            paint_canvas_final(scene, app, &r);
            paint_right_final(scene, app, &r);
            paint_status(scene, app, &r);
            if app.command_open {
                paint_command_palette(scene, app);
            }
        }
    }
}

// === TOP BAR — FINAL 28x28 logo, tabs flush no floating, + next to X — HTML source ===
fn paint_title_final(scene: &mut Scene, app: &AppState, _r: &Regions) {
    fill_rect(scene, Rect::new(0.0, 0.0, app.win_w, TITLE_H), C_PANEL);
    hline(scene, 0.0, app.win_w, TITLE_H, C_LINE);
    // Logo 28x28 — custom green #1BCB55 + white — exact from v45 HTML
    let lx = 6.0;
    let ly = (TITLE_H - 28.0) / 2.0;
    fill_rrect(scene, Rect::new(lx, ly, lx+28.0, ly+28.0), 4.0, C_FIELD);
    fill_rrect(scene, Rect::new(lx+14.0, ly+4.0, lx+24.0, ly+20.0), 2.0, C_ACCENT_GREEN);
    label_bar(scene, "X", lx+7.0, ly+7.0, 12.0, C_TEXT);
    // File tabs flush — no top/bottom gap, h = TITLE_H
    let mut tx = 44.0;
    let tabs = [
        ("Untitled", false, false),
        ("DESIGN_SYSTEM.md", true, true),
        ("Liquor App", false, false),
    ];
    for (name, active, is_md) in tabs.iter() {
        let tw = 132.0;
        if *active {
            fill_rect(scene, Rect::new(tx, 0.0, tx+tw, TITLE_H), C_FIELD);
            fill_rect(scene, Rect::new(tx, 0.0, tx+tw, 1.5), C_TEXT);
        } else {
            fill_rect(scene, Rect::new(tx, 0.0, tx+tw, TITLE_H), C_PANEL);
        }
        vline(scene, tx+tw, 0.0, TITLE_H, C_LINE);
        if *is_md {
            fill_rrect(scene, Rect::new(tx+8.0, 13.0, tx+20.0, 25.0), 2.0, C_MD_BADGE);
            label_bar(scene, "✦", tx+10.0, 13.0, 8.0, C_TEXT);
            label_bar(scene, name, tx+26.0, 12.0, 10.0, if *active {C_TEXT} else {C_DIM});
        } else {
            draw_icon(scene, Icon::File, tx+8.0, 12.0, 12.0, C_DIM);
            label_bar(scene, name, tx+26.0, 12.0, 10.0, if *active {C_TEXT} else {C_DIM});
        }
        draw_icon(scene, Icon::X, tx+tw-18.0, 14.0, 10.0, C_DIM);
        tx += tw;
    }
    // + next to X — expanding tabs to right
    fill_rect(scene, Rect::new(tx, 0.0, tx+32.0, TITLE_H), C_PANEL);
    draw_icon(scene, Icon::Plus, tx+8.0, 12.0, 14.0, C_DIM);
    // Right profile S 12% — unified with tabs, no border between
    let prx = app.win_w - 120.0;
    fill_rrect(scene, Rect::new(prx, 8.0, prx+24.0, 32.0), 12.0, C_AVATAR);
    label_bar(scene, "S", prx+8.0, 12.0, 10.0, Color::from_rgb8(0x00,0x00,0x00));
    label_bar(scene, "12%", prx+32.0, 12.0, 10.0, C_MUTED);
}

// === LEFT — FINAL — icons by element type — resizable 200-480 ===
fn paint_left_final(scene: &mut Scene, app: &AppState, _r: &Regions) {
    let lw = app.left_w;
    fill_rect(scene, Rect::new(0.0, TITLE_H, lw, app.win_h-STATUS_H), C_PANEL);
    vline(scene, lw, TITLE_H, app.win_h-STATUS_H, C_LINE);
    let mut y = TITLE_H + 8.0;
    // DRAFTS
    draw_icon(scene, Icon::Drafts, 12.0, y+2.0, 14.0, C_DIM);
    label_bar(scene, "DRAFTS", 32.0, y+2.0, 9.0, C_DIM);
    draw_icon(scene, Icon::More, lw-20.0, y+2.0, 12.0, C_DIM);
    y += 18.0;
    // File name directly below Draft — editable on hover/click — HTML truth
    fill_rrect(scene, Rect::new(8.0, y-2.0, 12.0, y+2.0), 2.0, C_DRAFT_DOT);
    label_bar(scene, &app.doc_name, 20.0, y, 11.0, C_TEXT);
    // hover underline hint if inspector_edit is DocName
    if let Some(ed) = &app.inspector_edit {
        if ed.field == crate::state::InspectorField::DocName {
            hline(scene, 20.0, 20.0+ (ed.buffer.len() as f64*6.0).min(lw-30.0), y+14.0, C_TEXT);
        }
    }
    y += 22.0;
    // Pill tabs LAYERS/ASSETS/TOKENS — active #222222 border #2A2A2A
    fill_rrect(scene, Rect::new(8.0, y, lw-8.0, y+30.0), 8.0, C_BG);
    stroke_rect(scene, Rect::new(8.0, y, lw-8.0, y+30.0), C_LINE, 1.0);
    let tab_w = (lw - 20.0) / 3.0;
    // LAYERS active
    fill_rrect(scene, Rect::new(10.0, y+2.0, 10.0+tab_w, y+28.0), 6.0, C_FIELD_2);
    stroke_rect(scene, Rect::new(10.0, y+2.0, 10.0+tab_w, y+28.0), C_LINE_2, 1.0);
    label_bar(scene, "LAYERS", 18.0, y+9.0, 9.0, C_TEXT);
    label_bar(scene, "ASSETS", 18.0+tab_w, y+9.0, 9.0, C_DIM);
    label_bar(scene, "TOKENS", 18.0+tab_w*2.0, y+9.0, 9.0, C_DIM);
    y += 40.0;
    label_bar(scene, "PAGES", 12.0, y, 9.0, C_DIM);
    draw_icon(scene, Icon::Plus, lw-20.0, y, 12.0, C_DIM);
    y += 18.0;
    fill_rrect(scene, Rect::new(8.0, y, lw-8.0, y+28.0), 6.0, C_FIELD);
    stroke_rect(scene, Rect::new(8.0, y, lw-8.0, y+28.0), C_LINE_2, 1.0);
    draw_icon(scene, Icon::File, 14.0, y+7.0, 12.0, C_TEXT);
    label_bar(scene, "Page 3", 32.0, y+8.0, 11.0, C_TEXT);
    y += 36.0;
    hline(scene, 0.0, lw, y, C_LINE);
    y += 8.0;
    label_bar(scene, "PAGE 3", 12.0, y+2.0, 9.0, C_DIM);
    draw_icon(scene, Icon::Search, lw-20.0, y+2.0, 12.0, C_DIM);
    y += 20.0;

    struct TreeItem { name: &'static str, icon: Icon, indent: usize, expanded: Option<bool> }
    let items = [
        TreeItem { name: "Board", icon: Icon::Board, indent: 0, expanded: None },
        TreeItem { name: "order-details", icon: Icon::Grid, indent: 0, expanded: Some(false) }, // group -> grid
        TreeItem { name: "Rectangle 12", icon: Icon::Square, indent: 0, expanded: None }, // rectangle -> square
        TreeItem { name: "payment-methods", icon: Icon::Grid, indent: 0, expanded: Some(true) },
        TreeItem { name: "pay-row", icon: Icon::Board, indent: 1, expanded: Some(false) },
        TreeItem { name: "section-header", icon: Icon::Type, indent: 2, expanded: None },
        TreeItem { name: "Ellipse 3", icon: Icon::Circle, indent: 2, expanded: None }, // ellipse -> circle
        TreeItem { name: "Vector", icon: Icon::PenTool, indent: 2, expanded: None }, // vector -> pen-tool
    ];
    for item in items.iter() {
        if y > app.win_h - STATUS_H - 20.0 { break; }
        let ix = 8.0 + item.indent as f64 * 16.0;
        if let Some(exp) = item.expanded {
            if exp {
                draw_icon(scene, Icon::ChevronDown, ix, y+6.0, 10.0, C_DIM);
            } else {
                draw_icon(scene, Icon::ChevronRight, ix, y+6.0, 10.0, C_DIM);
            }
            draw_icon(scene, item.icon, ix+14.0, y+5.0, 12.0, C_DIM);
            label_bar(scene, item.name, ix+30.0, y+6.0, 11.0, C_MUTED);
        } else {
            draw_icon(scene, item.icon, ix+14.0, y+5.0, 12.0, C_DIM);
            label_bar(scene, item.name, ix+30.0, y+6.0, 11.0, C_MUTED);
        }
        y += 22.0;
    }
}

// === CANVAS — final with bottom toolbar ===
fn paint_canvas_final(scene: &mut Scene, app: &AppState, r: &Regions) {
    fill_rect(scene, r.canvas, C_CANVAS);
    let aw = 520.0 * app.zoom;
    let ah = 340.0 * app.zoom;
    let ax = r.canvas.x0 + (r.canvas.width() - aw) * 0.5 + app.pan.0;
    let ay = r.canvas.y0 + (r.canvas.height() - ah) * 0.5 + app.pan.1;
    let board = Rect::new(ax, ay, ax+aw, ay+ah);
    fill_rrect(scene, board, 8.0, Color::from_rgb8(0xFF, 0xFF, 0xFF));
    // Bottom toolbar — Select active white, Frame not selected, pen added back
    let bar_w = 260.0;
    let bar_h = 36.0;
    let bar_x = r.canvas.x0 + (r.canvas.width() - bar_w) * 0.5;
    let bar_y = r.canvas.y1 - 50.0;
    fill_rrect(scene, Rect::new(bar_x, bar_y, bar_x+bar_w, bar_y+bar_h), 12.0, Color::from_rgba8(0x1A,0x1A,0x1A,0xDD));
    stroke_rect(scene, Rect::new(bar_x, bar_y, bar_x+bar_w, bar_y+bar_h), C_LINE_2, 1.0);
    let tools = [
        (Tool::Select, Icon::Cursor, true),
        (Tool::Frame, Icon::Board, false),
        (Tool::Text, Icon::Type, false),
        (Tool::Rectangle, Icon::Square, false),
        (Tool::Pen, Icon::PenTool, false),
        (Tool::Hand, Icon::Search, false),
    ];
    for (i, (t, icon, _)) in tools.iter().enumerate() {
        let bx = bar_x + 6.0 + i as f64 * 36.0;
        let is_active = *t == app.tool;
        if is_active {
            fill_rrect(scene, Rect::new(bx, bar_y+4.0, bx+28.0, bar_y+32.0), 7.0, C_TEXT);
            draw_icon(scene, *icon, bx+6.0, bar_y+10.0, 16.0, Color::from_rgb8(0x00,0x00,0x00));
        } else {
            draw_icon(scene, *icon, bx+6.0, bar_y+10.0, 16.0, C_DIM);
        }
    }
}

// === RIGHT — FINAL v45 — resizable 240-480 ===
fn paint_right_final(scene: &mut Scene, app: &AppState, _r: &Regions) {
    let rw = app.right_w;
    let rx = app.win_w - rw;
    fill_rect(scene, Rect::new(rx, TITLE_H, app.win_w, app.win_h-STATUS_H), C_PANEL);
    vline(scene, rx, TITLE_H, app.win_h-STATUS_H, C_LINE);

    // Unified profile + tabs — no border between
    let mut y = TITLE_H + 6.0;
    fill_rrect(scene, Rect::new(rx+8.0, y, rx+32.0, y+24.0), 12.0, Color::from_rgb8(0xFF,0xEB,0x3B));
    label_bar(scene, "S", rx+16.0, y+5.0, 10.0, Color::from_rgb8(0x00,0x00,0x00));
    label_bar(scene, "12%", rx+40.0, y+6.0, 10.0, C_MUTED);
    y += 32.0;
    // Pill tabs
    fill_rrect(scene, Rect::new(rx+8.0, y, rx+rw-8.0, y+30.0), 8.0, C_BG);
    stroke_rect(scene, Rect::new(rx+8.0, y, rx+rw-8.0, y+30.0), C_LINE, 1.0);
    let tab_w = (rw - 20.0) / 3.0;
    fill_rrect(scene, Rect::new(rx+10.0, y+2.0, rx+10.0+tab_w, y+28.0), 6.0, C_FIELD_2);
    stroke_rect(scene, Rect::new(rx+10.0, y+2.0, rx+10.0+tab_w, y+28.0), C_LINE_2, 1.0);
    label_bar(scene, "DESIGN", rx+18.0, y+9.0, 9.0, C_TEXT);
    label_bar(scene, "PROTOTYPE", rx+14.0+tab_w, y+9.0, 9.0, C_DIM);
    label_bar(scene, "INSPECT", rx+22.0+tab_w*2.0, y+9.0, 9.0, C_DIM);
    y += 38.0;
    hline(scene, rx, rx+rw, y, C_LINE);
    y += 8.0;

    // Combined Size+Position — no heading, Frame replaces Normal
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+120.0, y+28.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+PAD, y, rx+120.0, y+28.0), C_LINE, 1.0);
    label_bar(scene, "Frame", rx+PAD+10.0, y+8.0, 11.0, C_TEXT);
    draw_dropdown_chevron(scene, rx+100.0, y+10.0, C_DIM);
    fill_rrect(scene, Rect::new(rx+128.0, y, rx+196.0, y+28.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+128.0, y, rx+196.0, y+28.0), C_LINE, 1.0);
    label_bar(scene, "100%", rx+144.0, y+8.0, 11.0, C_TEXT);
    draw_icon(scene, Icon::Eye, rx+208.0, y+7.0, 12.0, C_DIM);
    draw_icon(scene, Icon::Lock, rx+rw-28.0, y+7.0, 12.0, C_DIM);
    y += 36.0;
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+rw/2.0-4.0, y+28.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+PAD, y, rx+rw/2.0-4.0, y+28.0), C_LINE, 1.0);
    label_bar(scene, "W 375", rx+PAD+8.0, y+8.0, 10.0, C_TEXT);
    fill_rrect(scene, Rect::new(rx+rw/2.0+4.0, y, rx+rw-36.0, y+28.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+rw/2.0+4.0, y, rx+rw-36.0, y+28.0), C_LINE, 1.0);
    label_bar(scene, "H 420", rx+rw/2.0+12.0, y+8.0, 10.0, C_TEXT);
    y += 36.0;
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+rw/2.0-4.0, y+28.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+PAD, y, rx+rw/2.0-4.0, y+28.0), C_LINE, 1.0);
    label_bar(scene, "X 0", rx+PAD+8.0, y+8.0, 10.0, C_TEXT);
    fill_rrect(scene, Rect::new(rx+rw/2.0+4.0, y, rx+rw-PAD, y+28.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+rw/2.0+4.0, y, rx+rw-PAD, y+28.0), C_LINE, 1.0);
    label_bar(scene, "Y 60", rx+rw/2.0+12.0, y+8.0, 10.0, C_TEXT);
    y += 36.0;
    hline(scene, rx, rx+rw, y, C_LINE);
    y += 8.0;

    // Auto Layout — smaller 84x84 dark, no 09, + dark icon
    label_bar(scene, "Auto layout", rx+PAD, y+2.0, 11.0, C_TEXT);
    fill_rrect(scene, Rect::new(rx+rw-28.0, y, rx+rw-8.0, y+20.0), 6.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+rw-28.0, y, rx+rw-8.0, y+20.0), C_LINE, 1.0);
    draw_icon(scene, Icon::Plus, rx+rw-23.0, y+4.0, 10.0, C_DIM);
    y += 24.0;
    label_bar(scene, "Flow", rx+PAD, y, 9.0, C_DIM);
    y += 14.0;
    let fw = (rw - PAD*2.0 - 9.0) / 4.0;
    for i in 0..4 {
        let fx = rx+PAD + i as f64*(fw+3.0);
        let active = i==0 || i==2;
        fill_rrect(scene, Rect::new(fx, y, fx+fw, y+28.0), 8.0, if active {C_FIELD_2} else {C_FIELD});
        stroke_rect(scene, Rect::new(fx, y, fx+fw, y+28.0), if active {C_LINE_2} else {C_LINE}, 1.0);
        draw_icon(scene, Icon::Grid, fx+(fw-14.0)/2.0, y+7.0, 14.0, if active {C_TEXT} else {C_DIM});
    }
    y += 36.0;
    // Alignment 84x84 dark
    label_bar(scene, "Alignment", rx+PAD, y, 9.0, C_DIM);
    label_bar(scene, "Gap", rx+PAD+96.0, y, 9.0, C_DIM);
    y += 14.0;
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+PAD+84.0, y+84.0), 12.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+PAD, y, rx+PAD+84.0, y+84.0), C_LINE, 1.0);
    fill_rect(scene, Rect::new(rx+PAD+12.0, y+42.0, rx+PAD+72.0, y+43.0), C_LINE_2);
    fill_rect(scene, Rect::new(rx+PAD+42.0, y+12.0, rx+PAD+43.0, y+72.0), C_LINE_2);
    // dots
    let dots = [
        (rx+PAD+18.0, y+18.0, true), (rx+PAD+42.0, y+18.0, false), (rx+PAD+66.0, y+18.0, true),
        (rx+PAD+18.0, y+42.0, false), (rx+PAD+42.0, y+42.0, false), (rx+PAD+66.0, y+42.0, false),
        (rx+PAD+18.0, y+66.0, false), (rx+PAD+42.0, y+66.0, false), (rx+PAD+66.0, y+66.0, false),
    ];
    for (dx, dy, active) in dots.iter() {
        if *active {
            fill_rrect(scene, Rect::new(*dx-4.0, *dy-4.0, *dx+4.0, *dy+4.0), 4.0, C_TEXT);
        } else {
            fill_rrect(scene, Rect::new(*dx-3.0, *dy-3.0, *dx+3.0, *dy+3.0), 3.0, C_DIM);
        }
    }
    fill_rrect(scene, Rect::new(rx+PAD+96.0, y, rx+rw-PAD, y+28.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+PAD+96.0, y, rx+rw-PAD, y+28.0), C_LINE, 1.0);
    label_bar(scene, "Gap 5", rx+PAD+104.0, y+8.0, 10.0, C_TEXT);
    y += 92.0;
    hline(scene, rx, rx+rw, y, C_LINE);
    y += 8.0;

    // Appearance
    label_bar(scene, "Appearance", rx+PAD, y+2.0, 10.0, C_TEXT);
    y += 20.0;
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+rw/2.0-4.0, y+28.0), 8.0, C_FIELD);
    label_bar(scene, "Opacity 100%", rx+PAD+8.0, y+8.0, 10.0, C_TEXT);
    fill_rrect(scene, Rect::new(rx+rw/2.0+4.0, y, rx+rw-PAD, y+28.0), 8.0, C_FIELD);
    label_bar(scene, "Radius 0", rx+rw/2.0+12.0, y+8.0, 10.0, C_TEXT);
    y += 36.0;
    hline(scene, rx, rx+rw, y, C_LINE);
    y += 8.0;

    // Typography V37
    label_bar(scene, "Typography", rx+PAD, y+2.0, 10.0, C_TEXT);
    y += 20.0;
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+rw-PAD, y+28.0), 8.0, C_FIELD);
    label_bar(scene, "Manrope Regular 14", rx+PAD+10.0, y+8.0, 11.0, C_TEXT);
    y += 36.0;
    hline(scene, rx, rx+rw, y, C_LINE);
    y += 8.0;

    // Fill above Stroke
    label_bar(scene, "Fill", rx+PAD, y+2.0, 10.0, C_TEXT);
    y += 20.0;
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+rw-PAD, y+28.0), 8.0, C_FIELD);
    fill_rrect(scene, Rect::new(rx+PAD+6.0, y+6.0, rx+PAD+22.0, y+22.0), 4.0, Color::from_rgb8(0xFF,0xFF,0xFF));
    label_bar(scene, "FFFFFF 100%", rx+PAD+28.0, y+8.0, 11.0, C_TEXT);
    y += 36.0;
    hline(scene, rx, rx+rw, y, C_LINE);
    y += 8.0;

    // Stroke
    label_bar(scene, "Stroke", rx+PAD, y+2.0, 10.0, C_TEXT);
    y += 20.0;
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+rw-PAD, y+28.0), 8.0, C_FIELD);
    fill_rrect(scene, Rect::new(rx+PAD+6.0, y+6.0, rx+PAD+22.0, y+22.0), 4.0, Color::from_rgb8(0x00,0x00,0x00));
    label_bar(scene, "000000 Outside 1", rx+PAD+28.0, y+8.0, 10.0, C_TEXT);
    y += 36.0;
    hline(scene, rx, rx+rw, y, C_LINE);
    y += 8.0;

    // Guides — no dropdown icon, styled like other tabs, Square 16
    label_bar(scene, "Guides", rx+PAD, y+2.0, 10.0, C_TEXT);
    // Square 16 icon — dark style
    fill_rrect(scene, Rect::new(rx+rw-28.0, y, rx+rw-8.0, y+20.0), 6.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+rw-28.0, y, rx+rw-8.0, y+20.0), C_LINE, 1.0);
    draw_icon(scene, Icon::Square, rx+rw-23.0, y+4.0, 10.0, C_DIM);
    y += 24.0;
    fill_rrect(scene, Rect::new(rx+PAD, y, rx+rw-PAD, y+28.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+PAD, y, rx+rw-PAD, y+28.0), C_LINE, 1.0);
    draw_icon(scene, Icon::Square, rx+PAD+8.0, y+7.0, 14.0, C_DIM);
    label_bar(scene, "Square 16", rx+PAD+28.0, y+8.0, 10.0, C_TEXT);
    y += 36.0;
    hline(scene, rx, rx+rw, y, C_LINE);
    y += 8.0;

    // Export — styled like other tabs, collapsed by default, + toggles visibility
    label_bar(scene, "Export", rx+PAD, y+2.0, 10.0, C_TEXT);
    fill_rrect(scene, Rect::new(rx+rw-28.0, y, rx+rw-8.0, y+20.0), 6.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx+rw-28.0, y, rx+rw-8.0, y+20.0), C_LINE, 1.0);
    draw_icon(scene, Icon::Plus, rx+rw-23.0, y+4.0, 10.0, C_DIM);
    y += 24.0;
    if app.export_expanded {
        fill_rrect(scene, Rect::new(rx+PAD, y, rx+PAD+56.0, y+24.0), 6.0, C_FIELD);
        label_bar(scene, "PNG", rx+PAD+10.0, y+6.0, 10.0, C_TEXT);
        fill_rrect(scene, Rect::new(rx+PAD+64.0, y, rx+PAD+108.0, y+24.0), 6.0, C_FIELD);
        label_bar(scene, "1x", rx+PAD+74.0, y+6.0, 10.0, C_TEXT);
        fill_rrect(scene, Rect::new(rx+PAD+116.0, y, rx+rw-PAD, y+24.0), 6.0, C_FIELD);
        label_bar(scene, "Suffix", rx+PAD+124.0, y+6.0, 10.0, C_DIM);
        y += 32.0;
        fill_rrect(scene, Rect::new(rx+PAD, y, rx+rw-PAD, y+28.0), 6.0, C_FIELD_2);
        stroke_rect(scene, Rect::new(rx+PAD, y, rx+rw-PAD, y+28.0), C_LINE_2, 1.0);
        label_bar(scene, "EXPORT 1 ELEMENT", rx+rw/2.0-44.0, y+8.0, 9.0, C_MUTED);
    } else {
        // collapsed hint
        label_bar(scene, "No exports — click + to add", rx+PAD, y+4.0, 10.0, C_FAINT);
    }
}

fn paint_status(scene: &mut Scene, app: &AppState, r: &Regions) {
    fill_rect(scene, r.status, C_PANEL);
    hline(scene, 0.0, app.win_w, app.win_h-STATUS_H, C_LINE);
    label_bar(scene, &app.status, PAD, app.win_h-STATUS_H+6.0, FONT_CAPTION, C_DIM);
}

fn paint_home(scene: &mut Scene, app: &AppState) {
    // Dashboard — final — matches dashboard-v1.html
    fill_rect(scene, Rect::new(0.0, 0.0, app.win_w, app.win_h), C_BG);
    fill_rect(scene, Rect::new(0.0, 0.0, app.win_w, TITLE_H), C_PANEL);
    hline(scene, 0.0, app.win_w, TITLE_H, C_LINE);
    let lx = 8.0;
    let ly = (TITLE_H - 28.0) / 2.0;
    fill_rrect(scene, Rect::new(lx, ly, lx+28.0, ly+28.0), 4.0, C_FIELD);
    fill_rrect(scene, Rect::new(lx+14.0, ly+4.0, lx+24.0, ly+20.0), 2.0, Color::from_rgb8(0x1B, 0xCB, 0x55));
    label_bar(scene, "X", lx+7.0, ly+7.0, 12.0, C_TEXT);
    label_bar(scene, "X-Native", lx+40.0, ly+8.0, 14.0, C_TEXT);
    let sw = 480.0;
    let sx = (app.win_w - sw) / 2.0;
    fill_rrect(scene, Rect::new(sx, 6.0, sx+sw, 34.0), 10.0, C_FIELD);
    stroke_rect(scene, Rect::new(sx, 6.0, sx+sw, 34.0), C_LINE, 1.0);
    draw_icon(scene, Icon::Search, sx+10.0, 12.0, 14.0, C_DIM);
    label_bar(scene, "Search files, teams, or projects", sx+32.0, 12.0, 11.0, C_FAINT);
    let rx = app.win_w - 140.0;
    fill_rrect(scene, Rect::new(rx, 6.0, rx+80.0, 34.0), 8.0, C_FIELD);
    stroke_rect(scene, Rect::new(rx, 6.0, rx+80.0, 34.0), C_LINE, 1.0);
    draw_icon(scene, Icon::Plus, rx+8.0, 12.0, 14.0, C_DIM);
    label_bar(scene, "New file", rx+28.0, 12.0, 11.0, C_TEXT);
    fill_rrect(scene, Rect::new(app.win_w-36.0, 6.0, app.win_w-8.0, 34.0), 14.0, Color::from_rgb8(0xFF, 0xEB, 0x3B));
    label_bar(scene, "S", app.win_w-26.0, 12.0, 11.0, Color::from_rgb8(0x00,0x00,0x00));

    let left_w = 260.0;
    fill_rect(scene, Rect::new(0.0, TITLE_H, left_w, app.win_h-TITLE_H), C_PANEL);
    vline(scene, left_w, TITLE_H, app.win_h, C_LINE);
    let mut y = TITLE_H + 12.0;
    draw_icon(scene, Icon::Drafts, 12.0, y+2.0, 14.0, C_DIM);
    label_bar(scene, "DRAFTS", 32.0, y+2.0, 9.0, C_DIM);
    y += 22.0;
    fill_rrect(scene, Rect::new(8.0, y-2.0, 12.0, y+2.0), 2.0, Color::from_rgb8(0x2E,0xCC,0x71));
    label_bar(scene, "Personal", 20.0, y, 11.0, C_TEXT);
    y += 28.0;
    let nav = [("Home", Icon::Home, true), ("Recents", Icon::Board, false), ("Starred", Icon::Sparkles, false), ("Trash", Icon::File, false)];
    for (name, icon, sel) in nav.iter() {
        if *sel {
            fill_rrect(scene, Rect::new(8.0, y, left_w-8.0, y+32.0), 8.0, C_FIELD);
            stroke_rect(scene, Rect::new(8.0, y, left_w-8.0, y+32.0), C_LINE_2, 1.0);
            draw_icon(scene, *icon, 14.0, y+8.0, 14.0, C_TEXT);
            label_bar(scene, name, 36.0, y+9.0, 11.0, C_TEXT);
        } else {
            draw_icon(scene, *icon, 14.0, y+8.0, 14.0, C_DIM);
            label_bar(scene, name, 36.0, y+9.0, 11.0, C_MUTED);
        }
        y += 36.0;
    }
    y += 8.0;
    hline(scene, 0.0, left_w, y, C_LINE);
    y += 12.0;
    label_bar(scene, "TEAMS", 12.0, y, 9.0, C_DIM);
    y += 20.0;
    fill_rrect(scene, Rect::new(12.0, y, 32.0, y+20.0), 6.0, Color::from_rgb8(0x5B, 0x7C, 0xFF));
    label_bar(scene, "L", 18.0, y+3.0, 10.0, C_TEXT);
    label_bar(scene, "Liquor Delivery", 40.0, y+4.0, 11.0, C_MUTED);
    y += 28.0;
    fill_rrect(scene, Rect::new(12.0, y, 32.0, y+20.0), 6.0, Color::from_rgb8(0xFF, 0x7A, 0x45));
    label_bar(scene, "D", 18.0, y+3.0, 10.0, C_TEXT);
    label_bar(scene, "Design System", 40.0, y+4.0, 11.0, C_MUTED);

    let mx = left_w + 24.0;
    let mut my = TITLE_H + 24.0;
    label_bar(scene, "Welcome back, Sahil", mx, my, 20.0, C_TEXT);
    my += 24.0;
    label_bar(scene, "You have 12 files edited in the last 7 days", mx, my, 11.0, C_MUTED);
    my += 32.0;
    let card_w = (app.win_w - left_w - 48.0 - 24.0) / 4.0;
    let qa = ["New design file", "Import file", "Browse templates", "Invite team"];
    for i in 0..4 {
        let cx = mx + i as f64 * (card_w + 8.0);
        fill_rrect(scene, Rect::new(cx, my, cx+card_w, my+88.0), 12.0, C_PANEL);
        stroke_rect(scene, Rect::new(cx, my, cx+card_w, my+88.0), C_LINE, 1.0);
        if i==0 {
            fill_rrect(scene, Rect::new(cx+12.0, my+12.0, cx+44.0, my+44.0), 8.0, C_TEXT);
            draw_icon(scene, Icon::Plus, cx+18.0, my+18.0, 16.0, Color::from_rgb8(0x00,0x00,0x00));
        } else {
            fill_rrect(scene, Rect::new(cx+12.0, my+12.0, cx+44.0, my+44.0), 8.0, C_FIELD);
            draw_icon(scene, Icon::Grid, cx+18.0, my+18.0, 16.0, C_DIM);
        }
        label_bar(scene, qa[i], cx+12.0, my+56.0, 11.0, C_TEXT);
    }
    my += 108.0;
    label_bar(scene, "Recents", mx, my, 14.0, C_TEXT);
    my += 32.0;
    let cols = 3;
    let gap = 12.0;
    let gw = (app.win_w - left_w - 48.0 - (cols as f64 -1.0)*gap) / cols as f64;
    let file_names = ["Liquor Delivery App UI", "DESIGN_SYSTEM.md", "Payment Flow", "Onboarding Screens", "Dashboard Redesign", "Landing Page"];
    for row in 0..2 {
        for col in 0..cols {
            let idx = row*cols+col;
            if idx >= file_names.len() { break; }
            let fx = mx + col as f64 * (gw+gap);
            let fy = my + row as f64 * (180.0+gap);
            fill_rrect(scene, Rect::new(fx, fy, fx+gw, fy+180.0), 12.0, C_PANEL);
            stroke_rect(scene, Rect::new(fx, fy, fx+gw, fy+180.0), C_LINE, 1.0);
            fill_rrect(scene, Rect::new(fx, fy, fx+gw, fy+110.0), 12.0, Color::from_rgb8(0xFF,0xFF,0xFF));
            label_bar(scene, "X", fx+gw/2.0-6.0, fy+46.0, 20.0, Color::from_rgba8(0x00,0x00,0x00,0x15));
            label_bar(scene, file_names[idx], fx+12.0, fy+122.0, 11.0, C_TEXT);
            label_bar(scene, "Edited 2h ago", fx+12.0, fy+138.0, 9.0, C_DIM);
        }
    }
}

fn paint_command_palette(scene: &mut Scene, app: &AppState) {
    let w = 440.0;
    let h = 280.0;
    let x = (app.win_w - w) / 2.0;
    let y = app.win_h * 0.18;
    fill_rect(scene, Rect::new(0.0, 0.0, app.win_w, app.win_h), Color::from_rgba8(0,0,0,120));
    fill_rrect(scene, Rect::new(x, y, x+w, y+h), RADIUS_LG, C_RAISED);
    stroke_rect(scene, Rect::new(x, y, x+w, y+h), C_LINE_2, 1.0);
    fill_rrect(scene, Rect::new(x+12.0, y+12.0, x+w-12.0, y+44.0), RADIUS_MD, C_FIELD);
    let q = if app.command_query.is_empty() {"Type a command..."} else {app.command_query.as_str()};
    label_bar(scene, q, x+24.0, y+22.0, FONT_BODY, if app.command_query.is_empty(){C_FAINT}else{C_TEXT});
    let cmds = ["Create frame", "Create rectangle", "Add text", "Zoom to fit", "Toggle grid", "Export PNG"];
    for (i, c) in cmds.iter().enumerate() { label_bar(scene, c, x+24.0, y+60.0+i as f64*28.0, FONT_BODY, C_MUTED); }
}
