//! App shell: regions + chrome paint (Graphite & Signal).

use vello::kurbo::Rect;
use vello::Scene;

use crate::paint::{fill_rect, fill_rrect, hline, label_bar, measure, stroke_rect, vline};
use crate::state::{AppState, LeftTab, Screen, Tool};
use crate::theme::*;

pub struct Regions {
    pub title: Rect,
    pub tools: Rect,
    pub left: Rect,
    pub right: Rect,
    pub canvas: Rect,
    pub status: Rect,
}

pub fn regions(w: f64, h: f64) -> Regions {
    Regions {
        title: Rect::new(0.0, 0.0, w, TITLE_H),
        tools: Rect::new(0.0, TITLE_H, TOOL_W, h - STATUS_H),
        left: Rect::new(TOOL_W, TITLE_H, TOOL_W + LEFT_W, h - STATUS_H),
        right: Rect::new(w - RIGHT_W, TITLE_H, w, h - STATUS_H),
        canvas: Rect::new(TOOL_W + LEFT_W, TITLE_H, w - RIGHT_W, h - STATUS_H),
        status: Rect::new(0.0, h - STATUS_H, w, h),
    }
}

pub fn paint_shell(scene: &mut Scene, app: &AppState) {
    let r = regions(app.win_w, app.win_h);
    fill_rect(scene, Rect::new(0.0, 0.0, app.win_w, app.win_h), C_BASE);

    match app.screen {
        Screen::Home => paint_home(scene, app),
        Screen::Editor => {
            paint_title(scene, app, &r);
            paint_tools(scene, app, &r);
            paint_left(scene, app, &r);
            paint_canvas(scene, app, &r);
            paint_right(scene, app, &r);
            paint_status(scene, app, &r);
            if app.command_open {
                paint_command_palette(scene, app);
            }
        }
    }
}

fn paint_title(scene: &mut Scene, app: &AppState, r: &Regions) {
    fill_rect(scene, r.title, C_PANEL);
    hline(scene, 0.0, app.win_w, TITLE_H, C_EDGE);
    // Mark + name
    fill_rrect(
        scene,
        Rect::new(12.0, 10.0, 28.0, 26.0),
        RADIUS_SM,
        C_ACCENT,
    );
    label_bar(scene, "X-NATIVE", 36.0, 12.0, FONT_BODY, C_TEXT);
    label_bar(scene, &app.doc_name, 110.0, 12.0, FONT_BODY, C_DIM);
    if app.dirty {
        fill_rrect(
            scene,
            Rect::new(110.0 + measure(&app.doc_name, FONT_BODY) + 8.0, 16.0, 116.0 + measure(&app.doc_name, FONT_BODY), 22.0),
            3.0,
            C_ACCENT,
        );
    }
    // Zoom chip
    let z = format!("{}%", (app.zoom * 100.0).round() as i32);
    let zx = app.win_w - RIGHT_W - 80.0;
    fill_rrect(
        scene,
        Rect::new(zx, 8.0, zx + 64.0, 28.0),
        RADIUS_MD,
        C_RAISED,
    );
    label_bar(scene, &z, zx + 16.0, 12.0, FONT_CAPTION, C_DIM);
}

fn paint_tools(scene: &mut Scene, app: &AppState, r: &Regions) {
    fill_rect(scene, r.tools, C_PANEL);
    vline(scene, TOOL_W, TITLE_H, app.win_h - STATUS_H, C_EDGE);
    let tools = [
        Tool::Select,
        Tool::Frame,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Line,
        Tool::Pen,
        Tool::Text,
        Tool::Hand,
    ];
    for (i, t) in tools.iter().enumerate() {
        let y = TITLE_H + 12.0 + i as f64 * 40.0;
        let box_r = Rect::new(8.0, y, TOOL_W - 8.0, y + 32.0);
        if *t == app.tool {
            fill_rrect(scene, box_r, RADIUS_MD, C_ACTIVE);
            stroke_rect(scene, box_r, C_ACCENT, 1.0);
        } else {
            // idle
        }
        // icon stand-in: small accent mark for active
        let c = if *t == app.tool { C_ACCENT } else { C_DIM };
        fill_rrect(
            scene,
            Rect::new(18.0, y + 10.0, TOOL_W - 18.0, y + 22.0),
            2.0,
            c,
        );
    }
}

