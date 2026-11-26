//! Port of `src/hooks/output/tool-shape.ts`, switched.
//!
//! The shape of a built-in tool's output as `sdk-tools.d.ts` types it (Agent Plugins Extended J7.138:
//! a `PostToolUse` hook's `updatedToolOutput` is checked against the tool's output type), as data the
//! kit can test a value against. `scripts/tool-output-shapes.ts` writes the tables from the SDK's
//! file, here as the core's own JSON tables (`tool_shapes.rs` and `tool_shapes/`, read by
//! [`read_table`]); a test checks they are current.
//!
//! A primitive is named; `literal` is one value; `array` its element; `object` its fields by name,
//! a name ending in `?` an optional field, and any other key allowed, as the SDK's types are open;
//! `oneOf` its alternatives. A type the file declares as `unknown` takes any value.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::json_parse::json_parse;
use crate::js::{JsValue, key_order};

/// A primitive shape: a type's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrimitiveShape {
    /// A string.
    String,
    /// A number.
    Number,
    /// A boolean.
    Boolean,
    /// `null`.
    Null,
    /// Any value.
    Unknown,
}

/// The one value a `literal` shape holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LiteralValue {
    /// A string.
    Text(String),
    /// A number.
    Number(f64),
    /// A boolean.
    Bool(bool),
}

/// The shape of a value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Shape {
    /// A primitive.
    Primitive(PrimitiveShape),
    /// One value.
    Literal {
        /// The value.
        literal: LiteralValue,
    },
    /// An array of one shape.
    Array {
        /// The element's shape.
        array: Box<Shape>,
    },
    /// An object by field name; a name ending in `?` is optional.
    Object {
        /// The fields.
        object: IndexMap<String, Shape>,
    },
    /// Any of the alternatives.
    OneOf {
        /// The alternatives.
        #[serde(rename = "oneOf")]
        one_of: Vec<Shape>,
    },
}

/// The output shape of each built-in tool, by the tool's name.
pub type ShapeTable = IndexMap<String, Shape>;

/// A generated table's JSON text as its shapes. The script writes only tables that read (the
/// unit test below reads every one), so a text that does not is an empty table.
#[must_use]
pub fn read_table(text: &str) -> ShapeTable {
    json_parse(text)
        .ok()
        .and_then(|value| ShapeTable::deserialize(value).ok())
        .unwrap_or_default()
}

fn alternatives(shape: Option<&Shape>) -> Vec<Shape> {
    match shape {
        None => Vec::new(),
        Some(Shape::OneOf { one_of }) => one_of.clone(),
        Some(other) => vec![other.clone()],
    }
}

/// The tables as one; a tool in several (its alternatives split to keep each file short) has them all.
#[must_use]
pub fn join_tables(tables: &[ShapeTable]) -> ShapeTable {
    let mut joined = ShapeTable::new();
    for table in tables {
        for (tool, shape) in table {
            let next = if joined.contains_key(tool) {
                Shape::OneOf {
                    one_of: [alternatives(joined.get(tool)), alternatives(Some(shape))].concat(),
                }
            } else {
                shape.clone()
            };
            joined.insert(tool.clone(), next);
        }
    }
    joined
}

fn primitive_fits(shape: PrimitiveShape, value: &JsValue) -> bool {
    match shape {
        PrimitiveShape::String => value.is_string(),
        PrimitiveShape::Number => value.is_number(),
        PrimitiveShape::Boolean => value.is_boolean(),
        PrimitiveShape::Null => value.is_null(),
        PrimitiveShape::Unknown => true,
    }
}

const fn primitive_name(shape: PrimitiveShape) -> &'static str {
    match shape {
        PrimitiveShape::String => "string",
        PrimitiveShape::Number => "number",
        PrimitiveShape::Boolean => "boolean",
        PrimitiveShape::Null => "null",
        PrimitiveShape::Unknown => "unknown",
    }
}

fn describe(value: &JsValue) -> &'static str {
    match value {
        JsValue::Null => "null",
        JsValue::Array(_) => "array",
        JsValue::Undefined => "undefined",
        JsValue::Bool(_) => "boolean",
        JsValue::Number(_) => "number",
        JsValue::String(_) => "string",
        JsValue::Object(_) => "object",
    }
}

