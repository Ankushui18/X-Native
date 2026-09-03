//! Component system 2.0 (P0).
//!
//! - Typed overrides: fill / text / visibility / opacity / instance-swap
//! - Component properties: text + boolean, bound to internal nodes
//! - Variants: "Button/Primary", "Button/Danger" — one component set,
//!   property-driven variant switching
//! - Nested components with per-instance override scoping (outer
//!   instance overrides its own subtree; inner instances own theirs)
//! - Detach: instance -> plain nodes (resolved, overrides applied)
//! - Dependency graph: which component uses which (cycle detection,
//!   update-impact queries)
//!
//! Overrides are stored as typed values keyed by target node id — the
//! legacy `HashMap<String, String>` ("#hex" / "text:") remains as a
//! serialization surface and is converted losslessly both ways.

use crate::{color_to_hex, parse_hex_color, Color, Node, NodeKind, Paint, Variables};
use std::collections::HashMap;

/// A typed per-node override carried by an Instance.
#[derive(Debug, Clone, PartialEq)]
pub enum OverrideValue {
    Fill(Color),
    Text(String),
    Visible(bool),
    Opacity(f32),
    /// Replace a nested INSTANCE's component with another component name.
    Swap(String),
    /// Numeric override (number property) — sets the target node's width.
    Number(f64),
}

impl OverrideValue {
    /// Encode into the legacy string form used by `.x` files and the
    /// renderer's override map.
    pub fn encode(&self) -> String {
        match self {
            OverrideValue::Fill(c) => color_to_hex(*c),
            OverrideValue::Text(t) => format!("text:{t}"),
            OverrideValue::Visible(v) => format!("visible:{v}"),
            OverrideValue::Opacity(o) => format!("opacity:{o}"),
            OverrideValue::Swap(c) => format!("swap:{c}"),
            OverrideValue::Number(n) => format!("num:{n}"),
        }
    }
    pub fn decode(s: &str) -> Option<OverrideValue> {
        if let Some(t) = s.strip_prefix("text:") {
            return Some(OverrideValue::Text(t.into()));
        }
        if let Some(v) = s.strip_prefix("visible:") {
            return v.parse().ok().map(OverrideValue::Visible);
        }
        if let Some(o) = s.strip_prefix("opacity:") {
            return o.parse().ok().map(OverrideValue::Opacity);
        }
        if let Some(c) = s.strip_prefix("swap:") {
            return Some(OverrideValue::Swap(c.into()));
        }
        if let Some(n) = s.strip_prefix("num:") {
            return n.parse().ok().map(OverrideValue::Number);
        }
        parse_hex_color(s).map(OverrideValue::Fill)
    }
}

/// Typed view over an instance's override map.
pub fn typed_overrides(node: &Node) -> HashMap<String, OverrideValue> {
    node.overrides
        .iter()
        .filter_map(|(k, v)| OverrideValue::decode(v).map(|ov| (k.clone(), ov)))
        .collect()
}

pub fn set_override(node: &mut Node, target: &str, value: OverrideValue) {
    node.overrides.insert(target.into(), value.encode());
}

/// Reset every override on an instance (Figma "reset overrides"). Slot
/// content lives in the instance's children, so it is kept.
pub fn reset_overrides(instance: &mut Node) {
    instance.overrides.clear();
}

// ------------------------------------------------------------- properties

/// Phase P0: Component property types for designer-facing component properties
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentPropertyType {
    Boolean {
        default: bool,
    },
    Text {
        default: String,
    },
    InstanceSwap {
        allowed_components: Vec<String>,
        default: Option<String>,
    },
    Color {
        default: Color,
    },
    Number {
        default: f64,
        min: Option<f64>,
        max: Option<f64>,
    },
    /// Slot insertion point (see [`ComponentProp::Slot`]).
    Slot {
        default: Option<String>,
    },
}

/// Phase P0: A designer-facing component property
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentProperty {
    pub name: String,
    pub id: String, // unique within component
    pub prop_type: ComponentPropertyType,
    pub preferred_input: Option<String>, // UI hint
}

/// Phase P0: Property binding connects a property to a target node
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyBinding {
    pub property_id: String,
    pub target_node_id: String,
    pub target_property: String, // "visible", "text", "fill", etc.
}