fn paint_left(scene: &mut Scene, app: &AppState, r: &Regions) {
    fill_rect(scene, r.left, C_PANEL);
    vline(
        scene,
        TOOL_W + LEFT_W,
        TITLE_H,
        app.win_h - STATUS_H,
        C_EDGE,
    );
    // Tabs
    let tabs = [
        (LeftTab::Layers, "Layers"),
        (LeftTab::Assets, "Assets"),
        (LeftTab::Components, "Components"),
        (LeftTab::Variables, "Variables"),
    ];
    let mut x = TOOL_W + 8.0;
    for (tab, name) in tabs {
        let w = measure(name, FONT_CAPTION) + 12.0;
        if tab == app.left_tab {
            fill_rrect(
                scene,
                Rect::new(x, TITLE_H + 6.0, x + w, TITLE_H + 26.0),
                RADIUS_SM,
                C_RAISED,
            );
            label_bar(scene, name, x + 6.0, TITLE_H + 10.0, FONT_CAPTION, C_TEXT);
        } else {
            label_bar(scene, name, x + 6.0, TITLE_H + 10.0, FONT_CAPTION, C_FAINT);
        }
        x += w + 4.0;
    }
    hline(
        scene,
        TOOL_W,
        TOOL_W + LEFT_W,
        TITLE_H + 32.0,
        C_EDGE,
    );

    if app.left_tab != LeftTab::Layers {
        label_bar(
            scene,
            "Workspace coming in Phase 4",
            TOOL_W + PAD,
            TITLE_H + 56.0,
            FONT_CAPTION,
            C_FAINT,
        );
        return;
    }

    // Pages
    label_bar(
        scene,
        "PAGES",
        TOOL_W + PAD,
        TITLE_H + 44.0,
        FONT_CAPTION,
        C_FAINT,
    );
    for (i, p) in app.pages.iter().enumerate() {
        let y = TITLE_H + 64.0 + i as f64 * ROW_H;
        let row = Rect::new(TOOL_W + 6.0, y, TOOL_W + LEFT_W - 6.0, y + ROW_H - 2.0);
        if i == app.page_idx {
            fill_rrect(scene, row, RADIUS_SM, C_SELECTED);
        }
        let name = if p.name.is_empty() {
            p.id.as_str()
        } else {
            p.name.as_str()
        };
        label_bar(
            scene,
            name,
            TOOL_W + 16.0,
            y + 6.0,
            FONT_BODY,
            if i == app.page_idx { C_TEXT } else { C_DIM },
        );
    }
    let pages_end = TITLE_H + 64.0 + app.pages.len() as f64 * ROW_H;
    label_bar(
        scene,
        "+ New Page",
        TOOL_W + 16.0,
        pages_end + 4.0,
        FONT_CAPTION,
        C_DIM,
    );

    // Layers
    let layers_y = pages_end + 28.0;
    hline(
        scene,
        TOOL_W + 8.0,
        TOOL_W + LEFT_W - 8.0,
        layers_y,
        C_EDGE,
    );
    label_bar(
        scene,
        "LAYERS",
        TOOL_W + PAD,
        layers_y + 8.0,
        FONT_CAPTION,
        C_FAINT,
    );
    let rows = app.layer_rows();
    if rows.is_empty() {
        label_bar(
            scene,
            "No layers on this page",
            TOOL_W + PAD,
            layers_y + 32.0,
            FONT_BODY,
            C_FAINT,
        );
    } else {
        for (i, (_id, name, depth)) in rows.iter().enumerate() {
            let y = layers_y + 28.0 + i as f64 * ROW_H;
            label_bar(
                scene,
                name,
                TOOL_W + PAD + *depth as f64 * 12.0,
                y + 4.0,
                FONT_BODY,
                C_TEXT,
            );
        }
    }
}

