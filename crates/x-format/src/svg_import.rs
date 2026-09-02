use x_core::*;
use crate::import_ir::{lower, ImportDoc, ImportKind, ImportNode};
#[allow(unused_imports)]
use crate::*;

// --------------------------------------------------------------- SVG import

/// Phase 7.4: import SVG into the native node tree. Handles the subset a
/// design tool actually round-trips: svg/g/rect/ellipse/circle/line/path/
/// text elements, fill/opacity/rx/transform=translate/rotate attributes,
/// nested groups, `<path d=...>` with M/L/C/Z (absolute and relative
/// m/l/c/z, H/V/h/v). Unknown elements/attributes are skipped, never fatal.
pub fn import_svg(svg: &str) -> Result<Node, String> {
    // parse -> shared Import IR -> lower() (ONE set of import semantics
    // across svg/sketch/figma/png), then unwrap the single page.
    let mut lexer = XmlLexer { s: svg.as_bytes(), i: 0 };
    loop {
        match lexer.next_tag()? {
            XmlTag::Open(name, attrs) | XmlTag::SelfClose(name, attrs) if name == "svg" => {
                let w = attr_num(&attrs, "width").unwrap_or(800.0);
                let h = attr_num(&attrs, "height").unwrap_or(600.0);
                let mut root = ImportNode::new(ImportKind::Frame).id("svg-root").size(w, h);
                parse_children(&mut lexer, &mut root)?;
                let doc = lower(ImportDoc { source: "svg", pages: vec![root], ..Default::default() });
                return doc.pages.into_iter().next().ok_or_else(|| "empty svg".into());
            }
            XmlTag::Eof => return Err("no <svg> element found".into()),
            _ => {}
        }
    }
}

// `Text`'s payload is kept for the upcoming <text> import path; the
// current lexer emits it but the importer does not consume it yet.
#[allow(dead_code)]
enum XmlTag { Open(String, Vec<(String, String)>), SelfClose(String, Vec<(String, String)>), Close(String), Text(String), Eof }

struct XmlLexer<'a> { s: &'a [u8], i: usize }
impl<'a> XmlLexer<'a> {
    fn next_tag(&mut self) -> Result<XmlTag, String> {
        // capture text content until next '<'
        let text_start = self.i;
        while self.i < self.s.len() && self.s[self.i] != b'<' { self.i += 1; }
        if self.i > text_start {
            let t = std::str::from_utf8(&self.s[text_start..self.i]).map_err(|_| "bad utf8")?.trim().to_string();
            if !t.is_empty() { return Ok(XmlTag::Text(t)); }
        }
        if self.i >= self.s.len() { return Ok(XmlTag::Eof); }
        self.i += 1; // consume '<'
        // comments / doctype / processing instructions
        if self.s.get(self.i) == Some(&b'!') || self.s.get(self.i) == Some(&b'?') {
            while self.i < self.s.len() && self.s[self.i] != b'>' { self.i += 1; }
            self.i += 1;
            return self.next_tag();
        }
        let closing = self.s.get(self.i) == Some(&b'/');
        if closing { self.i += 1; }
        let name_start = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'-' || self.s[self.i] == b':') { self.i += 1; }
        let name = std::str::from_utf8(&self.s[name_start..self.i]).map_err(|_| "bad utf8")?.to_string();
        let mut attrs = vec![];
        loop {
            while self.i < self.s.len() && (self.s[self.i] as char).is_ascii_whitespace() { self.i += 1; }
            match self.s.get(self.i) {
                Some(b'>') => { self.i += 1; return Ok(if closing { XmlTag::Close(name) } else { XmlTag::Open(name, attrs) }); }
                Some(b'/') => { self.i += 2; return Ok(XmlTag::SelfClose(name, attrs)); }
                None => return Ok(XmlTag::Eof),
                _ => {
                    let ks = self.i;
                    while self.i < self.s.len() && self.s[self.i] != b'=' && !(self.s[self.i] as char).is_ascii_whitespace() && self.s[self.i] != b'>' { self.i += 1; }
                    let key = std::str::from_utf8(&self.s[ks..self.i]).map_err(|_| "bad utf8")?.to_string();
                    if self.s.get(self.i) == Some(&b'=') {
                        self.i += 1;
                        let quote = *self.s.get(self.i).ok_or("eof in attr")?;
                        if quote == b'"' || quote == b'\'' {
                            self.i += 1;
                            let vs = self.i;
                            while self.i < self.s.len() && self.s[self.i] != quote { self.i += 1; }
                            let val = std::str::from_utf8(&self.s[vs..self.i]).map_err(|_| "bad utf8")?.to_string();
                            self.i += 1;
                            attrs.push((key, val));
                        }
                    }
                }
            }
        }
    }
}