/// A designer-facing component property (component properties).
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentProp {
    /// Text property: binds a property name to a text node id inside the master.
    Text {
        name: String,
        target: String,
        default: String,
    },
    /// Boolean property: toggles visibility of a node inside the master.
    Bool {
        name: String,
        target: String,
        default: bool,
    },
    /// Instance-swap property: swaps a nested instance's component.
    Swap {
        name: String,
        target: String,
        default: String,
    },
    /// Number property: binds a numeric value to the target node's width.
    Number {
        name: String,
        target: String,
        default: f64,
        min: Option<f64>,
        max: Option<f64>,
    },
    /// Slot property (Figma slots, 2024): an insertion point inside the
    /// master. `target` is the anchor node id — when an instance carries
    /// content for this slot, the anchor subtree is replaced by it; with
    /// no content, `default` (a component name) fills the anchor instead.
    Slot {
        name: String,
        target: String,
        default: Option<String>,
    },
}

impl ComponentProp {
    /// The property's designer-facing name.
    pub fn name(&self) -> &str {
        match self {
            ComponentProp::Text { name, .. }
            | ComponentProp::Bool { name, .. }
            | ComponentProp::Swap { name, .. }
            | ComponentProp::Number { name, .. }
            | ComponentProp::Slot { name, .. } => name,
        }
    }
}

/// Component property definitions live per master, keyed by component name.
#[derive(Debug, Clone, Default)]
pub struct PropRegistry {
    pub props: HashMap<String, Vec<ComponentProp>>,
}

impl PropRegistry {
    /// Apply a property assignment to an instance as typed overrides.
    pub fn apply(
        &self,
        component: &str,
        instance: &mut Node,
        prop_name: &str,
        value: &str,
    ) -> bool {
        let Some(props) = self.props.get(component) else {
            return false;
        };
        for p in props {
            match p {
                ComponentProp::Text { name, target, .. } if name == prop_name => {
                    set_override(instance, target, OverrideValue::Text(value.into()));
                    return true;
                }
                ComponentProp::Bool { name, target, .. } if name == prop_name => {
                    if let Ok(b) = value.parse::<bool>() {
                        set_override(instance, target, OverrideValue::Visible(b));
                        return true;
                    }
                }
                ComponentProp::Swap { name, target, .. } if name == prop_name => {
                    set_override(instance, target, OverrideValue::Swap(value.into()));
                    return true;
                }
                ComponentProp::Number { name, target, .. } if name == prop_name => {
                    if let Ok(n) = value.parse::<f64>() {
                        set_override(instance, target, OverrideValue::Number(n));
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
}

// ------------------------------------------------------------------ slots

/// Binding key tagging an instance child as slot content
/// (`bindings["slot"] = <slot name>`).
pub const SLOT_TAG: &str = "slot";

/// The subtree an instance carries for `slot`, if any.
pub fn slot_content<'a>(instance: &'a Node, slot: &str) -> Option<&'a Node> {
    instance
        .children
        .iter()
        .find(|c| c.bindings.get(SLOT_TAG).map(|s| s.as_str()) == Some(slot))
}

/// Bind `content` into the instance's `slot`, replacing any previous
/// content for that slot. Stored as a tagged child of the instance, so
/// it serializes with the instance and renders in place of the master's
/// anchor node.
pub fn set_slot_content(instance: &mut Node, slot: &str, content: Node) {
    instance
        .children
        .retain(|c| c.bindings.get(SLOT_TAG).map(|s| s.as_str()) != Some(slot));
    let mut c = content;
    c.bindings.insert(SLOT_TAG.into(), slot.into());
    instance.children.push(c);
}

/// Remove the instance's content for `slot` (the anchor's default, if
/// any, applies again).
pub fn clear_slot_content(instance: &mut Node, slot: &str) {
    instance
        .children
        .retain(|c| c.bindings.get(SLOT_TAG).map(|s| s.as_str()) != Some(slot));
}

/// Slot-aware children for an instance render: anchors declared by the
/// master's `Slot` props are substituted with the instance's slot
/// content (or an Instance of the slot's default component).
///
/// Returns `None` when the master has no slots — the zero-clone fast
/// path; renderers should then lower `def.children` directly.
pub fn resolve_slots(def: &Node, instance: &Node) -> Option<Vec<Node>> {
    if !def
        .props
        .iter()
        .any(|p| matches!(p, ComponentProp::Slot { .. }))
    {
        return None;
    }
    // anchor node id -> (slot name, default component)
    let mut anchors: HashMap<String, (String, Option<String>)> = HashMap::new();
    for p in &def.props {
        if let ComponentProp::Slot {
            name,
            target,
            default,
        } = p
        {
            anchors.insert(target.clone(), (name.clone(), default.clone()));
        }
    }
    let content: HashMap<&str, &Node> = instance
        .children
        .iter()
        .filter_map(|c| c.bindings.get(SLOT_TAG).map(|s| (s.as_str(), c)))
        .collect();
    Some(
        def.children
            .iter()
            .map(|c| substitute_slots(c, &anchors, &content))
            .collect(),
    )
}

fn substitute_slots(
    n: &Node,
    anchors: &HashMap<String, (String, Option<String>)>,
    content: &HashMap<&str, &Node>,
) -> Node {
    if let Some((slot, default)) = anchors.get(&n.id) {
        if let Some(&c) = content.get(slot.as_str()) {
            return c.clone();
        }
        if let Some(d) = default {
            // no instance content: the default component fills the anchor
            let mut inst = Node::instance(&n.id, d, n.transform.x, n.transform.y, n.w, n.h);
            inst.name = n.name.clone();
            return inst;
        }
        // no content, no default: keep the placeholder anchor as-is
    }
    let mut out = n.clone();
    out.children = n
        .children
        .iter()
        .map(|c| substitute_slots(c, anchors, content))
        .collect();
    out
}

// ---------------------------------------------------------------- variants

/// Variant identity: "Set/VariantName". Components named with a slash are
/// variant members of a set ("Button/Primary", "Button/Danger").
pub fn variant_set(component_name: &str) -> Option<(&str, &str)> {
    component_name.split_once('/')
}

/// All variants of a set present in the document.
pub fn variants_of<'a>(root: &'a Node, set: &str) -> Vec<&'a str> {
    let mut out = vec![];
    fn walk<'a>(n: &'a Node, set: &str, out: &mut Vec<&'a str>) {
        if let NodeKind::Component { name } = &n.kind {
            if let Some((s, _)) = variant_set(name) {
                if s == set {
                    out.push(name.as_str());
                }
            }
        }
        for c in &n.children {
            walk(c, set, out);
        }
    }
    walk(root, set, &mut out);
    out.sort();
    out
}