fn paint_canvas(scene: &mut Scene, app: &AppState, r: &Regions) {
    fill_rect(scene, r.canvas, C_CANVAS);
    // Page artboard ghost (if empty page)
    let aw = 1600.0 * app.zoom;
    let ah = 1000.0 * app.zoom;
    let ax = r.canvas.x0 + app.pan.0;
    let ay = r.canvas.y0 + app.pan.1;
    let board = Rect::new(ax, ay, ax + aw, ay + ah);
    // Only draw board outline if it intersects canvas
    fill_rect(scene, board, Color_white_soft());
    stroke_rect(scene, board, C_EDGE_2, 1.0);
}

fn Color_white_soft() -> vello::peniko::Color {
    vello::peniko::Color::from_rgb8(0xf4, 0xf5, 0xf7)
}

fn paint_right(scene: &mut Scene, app: &AppState, r: &Regions) {
    fill_rect(scene, r.right, C_PANEL);
    vline(scene, app.win_w - RIGHT_W, TITLE_H, app.win_h - STATUS_H, C_EDGE);

    let has_sel = !app.editor.selection.is_empty()
        && app
            .editor
            .selection
            .iter()
            .any(|id| id != &app.editor.root.id);

    if !has_sel {
        // Page / canvas properties — calm empty state
        label_bar(
            scene,
            "PAGE",
            app.win_w - RIGHT_W + PAD,
            TITLE_H + 16.0,
            FONT_CAPTION,
            C_FAINT,
        );
        label_bar(
            scene,
            &app.current_page_name(),
            app.win_w - RIGHT_W + PAD,
            TITLE_H + 36.0,
            FONT_TITLE,
            C_TEXT,
        );
        hline(
            scene,
            app.win_w - RIGHT_W + 8.0,
            app.win_w - 8.0,
            TITLE_H + 60.0,
            C_EDGE,
        );
        label_bar(
            scene,
            "Background",
            app.win_w - RIGHT_W + PAD,
            TITLE_H + 76.0,
            FONT_BODY,
            C_DIM,
        );
        fill_rrect(
            scene,
            Rect::new(
                app.win_w - RIGHT_W + PAD,
                TITLE_H + 98.0,
                app.win_w - RIGHT_W + PAD + 20.0,
                TITLE_H + 118.0,
            ),
            RADIUS_SM,
            C_CANVAS,
        );
        stroke_rect(
            scene,
            Rect::new(
                app.win_w - RIGHT_W + PAD,
                TITLE_H + 98.0,
                app.win_w - RIGHT_W + PAD + 20.0,
                TITLE_H + 118.0,
            ),
            C_EDGE,
            1.0,
        );
        label_bar(
            scene,
            "Canvas void",
            app.win_w - RIGHT_W + PAD + 28.0,
            TITLE_H + 102.0,
            FONT_BODY,
            C_DIM,
        );
        label_bar(
            scene,
            "Select a layer to edit properties",
            app.win_w - RIGHT_W + PAD,
            TITLE_H + 140.0,
            FONT_CAPTION,
            C_FAINT,
        );
        label_bar(
            scene,
            "F Frame · R Rect · T Text",
            app.win_w - RIGHT_W + PAD,
            TITLE_H + 160.0,
            FONT_CAPTION,
            C_FAINT,
        );
        return;
    }

    label_bar(
        scene,
        "DESIGN",
        app.win_w - RIGHT_W + PAD,
        TITLE_H + 16.0,
        FONT_CAPTION,
        C_FAINT,
    );
    label_bar(
        scene,
        "Position · Size · Fill · Stroke",
        app.win_w - RIGHT_W + PAD,
        TITLE_H + 40.0,
        FONT_BODY,
        C_DIM,
    );
    label_bar(
        scene,
        "(Property editors Phase 2)",
        app.win_w - RIGHT_W + PAD,
        TITLE_H + 60.0,
        FONT_CAPTION,
        C_FAINT,
    );
}