fn attr<'v>(attrs: &'v [(String, String)], key: &str) -> Option<&'v str> { attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str()) }
fn attr_num(attrs: &[(String, String)], key: &str) -> Option<f64> {
    attr(attrs, key).and_then(|v| v.trim_end_matches("px").parse().ok())
}
fn attr_fill(attrs: &[(String, String)]) -> Color {
    match attr(attrs, "fill") {
        Some("none") => Color::TRANSPARENT,
        Some(s) => parse_hex_color(s).unwrap_or(Color::BLACK),
        None => Color::BLACK,
    }
}
fn apply_transform_attr(node: &mut ImportNode, attrs: &[(String, String)]) {
    if let Some(t) = attr(attrs, "transform") {
        // supports translate(x y) and rotate(deg ...) — what we export.
        // SVG rotate() is clockwise-positive in y-down space, which IS
        // the native convention: no sign flip (unlike Sketch).
        if let Some(rest) = t.split("translate(").nth(1) {
            let args: Vec<f64> = rest.split(')').next().unwrap_or("").split([' ', ',']).filter_map(|v| v.trim().parse().ok()).collect();
            if let Some(x) = args.first() { node.x += x; }
            if let Some(y) = args.get(1) { node.y += y; }
        }
        if let Some(rest) = t.split("rotate(").nth(1) {
            if let Some(deg) = rest.split(')').next().unwrap_or("").split([' ', ',']).next().and_then(|v| v.trim().parse::<f64>().ok()) {
                node.rotation += deg.to_radians();
            }
        }
    }
    if let Some(op) = attr_num(attrs, "opacity") { node.opacity = op as f32; }
}

pub(crate) fn parse_path_d(d: &str) -> Vec<PathCmd> {
    let mut out = vec![];
    let mut nums: Vec<f64> = vec![];
    let mut cmd = ' ';
    let (mut cx, mut cy) = (0.0f64, 0.0f64);
    let flush = |cmd: char, nums: &mut Vec<f64>, out: &mut Vec<PathCmd>, cx: &mut f64, cy: &mut f64| {
        let rel = cmd.is_ascii_lowercase();
        match cmd.to_ascii_uppercase() {
            'M' => for pair in nums.chunks(2) { if pair.len() == 2 {
                let (x, y) = if rel { (*cx + pair[0], *cy + pair[1]) } else { (pair[0], pair[1]) };
                out.push(PathCmd::MoveTo(x, y)); *cx = x; *cy = y;
            }},
            'L' => for pair in nums.chunks(2) { if pair.len() == 2 {
                let (x, y) = if rel { (*cx + pair[0], *cy + pair[1]) } else { (pair[0], pair[1]) };
                out.push(PathCmd::LineTo(x, y)); *cx = x; *cy = y;
            }},
            'H' => for v in nums.iter() { let x = if rel { *cx + v } else { *v }; out.push(PathCmd::LineTo(x, *cy)); *cx = x; },
            'V' => for v in nums.iter() { let y = if rel { *cy + v } else { *v }; out.push(PathCmd::LineTo(*cx, y)); *cy = y; },
            'C' => for six in nums.chunks(6) { if six.len() == 6 {
                let (x1, y1, x2, y2, x, y) = if rel {
                    (*cx + six[0], *cy + six[1], *cx + six[2], *cy + six[3], *cx + six[4], *cy + six[5])
                } else { (six[0], six[1], six[2], six[3], six[4], six[5]) };
                out.push(PathCmd::CurveTo(x1, y1, x2, y2, x, y)); *cx = x; *cy = y;
            }},
            'Z' => out.push(PathCmd::Close),
            _ => {}
        }
        nums.clear();
    };
    let mut num_buf = String::new();
    let push_num = |num_buf: &mut String, nums: &mut Vec<f64>| {
        if !num_buf.is_empty() { if let Ok(v) = num_buf.parse() { nums.push(v); } num_buf.clear(); }
    };
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            push_num(&mut num_buf, &mut nums);
            if cmd != ' ' { flush(cmd, &mut nums, &mut out, &mut cx, &mut cy); }
            cmd = ch;
            if ch == 'Z' || ch == 'z' { flush(cmd, &mut nums, &mut out, &mut cx, &mut cy); cmd = ' '; }
        } else if ch.is_ascii_digit() || ch == '.' || ch == 'e' {
            num_buf.push(ch);
        } else if ch == '-' {
            if !num_buf.is_empty() && !num_buf.ends_with('e') { push_num(&mut num_buf, &mut nums); }
            num_buf.push(ch);
        } else {
            push_num(&mut num_buf, &mut nums);
        }
    }
    push_num(&mut num_buf, &mut nums);
    if cmd != ' ' { flush(cmd, &mut nums, &mut out, &mut cx, &mut cy); }
    out
}