/// Switch an instance to a different variant of the same set. Keeps
/// overrides (they target ids shared across variants by convention).
pub fn switch_variant(instance: &mut Node, to_variant: &str) -> bool {
    if let NodeKind::Instance { component } = &mut instance.kind {
        let same_set = match (variant_set(component), variant_set(to_variant)) {
            (Some((a, _)), Some((b, _))) => a == b,
            _ => false,
        };
        if same_set {
            *component = to_variant.to_string();
            return true;
        }
    }
    false
}

// ------------------------------------------------------------------ detach

/// Find a component master by name anywhere in the tree.
pub fn find_master<'a>(root: &'a Node, name: &str) -> Option<&'a Node> {
    if let NodeKind::Component { name: n } = &root.kind {
        if n == name {
            return Some(root);
        }
    }
    root.children.iter().find_map(|c| find_master(c, name))
}

/// Detach an instance: resolve the master's children WITH the instance's
/// overrides applied, and return them re-based at the instance's position
/// wrapped in a Group. Nested instances stay instances (standard behavior).
pub fn detach_instance(root: &Node, instance: &Node, vars: &Variables) -> Option<Node> {
    let NodeKind::Instance { component } = &instance.kind else {
        return None;
    };
    let master = find_master(root, component)?;
    let ovr = typed_overrides(instance);
    let mut group = Node::group(&format!("{}-detached", instance.id), instance.w, instance.h);
    group.transform = instance.transform;
    let resolved = resolve_slots(master, instance);
    let kids: &[Node] = resolved.as_deref().unwrap_or(&master.children);
    for child in kids {
        let mut c = child.clone();
        apply_overrides_deep(&mut c, &ovr, vars);
        group.children.push(c);
    }
    Some(group)
}

