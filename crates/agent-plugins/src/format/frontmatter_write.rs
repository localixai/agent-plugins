//! Port of `src/format/frontmatter-write.ts`, switched.
//!
//! The writer of a Markdown file's frontmatter, the inverse of `parse_frontmatter`: what it writes
//! reads back to the same `fields` and the same `body` (tests/format/frontmatter-write.test.ts).
//! The YAML is written as the package's `yaml` writes it (`crate::yaml`), the parser's own, so a
//! value that needs quoting (`Use when: x`, `true` as text, `@file`) is quoted and never relies on
//! the reader's relaxed retry. A value is a JSON-shaped one (text, number, boolean, `null`, a
//! list, a table); `undefined` is "no key".

use serde::Serialize;

use crate::format::frontmatter::{FrontmatterState, frontmatter_document, parse_frontmatter};
use crate::js::{JsObject, JsValue, number_to_string, property_order};
use crate::yaml::nodes::{Body, Document, Key, Pair, map_set};
use crate::yaml::{create_node, stringify_document};

// `WRITE_OPTIONS` (`{ lineWidth: 0 }`, no line folding: a long `description` stays the one line
// it was given) and `NODE_OPTIONS` (`{ aliasDuplicateObjects: false }`, a value given twice is
// written twice, never as a YAML anchor and alias) are the only options `crate::yaml` writes and
// creates nodes with.

fn defined_entries(fields: &JsObject) -> Vec<(&String, &JsValue)> {
    property_order(fields)
        .into_iter()
        .filter(|(_, value)| !value.is_undefined())
        .collect()
}

/// A whole file: `---`, the fields in the order given (JavaScript lists integer-like keys first),
/// `---`, then `body` exactly as given, with no line added before it. An empty `fields` still
/// writes the two markers, so a body that opens with `---` stays body.
///
/// # Errors
///
/// What the YAML writer throws: for a JSON-shaped value, only that it is nested too deep.
pub fn write_frontmatter(fields: &JsObject, body: &str) -> Result<String, String> {
    let entries = defined_entries(fields);
    let object: JsObject = entries
        .iter()
        .map(|(key, value)| ((*key).clone(), (*value).clone()))
        .collect();
    let document = Document::new(Some(create_node(&JsValue::Object(object))?));
    let yaml = if entries.is_empty() {
        String::new()
    } else {
        stringify_document(&document)?
    };
    Ok(format!("---\n{yaml}---\n{body}"))
}

/// What an edit gives: the new text, or why the block is left alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum FrontmatterEdit {
    /// The file with the changes.
    Written {
        /// Always `true`.
        ok: bool,
        /// The new text.
        text: String,
    },
    /// The block does not read: writing would drop what the file holds, so nothing is written.
    Refused {
        /// Always `false`.
        ok: bool,
        /// `invalid-yaml` or `not-a-table`.
        state: FrontmatterState,
        /// What `parse_frontmatter` gave as the problem.
        problem: String,
    },
}

/// A block that is there but does not read: what `parse_frontmatter` gives a `problem` for.
const REFUSED: [FrontmatterState; 2] = [FrontmatterState::InvalidYaml, FrontmatterState::NotATable];

fn is_refused(state: FrontmatterState) -> bool {
    REFUSED.contains(&state)
}

/// `String(value)` of a scalar key's value.
fn key_text(value: &JsValue) -> String {
    match value {
        JsValue::String(text) => text.clone(),
        JsValue::Number(number) => number_to_string(*number),
        JsValue::Bool(flag) => flag.to_string(),
        JsValue::Null => "null".to_owned(),
        JsValue::Undefined => "undefined".to_owned(),
        JsValue::Array(_) | JsValue::Object(_) => value.to_json_text(),
    }
}

fn is_key(key: &str, pair: &Pair) -> bool {
    match &pair.key {
        Key::Node(node) => node
            .as_scalar()
            .is_some_and(|scalar| key_text(&scalar.value) == key),
        Key::Text(text) => text == key,
    }
}