fn parse_children(lexer: &mut XmlLexer, parent: &mut ImportNode) -> Result<(), String> {
    let mut pending_text_node: Option<ImportNode> = None;
    loop {
        match lexer.next_tag()? {
            XmlTag::Eof => return Ok(()),
            XmlTag::Close(_) => {
                if let Some(t) = pending_text_node.take() { parent.children.push(t); }
                return Ok(());
            }
            XmlTag::Text(content) => {
                if let Some(mut t) = pending_text_node.take() {
                    if let ImportKind::Text { content: c, .. } = &mut t.kind { *c = content; }
                    parent.children.push(t);
                }
            }
            tag @ (XmlTag::Open(..) | XmlTag::SelfClose(..)) => {
                let (name, attrs, self_closed) = match tag {
                    XmlTag::Open(n, a) => (n, a, false),
                    XmlTag::SelfClose(n, a) => (n, a, true),
                    _ => unreachable!(),
                };
                // source id if present; the shared lowering generates
                // fallbacks and dedupes — no local counter needed anymore
                let src_id = attr(&attrs, "id").map(String::from);
                let with_id = |mut n: ImportNode| { if let Some(i) = &src_id { n = n.id(i.clone()); } n };
                match name.as_str() {
                    "g" => {
                        let mut g = with_id(ImportNode::new(ImportKind::Group));
                        apply_transform_attr(&mut g, &attrs);
                        if !self_closed { parse_children(lexer, &mut g)?; }
                        parent.children.push(g);
                    }
                    "rect" => {
                        let mut n = with_id(ImportNode::new(ImportKind::Rect { radius: attr_num(&attrs, "rx").unwrap_or(0.0) }))
                            .at(attr_num(&attrs, "x").unwrap_or(0.0), attr_num(&attrs, "y").unwrap_or(0.0))
                            .size(attr_num(&attrs, "width").unwrap_or(0.0), attr_num(&attrs, "height").unwrap_or(0.0))
                            .fill(Paint::Solid(attr_fill(&attrs)));
                        apply_transform_attr(&mut n, &attrs);
                        if !self_closed { skip_element(lexer)?; }
                        parent.children.push(n);
                    }
                    "ellipse" | "circle" => {
                        let r = attr_num(&attrs, "r");
                        let rx = attr_num(&attrs, "rx").or(r).unwrap_or(0.0);
                        let ry = attr_num(&attrs, "ry").or(r).unwrap_or(0.0);
                        let cx = attr_num(&attrs, "cx").unwrap_or(0.0);
                        let cy = attr_num(&attrs, "cy").unwrap_or(0.0);
                        let mut n = with_id(ImportNode::new(ImportKind::Ellipse))
                            .at(cx - rx, cy - ry).size(rx * 2.0, ry * 2.0)
                            .fill(Paint::Solid(attr_fill(&attrs)));
                        apply_transform_attr(&mut n, &attrs);
                        if !self_closed { skip_element(lexer)?; }
                        parent.children.push(n);
                    }
                    "line" => {
                        let x1 = attr_num(&attrs, "x1").unwrap_or(0.0);
                        let y1 = attr_num(&attrs, "y1").unwrap_or(0.0);
                        let x2 = attr_num(&attrs, "x2").unwrap_or(0.0);
                        let stroke_c = attr(&attrs, "stroke").and_then(parse_hex_color).unwrap_or(Color::BLACK);
                        let mut n = with_id(ImportNode::new(ImportKind::Line))
                            .at(x1, y1).size((x2 - x1).abs().max(1.0), 1.0);
                        n.stroke = Some((Paint::Solid(stroke_c), attr_num(&attrs, "stroke-width").unwrap_or(1.0)));
                        apply_transform_attr(&mut n, &attrs);
                        if !self_closed { skip_element(lexer)?; }
                        parent.children.push(n);
                    }
                    "path" => {
                        let cmds = attr(&attrs, "d").map(parse_path_d).unwrap_or_default();
                        // bounds -> node w/h so hit testing & layout see the real size
                        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
                        for c in &cmds {
                            let pts: &[(f64, f64)] = match c {
                                PathCmd::MoveTo(x, y) | PathCmd::LineTo(x, y) => &[(*x, *y)],
                                PathCmd::CurveTo(a, b, cc, d, e, f) => &[(*a, *b), (*cc, *d), (*e, *f)],
                                PathCmd::Close => &[],
                            };
                            for (x, y) in pts { x0 = x0.min(*x); y0 = y0.min(*y); x1 = x1.max(*x); y1 = y1.max(*y); }
                        }
                        let (w, h) = if x1 > x0 { (x1 - x0, y1 - y0) } else { (0.0, 0.0) };
                        let mut n = with_id(ImportNode::new(ImportKind::Path { cmds }))
                            .size(w, h)
                            .fill(Paint::Solid(attr_fill(&attrs)));
                        if let (Some(sc), Some(sw)) = (attr(&attrs, "stroke").and_then(parse_hex_color), attr_num(&attrs, "stroke-width")) {
                            n.stroke = Some((Paint::Solid(sc), sw));
                        }
                        apply_transform_attr(&mut n, &attrs);
                        if !self_closed { skip_element(lexer)?; }
                        parent.children.push(n);
                    }
                    "text" => {
                        let size = attr_num(&attrs, "font-size").unwrap_or(16.0);
                        let mut n = with_id(ImportNode::new(ImportKind::Text { content: String::new(), size: None, font: None, line_height: None, letter_spacing: None, runs: vec![] }))
                            .at(attr_num(&attrs, "x").unwrap_or(0.0), attr_num(&attrs, "y").unwrap_or(0.0) - size * 0.8)
                            .size(10.0 * size, size * 1.25)
                            .fill(Paint::Solid(attr_fill(&attrs)));
                        apply_transform_attr(&mut n, &attrs);
                        if self_closed { parent.children.push(n); } else { pending_text_node = Some(n); }
                    }
                    "defs" | "style" | "clipPath" | "mask" | "linearGradient" | "radialGradient" | "symbol" => {
                        if !self_closed { skip_element(lexer)?; }
                    }
                    _ => { if !self_closed { skip_element(lexer)?; } }
                }
            }
        }
    }
}

fn skip_element(lexer: &mut XmlLexer) -> Result<(), String> {
    let mut depth = 1i32;
    loop {
        match lexer.next_tag()? {
            XmlTag::Open(..) => depth += 1,
            XmlTag::Close(_) => { depth -= 1; if depth == 0 { return Ok(()); } }
            XmlTag::Eof => return Ok(()),
            _ => {}
        }
    }
}