fn apply_overrides_deep(node: &mut Node, ovr: &HashMap<String, OverrideValue>, vars: &Variables) {
    if let Some(v) = ovr.get(&node.id) {
        match v {
            OverrideValue::Fill(c) => node.fill = Paint::Solid(*c),
            OverrideValue::Text(t) => {
                if let NodeKind::Text { text } = &mut node.kind {
                    *text = t.clone();
                    node.text_runs.clear();
                }
            }
            OverrideValue::Visible(b) => node.visible = *b,
            OverrideValue::Opacity(o) => node.opacity = *o,
            OverrideValue::Swap(c) => {
                if let NodeKind::Instance { component } = &mut node.kind {
                    *component = c.clone();
                }
            }
            OverrideValue::Number(n) => {
                node.w = *n;
            }
        }
    }
    let _ = vars;
    // nested instances keep their own override scope: do not descend into them
    if matches!(node.kind, NodeKind::Instance { .. }) {
        return;
    }
    for c in &mut node.children {
        apply_overrides_deep(c, ovr, vars);
    }
}

// -------------------------------------------------------- dependency graph

/// Component dependency graph: master name -> component names it instances.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    pub edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    pub fn build(root: &Node) -> Self {
        let mut g = Self::default();
        fn masters(n: &Node, g: &mut DependencyGraph) {
            if let NodeKind::Component { name } = &n.kind {
                let mut deps = vec![];
                fn inner(c: &Node, deps: &mut Vec<String>) {
                    if let NodeKind::Instance { component } = &c.kind {
                        deps.push(component.clone());
                    }
                    for ch in &c.children {
                        inner(ch, deps);
                    }
                }
                for c in &n.children {
                    inner(c, &mut deps);
                }
                deps.sort();
                deps.dedup();
                g.edges.insert(name.clone(), deps);
            }
            for c in &n.children {
                masters(c, g);
            }
        }
        masters(root, &mut g);
        g
    }

    /// Every component whose render depends (transitively) on `name` —
    /// i.e. what must re-render when `name`'s master is edited.
    pub fn dependents_of(&self, name: &str) -> Vec<String> {
        let mut out = vec![];
        for (m, deps) in &self.edges {
            if self.reaches(m, name, &mut vec![]) && m != name && !deps.is_empty() {
                out.push(m.clone());
            }
        }
        out.sort();
        out
    }

    fn reaches(&self, from: &str, to: &str, seen: &mut Vec<String>) -> bool {
        if seen.iter().any(|s| s == from) {
            return false;
        }
        seen.push(from.to_string());
        let Some(deps) = self.edges.get(from) else {
            return false;
        };
        deps.iter().any(|d| d == to || self.reaches(d, to, seen))
    }

    /// True if adding an instance of `child` inside master `parent`
    /// would create a cycle.
    pub fn would_cycle(&self, parent: &str, child: &str) -> bool {
        parent == child || self.reaches(child, parent, &mut vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peniko::Color;

    fn master_with_slot() -> Node {
        // Card master: label text + a slot anchored on the "body" frame
        let mut m = Node::component("Card", "Card", 200.0, 100.0);
        m.children
            .push(Node::text("title", 8.0, 8.0, 180.0, 16.0, "Card"));
        m.children.push(Node::frame("body", 184.0, 60.0));
        m.props.push(ComponentProp::Slot {
            name: "Content".into(),
            target: "body".into(),
            default: None,
        });
        m
    }

    #[test]
    fn slot_content_set_get_clear_roundtrip() {
        let mut inst = Node::instance("i1", "Card", 0.0, 0.0, 200.0, 100.0);
        assert!(slot_content(&inst, "Content").is_none());
        let mut c = Node::rect("badge", 0.0, 0.0, 60.0, 20.0, Color::from_rgb8(0xff, 0, 0));
        c.name = "Badge".into();
        set_slot_content(&mut inst, "Content", c);
        assert_eq!(slot_content(&inst, "Content").unwrap().id, "badge");
        assert_eq!(inst.children.len(), 1);
        // replacing swaps, doesn't accumulate
        let c2 = Node::rect("pill", 0.0, 0.0, 40.0, 12.0, Color::WHITE);
        set_slot_content(&mut inst, "Content", c2);
        assert_eq!(inst.children.len(), 1);
        assert_eq!(slot_content(&inst, "Content").unwrap().id, "pill");
        clear_slot_content(&mut inst, "Content");
        assert!(slot_content(&inst, "Content").is_none());
        assert!(inst.children.is_empty());
    }

    #[test]
    fn resolve_slots_substitutes_content_at_anchor() {
        let m = master_with_slot();
        let mut inst = Node::instance("i1", "Card", 0.0, 0.0, 200.0, 100.0);
        set_slot_content(
            &mut inst,
            "Content",
            Node::rect("badge", 0.0, 0.0, 60.0, 20.0, Color::from_rgb8(0xff, 0, 0)),
        );
        let kids = resolve_slots(&m, &inst).expect("slots resolved");
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].id, "title");
        assert_eq!(kids[1].id, "badge", "anchor replaced by slot content");
        // no instance content: placeholder anchor kept
        let plain = Node::instance("i2", "Card", 0.0, 0.0, 200.0, 100.0);
        let kids2 = resolve_slots(&m, &plain).expect("slots resolved");
        assert_eq!(kids2[1].id, "body");
    }

    #[test]
    fn resolve_slots_default_component_fills_anchor() {
        // Badge master + Card master whose slot defaults to "Badge"
        let mut m = master_with_slot();
        if let ComponentProp::Slot { default, .. } = m.props.last_mut().unwrap() {
            *default = Some("Badge".into());
        }
        let plain = Node::instance("i2", "Card", 0.0, 0.0, 200.0, 100.0);
        let kids = resolve_slots(&m, &plain).expect("slots resolved");
        assert!(matches!(&kids[1].kind, NodeKind::Instance { component } if component == "Badge"));
        // instance content still wins over the default
        let mut inst = Node::instance("i1", "Card", 0.0, 0.0, 200.0, 100.0);
        set_slot_content(
            &mut inst,
            "Content",
            Node::rect("x", 0.0, 0.0, 10.0, 10.0, Color::WHITE),
        );
        let kids2 = resolve_slots(&m, &inst).expect("slots resolved");
        assert_eq!(kids2[1].id, "x");
    }

    #[test]
    fn resolve_slots_fast_path_without_slots() {
        let mut m = Node::component("Plain", "Plain", 10.0, 10.0);
        m.children
            .push(Node::rect("a", 0.0, 0.0, 5.0, 5.0, Color::WHITE));
        let inst = Node::instance("i", "Plain", 0.0, 0.0, 10.0, 10.0);
        assert!(resolve_slots(&m, &inst).is_none());
    }

    #[test]
    fn detach_instance_applies_slots_and_overrides() {
        let mut root = Node::frame("root", 800.0, 600.0);
        root.children.push(master_with_slot());
        let mut inst = Node::instance("i1", "Card", 100.0, 50.0, 200.0, 100.0);
        set_override(&mut inst, "title", OverrideValue::Text("Hi".into()));
        set_slot_content(
            &mut inst,
            "Content",
            Node::rect("badge", 0.0, 0.0, 60.0, 20.0, Color::from_rgb8(0xff, 0, 0)),
        );
        root.children.push(inst);

        let inst_ref = find(&root, "i1").unwrap();
        let group = detach_instance(&root, inst_ref, &Variables::default()).expect("detach");
        assert_eq!(group.id, "i1-detached");
        assert_eq!(group.children.len(), 2);
        let title = &group.children[0];
        assert!(matches!(&title.kind, NodeKind::Text { text } if text == "Hi"));
        assert_eq!(
            group.children[1].id, "badge",
            "slot content detached in place"
        );
    }

    #[test]
    fn reset_overrides_keeps_slot_content() {
        let mut inst = Node::instance("i1", "Card", 0.0, 0.0, 200.0, 100.0);
        set_override(&mut inst, "title", OverrideValue::Text("Hi".into()));
        set_slot_content(
            &mut inst,
            "Content",
            Node::rect("x", 0.0, 0.0, 10.0, 10.0, Color::WHITE),
        );
        reset_overrides(&mut inst);
        assert!(inst.overrides.is_empty());
        assert_eq!(inst.children.len(), 1, "slot content survives reset");
    }

    #[test]
    fn slot_prop_name_and_typed_twin() {
        let p = ComponentProp::Slot {
            name: "Content".into(),
            target: "body".into(),
            default: Some("Badge".into()),
        };
        assert_eq!(p.name(), "Content");
        let typed = ComponentPropertyType::Slot {
            default: Some("Badge".into()),
        };
        assert!(matches!(typed, ComponentPropertyType::Slot { .. }));
    }

    fn find<'a>(n: &'a Node, id: &str) -> Option<&'a Node> {
        if n.id == id {
            return Some(n);
        }
        n.children.iter().find_map(|c| find(c, id))
    }
}
