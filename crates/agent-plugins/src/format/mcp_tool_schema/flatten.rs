//! The flattening of a root-level combinator (`flatten` and its helpers of
//! `src/format/mcp-tool-schema.ts`), split from `mcp_tool_schema.rs` to keep each file within
//! RS5's 300 lines; the functions are in the TypeScript's order.

use crate::js::{JsObject, JsValue, in_property_order, json_stringify, property_order};
use crate::protocol::is_record::is_record;

/// The three combinators, in the order they are read.
pub(super) const COMBINATORS: [&str; 3] = ["allOf", "anyOf", "oneOf"];
pub(super) fn has_root_combinator(schema: &JsObject) -> bool {
    COMBINATORS
        .iter()
        .any(|keyword| schema.contains_key(*keyword))
}

/// A group of parameters named together: one branch of an `anyOf` or a `oneOf`.
pub(super) struct Group {
    keyword: &'static str,
    required: Vec<String>,
}

pub(super) struct Flat {
    pub(super) properties: JsObject,
    pub(super) required: Vec<String>,
    pub(super) groups: Vec<Group>,
}

fn stringified(value: &JsValue) -> Option<String> {
    json_stringify(value)
}

/// A property two branches define differently keeps both, nested where the API accepts a combinator.
fn merged_property(existing: &JsValue, incoming: &JsValue) -> JsValue {
    if stringified(existing) == stringified(incoming) {
        return existing.clone();
    }
    let options: Vec<JsValue> = match existing.get("anyOf").and_then(JsValue::as_array) {
        Some(any_of) if is_record(existing) => any_of.clone(),
        Some(_) | None => vec![existing.clone()],
    };
    let seen = options
        .iter()
        .any(|each| stringified(each) == stringified(incoming));
    let mut chosen = options;
    if !seen {
        chosen.push(incoming.clone());
    }
    JsValue::from([("anyOf", JsValue::Array(chosen))])
}

fn merge_properties(into: &mut JsObject, from: &JsObject) {
    for (name, definition) in property_order(from) {
        let value = match into.get(name) {
            Some(existing) => merged_property(existing, definition),
            None => definition.clone(),
        };
        into.insert(name.clone(), value);
    }
}

fn required_of(schema: &JsObject) -> Vec<String> {
    schema
        .get("required")
        .and_then(JsValue::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The branch as an object schema, or why it cannot be folded into one. A branch with no `type` is
/// read as an object schema, as one that says `object` is.
fn object_branch(branch: &JsValue) -> Result<&JsObject, &'static str> {
    let JsValue::Object(members) = branch else {
        return Err("a branch is not a schema object");
    };
    let type_is_object = match members.get("type") {
        None | Some(JsValue::Undefined) => true,
        Some(declared) => declared.as_str() == Some("object"),
    };
    if !type_is_object {
        return Err("a branch is not an object schema");
    }
    if members.contains_key("$ref") {
        Err("a branch is a $ref, which cannot be merged")
    } else {
        Ok(members)
    }
}

/// One strategy per combinator (K1): what a branch contributes to the flattened schema.
fn merge(into: &mut Flat, branch: Flat, keyword: &str) {
    merge_properties(&mut into.properties, &branch.properties);
    match keyword {
        "allOf" => {
            into.required.extend(branch.required);
            into.groups.extend(branch.groups);
        }
        "anyOf" | "oneOf" => {
            into.groups.push(Group {
                keyword: if keyword == "anyOf" { "anyOf" } else { "oneOf" },
                required: branch.required,
            });
            into.groups.extend(branch.groups);
        }
        _ => {}
    }
}

pub(super) fn flatten(schema: &JsObject) -> Result<Flat, String> {
    let mut flat = Flat {
        properties: match schema.get("properties") {
            Some(JsValue::Object(properties)) => properties.clone(),
            Some(_) | None => JsObject::new(),
        },
        required: required_of(schema),
        groups: Vec::new(),
    };
    for keyword in COMBINATORS {
        let Some(branches) = schema.get(keyword) else {
            continue;
        };
        let Some(list) = branches.as_array().filter(|items| !items.is_empty()) else {
            return Err(format!("{keyword} is not a non-empty list of schemas"));
        };
        for branch in list {
            let object = object_branch(branch).map_err(str::to_owned)?;
            let inner = flatten(object)?;
            merge(&mut flat, inner, keyword);
        }
    }
    flat.properties = in_property_order(flat.properties);
    Ok(flat)
}

fn group_intro(keyword: &str) -> &'static str {
    if keyword == "anyOf" {
        "Provide the parameters of at least one of these groups:"
    } else {
        "Provide the parameters of exactly one of these groups:"
    }
}

/// The sentence prepended to the description (judgment, spec/judgments.md: the docs say it "tells Claude
/// which parameter groups belong together", not what it says). A group with no required parameter
/// names nothing, so with none left there is no sentence.
pub(super) fn group_sentence(groups: &[Group]) -> Option<String> {
    let sentences: Vec<String> = ["anyOf", "oneOf"]
        .into_iter()
        .filter_map(|keyword| {
            let named: Vec<String> = groups
                .iter()
                .filter(|group| group.keyword == keyword && !group.required.is_empty())
                .map(|group| format!("({})", group.required.join(", ")))
                .collect();
            if named.is_empty() {
                None
            } else {
                Some(format!("{} {}.", group_intro(keyword), named.join(" or ")))
            }
        })
        .collect();
    if sentences.is_empty() {
        None
    } else {
        Some(sentences.join(" "))
    }
}
