use std::collections::HashMap;
use vello::kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Color, Fill, Gradient, Mix};
#[allow(unused_imports)]
use crate::*;

// ---------------------------------------------------------------- variables

/// Variables v2 (Phase 5.4): color/number/string/bool storage, aliases
/// (var -> var, cycle-limited), and color modes (e.g. "light"/"dark").
/// Lookup order for colors: alias chain -> active mode table -> base table.
#[derive(Debug, Default, Clone)]
pub struct Variables {
    /// P1: collection name per variable ("Primitives", "Semantic", ...).
    /// Unlisted variables belong to the implicit "Local" collection.
    pub collections: HashMap<String, String>,
    pub colors: HashMap<String, Color>,
    pub numbers: HashMap<String, f64>,
    pub strings: HashMap<String, String>,
    pub bools: HashMap<String, bool>,
    pub aliases: HashMap<String, String>,
    pub modes: HashMap<String, HashMap<String, Color>>,
    pub active_mode: Option<String>,
}
const MAX_ALIAS_DEPTH: u32 = 8;
impl Variables {
    fn resolve_name<'a>(&'a self, name: &'a str) -> &'a str {
        let mut cur = name;
        for _ in 0..MAX_ALIAS_DEPTH {
            match self.aliases.get(cur) { Some(next) => cur = next, None => break }
        }
        cur
    }
    pub fn color(&self, name: &str, fallback: Color) -> Color {
        let name = self.resolve_name(name);
        if let Some(mode) = &self.active_mode {
            if let Some(table) = self.modes.get(mode) {
                if let Some(c) = table.get(name) { return *c; }
            }
        }
        self.colors.get(name).copied().unwrap_or(fallback)
    }
    pub fn number(&self, name: &str, fallback: f64) -> f64 { self.numbers.get(self.resolve_name(name)).copied().unwrap_or(fallback) }
    pub fn string(&self, name: &str, fallback: &str) -> String { self.strings.get(self.resolve_name(name)).cloned().unwrap_or_else(|| fallback.to_string()) }
    pub fn boolean(&self, name: &str, fallback: bool) -> bool { self.bools.get(self.resolve_name(name)).copied().unwrap_or(fallback) }

    pub fn collection_of(&self, name: &str) -> &str {
        self.collections.get(name).map(String::as_str).unwrap_or("Local")
    }
    /// All (collection, name, kind) triples, sorted, for the variables UI.
    pub fn catalog(&self) -> Vec<(String, String, &'static str)> {
        let mut out = vec![];
        for k in self.colors.keys() { out.push((self.collection_of(k).to_string(), k.clone(), "color")); }
        for k in self.numbers.keys() { out.push((self.collection_of(k).to_string(), k.clone(), "number")); }
        for k in self.strings.keys() { out.push((self.collection_of(k).to_string(), k.clone(), "string")); }
        for k in self.bools.keys() { out.push((self.collection_of(k).to_string(), k.clone(), "bool")); }
        out.sort();
        out
    }
    pub fn mode_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.modes.keys().cloned().collect();
        v.sort();
        v
    }
}

pub fn paint_color(p: &Paint, vars: &Variables) -> Color {
    match p {
        Paint::Solid(c) => *c,
        Paint::Variable(n) => vars.color(n, Color::BLACK),
        Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } =>
            stops.first().map(|s| s.1).unwrap_or(Color::BLACK),
    }
}

pub fn paint_brush(p: &Paint, vars: &Variables) -> Brush {
    match p {
        Paint::Solid(c) => Brush::Solid(*c),
        Paint::Variable(n) => Brush::Solid(vars.color(n, Color::BLACK)),
        Paint::LinearGradient { start, end, stops } => Brush::Gradient(
            Gradient::new_linear((start.0, start.1), (end.0, end.1))
                .with_stops(stops.as_slice()),
        ),
        Paint::RadialGradient { center, radius, stops } => Brush::Gradient(
            Gradient::new_radial((center.0, center.1), *radius as f32)
                .with_stops(stops.as_slice()),
        ),
    }
}

/// Parses "#rrggbb" or "#rrggbbaa" into a Color.
pub fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#').unwrap_or(s);
    let (r, g, b, a) = match s.len() {
        6 => (u8::from_str_radix(&s[0..2], 16).ok()?, u8::from_str_radix(&s[2..4], 16).ok()?, u8::from_str_radix(&s[4..6], 16).ok()?, 255u8),
        8 => (u8::from_str_radix(&s[0..2], 16).ok()?, u8::from_str_radix(&s[2..4], 16).ok()?, u8::from_str_radix(&s[4..6], 16).ok()?, u8::from_str_radix(&s[6..8], 16).ok()?),
        _ => return None,
    };
    Some(Color::rgba8(r, g, b, a))
}
pub fn color_to_hex(c: Color) -> String {
    if c.a == 255 { format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b) } else { format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a) }
}

/// Typed instance overrides (Phase 5.3): an override value keyed by a node id
/// is either a hex color ("#12ab34") applied to that node's fill, or — new —
/// prefixed "text:" to replace a Text node's content.
pub fn effective_fill(node: &Node, overrides: &HashMap<String, String>, vars: &Variables) -> Color {
    if let Some(v) = overrides.get(&node.id) {
        if let Some(c) = parse_hex_color(v) { return c; }
    }
    paint_color(&node.fill, vars)
}
pub fn effective_brush(node: &Node, overrides: &HashMap<String, String>, vars: &Variables) -> Brush {
    if let Some(v) = overrides.get(&node.id) {
        if let Some(c) = parse_hex_color(v) { return Brush::Solid(c); }
    }
    paint_brush(&node.fill, vars)
}
pub fn effective_text<'a>(node: &'a Node, overrides: &'a HashMap<String, String>) -> Option<&'a str> {
    overrides.get(&node.id).and_then(|v| v.strip_prefix("text:"))
}

