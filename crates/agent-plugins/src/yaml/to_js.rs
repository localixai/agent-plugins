//! A document as a value (`Document.prototype.toJS`, `dist/nodes/toJS.js`,
//! `dist/nodes/addPairToJSMap.js`, the `toJSON` of the nodes and `Alias.prototype.resolve`): maps
//! as objects, an alias as its anchor's value, at most 100 aliases' worth of copies.
//!
//! JavaScript gives every alias of an anchor the same object, so an alias inside its own anchor
//! (`&a [*a]`) makes a cycle. A value here has no cycles: such an alias is the anchor's
//! collection as it was when it began, empty.

use std::collections::HashSet;

use indexmap::IndexMap;

use super::nodes::{Body, Document, Key, Node, Pair};
use super::stringify::stringify_key_node;
use crate::js::{JsObject, JsValue, in_property_order, number_to_string};

/// `maxAliasCount`'s default.
const MAX_ALIAS_COUNT: u64 = 100;

/// What `ctx.anchors` holds for an anchored node.
#[derive(Debug, Clone)]
struct Anchor {
    alias_count: u64,
    count: u64,
    res: Option<JsValue>,
}

struct ToJs<'d> {
    doc: &'d Document,
    anchors: IndexMap<usize, Anchor>,
    /// The anchored nodes and aliases in document order (`aliasResolveCache`).
    nodes: Option<Vec<&'d Node>>,
}

/// `doc.toJS()`: the document's value, or the message of what the package throws.
///
/// # Errors
///
/// An alias without an anchor before it, or too many aliases.
pub(crate) fn to_js(doc: &Document) -> Result<JsValue, String> {
    let mut ctx = ToJs {
        doc,
        anchors: IndexMap::new(),
        nodes: None,
    };
    match &doc.contents {
        Some(contents) => ctx.value(contents),
        None => Ok(JsValue::Null),
    }
}

/// `visit(doc, { Node })`: every node, each before what it holds, a pair's key before its value.
fn visit<'d>(node: &'d Node, out: &mut Vec<&'d Node>) {
    out.push(node);
    match &node.body {
        Body::Map { items, .. } => {
            for Pair { key, value } in items {
                if let Key::Node(key) = key {
                    visit(key, out);
                }
                if let Some(value) = value {
                    visit(value, out);
                }
            }
        }
        Body::Seq { items, .. } => {
            for item in items {
                visit(item, out);
            }
        }
        Body::Scalar(_) | Body::Alias(_) => {}
    }
}

/// The anchored nodes and aliases of a document, in order.
fn anchored(doc: &Document) -> Vec<&Node> {
    let mut all = Vec::new();
    if let Some(contents) = &doc.contents {
        visit(contents, &mut all);
    }
    all.retain(|node| matches!(node.body, Body::Alias(_)) || node.has_anchor());
    all
}

/// The last node before `alias` anchored as `source`.
fn find<'d>(nodes: &[&'d Node], alias: &Node, source: &str) -> Option<&'d Node> {
    let mut found = None;
    for node in nodes {
        if node.id == alias.id {
            break;
        }
        if node.anchor.as_deref() == Some(source) {
            found = Some(*node);
        }
    }
    found
}

/// The value an empty collection of a node's kind starts as (`ctx.onCreate`).
fn created(node: &Node) -> Option<JsValue> {
    match node.body {
        Body::Map { .. } => Some(JsValue::Object(JsObject::new())),
        Body::Seq { .. } => Some(JsValue::Array(Vec::new())),
        Body::Scalar(_) | Body::Alias(_) => None,
    }
}

impl<'d> ToJs<'d> {
    /// `toJS(value, arg, ctx)` of a node.
    fn value(&mut self, node: &'d Node) -> Result<JsValue, String> {
        if !node.has_anchor() {
            return self.node_json(node);
        }
        self.anchors.insert(
            node.id,
            Anchor {
                alias_count: 0,
                count: 1,
                res: created(node),
            },
        );
        let res = self.node_json(node)?;
        if let Some(anchor) = self.anchors.get_mut(&node.id) {
            anchor.res = Some(res.clone());
        }
        Ok(res)
    }