/// One key of the map set or removed; a key the block repeats keeps its first place only. Fails
/// as the package's `createNode` does: on a value too deeply nested.
fn apply(items: &mut Vec<Pair>, key: &str, value: &JsValue) -> Result<(), String> {
    let first = items.iter().position(|pair| is_key(key, pair));
    let mut index = 0;
    items.retain(|pair| {
        let keep = Some(index) == first || !is_key(key, pair);
        index += 1;
        keep
    });
    if value.is_undefined() {
        items.retain(|pair| !is_key(key, pair));
        return Ok(());
    }
    map_set(items, key, create_node(value)?);
    Ok(())
}

/// `text` with `changes` set in its frontmatter: a key it has keeps its place, a new one goes
/// after the others in the order given, `undefined` removes one; every other key and the
/// comments stay. `body` replaces the body when given, else the body stays byte for byte. A file
/// without a block gets one, its whole text the body. A block that does not read is refused, not
/// overwritten.
///
/// # Errors
///
/// What the YAML writer throws: an alias whose anchor the changes removed, a value nested too
/// deep.
pub fn edit_frontmatter(
    text: &str,
    changes: &JsObject,
    body: Option<&str>,
) -> Result<FrontmatterEdit, String> {
    let read = parse_frontmatter(text);
    let found = frontmatter_document(text);
    if let (Some(problem), true) = (&read.problem, is_refused(read.state)) {
        return Ok(FrontmatterEdit::Refused {
            ok: false,
            state: read.state,
            problem: problem.clone(),
        });
    }
    let kept = body.map_or(read.body, str::to_owned);
    let Some(found) = found else {
        return Ok(FrontmatterEdit::Written {
            ok: true,
            text: write_frontmatter(changes, &kept)?,
        });
    };
    let mut document = found.document;
    if !matches!(
        document.contents.as_ref().map(|node| &node.body),
        Some(Body::Map { .. })
    ) {
        document = Document::new(Some(create_node(&JsValue::Object(JsObject::new()))?));
    }
    let Some(Body::Map { items, .. }) = document.contents.as_mut().map(|node| &mut node.body)
    else {
        return Ok(FrontmatterEdit::Written {
            ok: true,
            text: write_frontmatter(changes, &kept)?,
        });
    };
    for (key, value) in property_order(changes) {
        apply(items, key, value)?;
    }
    let yaml = if items.is_empty() {
        String::new()
    } else {
        stringify_document(&document)?
    };
    Ok(FrontmatterEdit::Written {
        ok: true,
        text: format!("---\n{yaml}---\n{kept}"),
    })
}

crate::registry::exports! {
    "format/frontmatter-write" {
        "writeFrontmatter" => fallible |fields: JsObject, body: String| write_frontmatter(&fields, &body);
        "editFrontmatter" => fallible |text: String, changes: JsObject, body: Option<String>| {
            edit_frontmatter(&text, &changes, body.as_deref())
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{FrontmatterEdit, edit_frontmatter};
    use crate::js::{JsObject, JsValue};

    /// A member that is `undefined` removes its key. The vectors cannot hold this call (the
    /// recorder leaves an `undefined` member out, the wire keeps it), so the answer is the
    /// package's: `bun -e "import { editFrontmatter } from './src/format/frontmatter-write.ts';
    /// console.log(JSON.stringify(editFrontmatter('---\nname: demo # inline\ndescription:
    /// Old\nx-host: kept\n---\nBody\n', { description: undefined })))"`.
    #[test]
    fn an_undefined_member_removes_its_key() {
        let mut changes = JsObject::new();
        changes.insert("description".to_owned(), JsValue::Undefined);
        let text = "---\nname: demo # inline\ndescription: Old\nx-host: kept\n---\nBody\n";
        assert_eq!(
            edit_frontmatter(text, &changes, None),
            Ok(FrontmatterEdit::Written {
                ok: true,
                text: "---\nname: demo # inline\nx-host: kept\n---\nBody\n".to_owned(),
            })
        );
    }
}
