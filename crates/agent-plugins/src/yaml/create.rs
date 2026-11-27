//! A value as document nodes (`Document.prototype.createNode` and `dist/doc/createNode.js` with
//! `YAMLMap.from` and `YAMLSeq.from`), as the writer creates them with
//! `{ aliasDuplicateObjects: false }`: an object's members in property order without the
//! `undefined` ones, an array's items, any other value a scalar.

use super::nodes::{Body, Key, Node, Pair};
use super::{MAX_DEPTH, STACK_EXCEEDED};
use crate::js::{JsValue, property_order};

/// `createNode(value)`.
///
/// # Errors
///
/// A value nested deeper than `MAX_DEPTH`, where the package runs out of stack.
pub(crate) fn create_node(value: &JsValue) -> Result<Node, String> {
    create_at(value, 0)
}

/// `createNode` of a value `depth` collections down.
fn create_at(value: &JsValue, depth: usize) -> Result<Node, String> {
    let nested = |value: &JsValue| create_at(value, depth + 1);
    if matches!(value, JsValue::Array(_) | JsValue::Object(_)) && depth >= MAX_DEPTH {
        return Err(STACK_EXCEEDED.to_owned());
    }
    Ok(match value {
        JsValue::Array(items) => Node::new(Body::Seq {
            items: items.iter().map(nested).collect::<Result<_, _>>()?,
            flow: false,
        }),
        JsValue::Object(members) => Node::new(Body::Map {
            items: property_order(members)
                .into_iter()
                .filter(|(_, value)| !value.is_undefined())
                .map(|(key, value)| {
                    Ok(Pair {
                        key: Key::Node(Box::new(Node::scalar(JsValue::String(key.clone())))),
                        value: Some(nested(value)?),
                    })
                })
                .collect::<Result<_, String>>()?,
            flow: false,
        }),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_) => Node::scalar(value.clone()),
    })
}