    /// The node's `toJSON(arg, ctx)`.
    fn node_json(&mut self, node: &'d Node) -> Result<JsValue, String> {
        match &node.body {
            Body::Scalar(scalar) => Ok(scalar.value.clone()),
            Body::Seq { items, .. } => items
                .iter()
                .map(|item| self.value(item))
                .collect::<Result<_, _>>()
                .map(JsValue::Array),
            Body::Map { items, .. } => {
                let mut map = JsObject::new();
                for pair in items {
                    self.add_pair(&mut map, pair)?;
                }
                Ok(JsValue::Object(in_property_order(map)))
            }
            Body::Alias(source) => self.alias(node, source),
        }
    }

    /// `addPairToJSMap` for an object: the key as its text, a later one over an earlier one in
    /// its place.
    fn add_pair(&mut self, map: &mut JsObject, pair: &'d Pair) -> Result<(), String> {
        let (js_key, key_node) = match &pair.key {
            Key::Node(key) => (self.value(key)?, Some(&**key)),
            Key::Text(text) => (JsValue::String(text.clone()), None),
        };
        let string_key = self.stringify_key(key_node, &js_key)?;
        let value = match &pair.value {
            Some(value) => self.value(value)?,
            None => JsValue::Null,
        };
        map.insert(string_key, value);
        Ok(())
    }

    /// `stringifyKey`.
    fn stringify_key(&self, key: Option<&Node>, js_key: &JsValue) -> Result<String, String> {
        match js_key {
            JsValue::Null => Ok(String::new()),
            JsValue::String(text) => Ok(text.clone()),
            JsValue::Number(number) => Ok(number_to_string(*number)),
            JsValue::Bool(flag) => Ok(flag.to_string()),
            JsValue::Undefined => Ok("undefined".to_owned()),
            JsValue::Array(_) | JsValue::Object(_) => match key {
                Some(key) => {
                    let anchors: HashSet<String> = self
                        .anchors
                        .keys()
                        .filter_map(|id| self.anchor_name(*id))
                        .collect();
                    stringify_key_node(self.doc, key, anchors)
                }
                None => Ok(js_key.to_json_text()),
            },
        }
    }

    /// The anchor of the node with this id.
    fn anchor_name(&self, id: usize) -> Option<String> {
        anchored(self.doc)
            .into_iter()
            .find(|node| node.id == id)
            .and_then(|node| node.anchor.clone())
    }

    /// `Alias.prototype.toJSON` with `resolve`: the anchor's value, counted.
    fn alias(&mut self, alias: &Node, source: &str) -> Result<JsValue, String> {
        let doc = self.doc;
        let nodes = self.nodes.get_or_insert_with(|| anchored(doc));
        let Some(found) = find(nodes, alias, source) else {
            return Err(format!(
                "Unresolved alias (the anchor must be set before the alias): {source}"
            ));
        };
        if !self.anchors.contains_key(&found.id) {
            // Resolve anchors for `Node.prototype.toJS()`.
            self.value(found)?;
        }
        let alias_count = match self.anchors.get(&found.id) {
            Some(anchor) if anchor.alias_count == 0 => self.alias_count(found),
            Some(anchor) => anchor.alias_count,
            None => 0,
        };
        let Some(anchor) = self.anchors.get_mut(&found.id) else {
            return Err("This should not happen: Alias anchor was not resolved?".to_owned());
        };
        anchor.count += 1;
        anchor.alias_count = alias_count;
        if anchor.count.saturating_mul(anchor.alias_count) > MAX_ALIAS_COUNT {
            return Err("Excessive alias count indicates a resource exhaustion attack".to_owned());
        }
        anchor
            .res
            .clone()
            .ok_or_else(|| "This should not happen: Alias anchor was not resolved?".to_owned())
    }

    /// `getAliasCount`: how many copies a node's aliases make.
    fn alias_count(&self, node: &Node) -> u64 {
        match &node.body {
            Body::Alias(source) => {
                let nodes = anchored(self.doc);
                find(&nodes, node, source)
                    .and_then(|found| self.anchors.get(&found.id))
                    .map_or(0, |anchor| anchor.count.saturating_mul(anchor.alias_count))
            }
            Body::Seq { items, .. } => items
                .iter()
                .map(|item| self.alias_count(item))
                .max()
                .unwrap_or(0),
            Body::Map { items, .. } => items
                .iter()
                .map(|Pair { key, value }| {
                    let key = match key {
                        Key::Node(key) => self.alias_count(key),
                        Key::Text(_) => 1,
                    };
                    key.max(value.as_ref().map_or(1, |value| self.alias_count(value)))
                })
                .max()
                .unwrap_or(0),
            Body::Scalar(_) => 1,
        }
    }
}