fn paint_status(scene: &mut Scene, app: &AppState, r: &Regions) {
    fill_rect(scene, r.status, C_PANEL);
    hline(scene, 0.0, app.win_w, app.win_h - STATUS_H, C_EDGE);
    label_bar(
        scene,
        &app.status,
        PAD,
        app.win_h - STATUS_H + 6.0,
        FONT_CAPTION,
        C_DIM,
    );
    label_bar(
        scene,
        "⌘K",
        app.win_w - 48.0,
        app.win_h - STATUS_H + 6.0,
        FONT_CAPTION,
        C_ACCENT,
    );
}

fn paint_home(scene: &mut Scene, app: &AppState) {
    fill_rect(scene, Rect::new(0.0, 0.0, app.win_w, app.win_h), C_BASE);
    // Center card
    let cw = 480.0;
    let ch = 360.0;
    let cx = (app.win_w - cw) / 2.0;
    let cy = (app.win_h - ch) / 2.0;
    fill_rrect(scene, Rect::new(cx, cy, cx + cw, cy + ch), RADIUS_LG, C_PANEL);
    stroke_rect(scene, Rect::new(cx, cy, cx + cw, cy + ch), C_EDGE, 1.0);
    fill_rrect(
        scene,
        Rect::new(cx + 32.0, cy + 36.0, cx + 56.0, cy + 60.0),
        RADIUS_SM,
        C_ACCENT,
    );
    label_bar(scene, "X-Native", cx + 68.0, cy + 40.0, FONT_TITLE, C_TEXT);
    label_bar(
        scene,
        "Create your first design",
        cx + 32.0,
        cy + 88.0,
        FONT_BODY,
        C_DIM,
    );

    let options = ["Blank canvas", "Desktop 1440", "Mobile 390", "Open file…"];
    for (i, opt) in options.iter().enumerate() {
        let y = cy + 130.0 + i as f64 * 44.0;
        let row = Rect::new(cx + 32.0, y, cx + cw - 32.0, y + 36.0);
        fill_rrect(scene, row, RADIUS_MD, C_RAISED);
        label_bar(scene, opt, cx + 48.0, y + 10.0, FONT_BODY, C_TEXT);
    }
}

fn paint_command_palette(scene: &mut Scene, app: &AppState) {
    let w = 440.0;
    let h = 280.0;
    let x = (app.win_w - w) / 2.0;
    let y = app.win_h * 0.18;
    // Dim
    fill_rect(
        scene,
        Rect::new(0.0, 0.0, app.win_w, app.win_h),
        vello::peniko::Color::from_rgba8(0, 0, 0, 120),
    );
    fill_rrect(scene, Rect::new(x, y, x + w, y + h), RADIUS_LG, C_RAISED);
    stroke_rect(scene, Rect::new(x, y, x + w, y + h), C_EDGE_2, 1.0);
    fill_rrect(
        scene,
        Rect::new(x + 12.0, y + 12.0, x + w - 12.0, y + 44.0),
        RADIUS_MD,
        C_FIELD,
    );
    let q = if app.command_query.is_empty() {
        "Type a command…"
    } else {
        app.command_query.as_str()
    };
    label_bar(
        scene,
        q,
        x + 24.0,
        y + 22.0,
        FONT_BODY,
        if app.command_query.is_empty() {
            C_FAINT
        } else {
            C_TEXT
        },
    );
    let cmds = [
        "Create frame",
        "Create rectangle",
        "Add text",
        "Zoom to fit",
        "Toggle grid",
        "Export PNG",
    ];
    for (i, c) in cmds.iter().enumerate() {
        label_bar(
            scene,
            c,
            x + 24.0,
            y + 60.0 + i as f64 * 28.0,
            FONT_BODY,
            C_DIM,
        );
    }
}