fn at(path: &str, why: &str) -> String {
    if path.is_empty() {
        why.to_owned()
    } else {
        format!("{path}: {why}")
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

const OPTIONAL: char = '?';

fn object_problem(fields: &IndexMap<String, Shape>, value: &JsValue, path: &str) -> Option<String> {
    let JsValue::Object(members) = value else {
        return Some(at(
            path,
            &format!("expected an object, received {}", describe(value)),
        ));
    };
    for written in key_order(fields.keys()) {
        let Some(shape) = fields.get(written) else {
            continue;
        };
        let optional = written.ends_with(OPTIONAL);
        let key = written.strip_suffix(OPTIONAL).unwrap_or(written);
        let own = members.get(key).filter(|found| !found.is_undefined());
        let Some(own) = own else {
            if optional {
                continue;
            }
            return Some(at(&join(path, key), "required"));
        };
        if let Some(problem) = shape_problem(shape, own, &join(path, key)) {
            return Some(problem);
        }
    }
    None
}

fn array_problem(element: &Shape, value: &JsValue, path: &str) -> Option<String> {
    let JsValue::Array(items) = value else {
        return Some(at(
            path,
            &format!("expected an array, received {}", describe(value)),
        ));
    };
    items
        .iter()
        .enumerate()
        .find_map(|(index, item)| shape_problem(element, item, &join(path, &index.to_string())))
}

fn literal_fits(literal: &LiteralValue, value: &JsValue) -> bool {
    match (literal, value) {
        (LiteralValue::Text(expected), JsValue::String(found)) => expected == found,
        (LiteralValue::Number(expected), JsValue::Number(found)) => expected == found,
        (LiteralValue::Bool(expected), JsValue::Bool(found)) => expected == found,
        (
            LiteralValue::Text(_) | LiteralValue::Number(_) | LiteralValue::Bool(_),
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::String(_)
            | JsValue::Array(_)
            | JsValue::Object(_),
        ) => false,
    }
}

fn literal_text(literal: &LiteralValue) -> String {
    match literal {
        LiteralValue::Text(text) => JsValue::from(text.as_str()).to_json_text(),
        LiteralValue::Number(number) => JsValue::from(*number).to_json_text(),
        LiteralValue::Bool(flag) => JsValue::from(*flag).to_json_text(),
    }
}

/// The first place `value` breaks `shape`, as `<path>: <why>`; `None` when it fits.
#[must_use]
pub fn shape_problem(shape: &Shape, value: &JsValue, path: &str) -> Option<String> {
    match shape {
        Shape::Primitive(primitive) => {
            if primitive_fits(*primitive, value) {
                None
            } else {
                Some(at(
                    path,
                    &format!(
                        "expected {}, received {}",
                        primitive_name(*primitive),
                        describe(value)
                    ),
                ))
            }
        }
        Shape::Literal { literal } => {
            if literal_fits(literal, value) {
                None
            } else {
                Some(at(path, &format!("expected {}", literal_text(literal))))
            }
        }
        Shape::Array { array } => array_problem(array, value, path),
        Shape::Object { object } => object_problem(object, value, path),
        Shape::OneOf { one_of } => {
            let fits = one_of
                .iter()
                .any(|one| shape_problem(one, value, path).is_none());
            if fits {
                None
            } else {
                Some(at(path, "matches none of the shapes the type allows"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tool_shapes::TOOL_SHAPES;
    use super::{Shape, read_table, shape_problem};
    use crate::js::JsValue;

    #[test]
    fn every_generated_table_reads_and_a_tool_split_over_two_holds_both_halves() {
        assert!(TOOL_SHAPES.len() > 40, "{} tools", TOOL_SHAPES.len());
        assert!(matches!(
            TOOL_SHAPES.get("Bash"),
            Some(Shape::Object { .. } | Shape::OneOf { .. })
        ));
        assert!(read_table("{").is_empty());
        let read = TOOL_SHAPES.get("Read").unwrap();
        assert!(shape_problem(read, &JsValue::from("text"), "").is_some());
    }
}

crate::registry::exports! {
    "hooks/output/tool-shape" {
        "shapeProblem" => ok |shape: Shape, value: JsValue, path: Option<String>| shape_problem(&shape, &value, path.as_deref().unwrap_or(""));
    }
}
