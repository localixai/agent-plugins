//! Port of `src/format/workflow-meta-rewrite.ts`, switched.
//!
//! Rewrites single fields of the `meta` literal of a workflow script and keeps every other byte
//! (workflows.md, "Edit a saved script"). What the docs leave open (quoting, layout, line endings)
//! is in spec/judgments.md.
//!
//! Positions are the tokens', in UTF-16 code units, so the script is cut as units.

use serde::Serialize;

use super::issues::PluginIssue;
use super::js_literal::LiteralObject;
use super::workflow_meta::{ScriptScope, read_script_head, read_workflow_meta};
use crate::js::identifier::{is_id_continue, is_id_start};
use crate::js::utf16::{slice, text_of, units};
use crate::js::{JsValue, number_to_string};
use crate::protocol::is_record::is_record;

/// The fields a rewrite sets, in the order it sets them; a field not given is left as written.
const PATCH_KEYS: [&str; 4] = ["name", "description", "phases", "whenToUse"];

/// What a single-quoted JavaScript string must escape: the quote, the backslash and every line
/// break (the two Unicode line separators end a line in a JavaScript string too).
fn escaped(character: char) -> Option<&'static str> {
    match character {
        '\\' => Some("\\\\"),
        '\'' => Some("\\'"),
        '\n' => Some("\\n"),
        '\r' => Some("\\r"),
        '\u{2028}' => Some("\\u2028"),
        '\u{2029}' => Some("\\u2029"),
        _ => None,
    }
}

/// `/^[\p{ID_Start}_$][\p{ID_Continue}$]*$/u`.
fn is_identifier(key: &str) -> bool {
    let mut characters = key.chars();
    characters
        .next()
        .is_some_and(|first| is_id_start(first) || matches!(first, '_' | '$'))
        && characters.all(|next| is_id_continue(next) || next == '$')
}

fn quote(text: &str) -> String {
    let mut out = String::from("'");
    for character in text.chars() {
        match escaped(character) {
            Some(escape) => out.push_str(escape),
            None => out.push(character),
        }
    }
    out.push('\'');
    out
}

fn key_text(key: &str) -> String {
    if is_identifier(key) {
        key.to_owned()
    } else {
        quote(key)
    }
}

/// A literal value as source: strings single-quoted, arrays and objects in one line. What a
/// literal cannot hold is written as JSON writes it (judgment, spec/judgments.md): a field whose
/// value is `undefined` is left out, and `NaN`, `Infinity` or any value of another type is
/// `null`.
fn literal(value: &JsValue) -> String {
    match value {
        JsValue::String(text) => quote(text),
        JsValue::Bool(flag) => flag.to_string(),
        JsValue::Number(number) if number.is_finite() => number_to_string(*number),
        JsValue::Array(items) => {
            let items: Vec<String> = items.iter().map(literal).collect();
            format!("[{}]", items.join(", "))
        }
        JsValue::Object(members) if is_record(value) => {
            let fields: Vec<String> = members
                .iter()
                .filter(|(_, each)| **each != JsValue::Undefined)
                .map(|(key, each)| format!("{}: {}", key_text(key), literal(each)))
                .collect();
            if fields.is_empty() {
                "{}".to_owned()
            } else {
                format!("{{ {} }}", fields.join(", "))
            }
        }
        JsValue::Undefined | JsValue::Null | JsValue::Number(_) | JsValue::Object(_) => {
            "null".to_owned()
        }
    }
}

struct Splice {
    start: usize,
    end: usize,
    text: String,
}

fn indent_of(source: &[u16], object: &LiteralObject) -> String {
    let Some(last) = object.properties.last() else {
        return "  ".to_owned();
    };
    let newline = u16::from(b'\n');
    let line_start = slice(source, 0, last.value_start.saturating_add(1))
        .iter()
        .rposition(|unit| *unit == newline)
        .map_or(0, |at| at + 1);
    let line = slice(source, line_start, source.len());
    let blank = line
        .iter()
        .take_while(|unit| **unit == u16::from(b' ') || **unit == u16::from(b'\t'))
        .count();
    text_of(slice(line, 0, blank))
}

/// The line terminator of the block: `\r\n` when the block uses it, so a rewrite never mixes
/// endings (judgment).
fn terminator_of(block: &str) -> &'static str {
    if block.contains("\r\n") { "\r\n" } else { "\n" }
}

/// The text that appends `added` (`key: value` pieces) after the last property, in the style of
/// the block.
fn appendix(source: &[u16], object: &LiteralObject, added: &[String]) -> Option<Splice> {
    if added.is_empty() {
        return None;
    }
    let block = text_of(slice(source, object.start, object.end));
    let indent = indent_of(source, object);
    let Some(last) = object.properties.last() else {
        let at = object.start + 1;
        let text = format!(" {} ", added.join(", "));
        return Some(Splice {
            start: at,
            end: at,
            text,
        });
    };
    let trailing = object.trailing_comma;
    let at = if trailing {
        object.after_last
    } else {
        last.value_end
    };
    let eol = terminator_of(&block);
    let lines: Vec<String> = if block.contains('\n') {
        added
            .iter()
            .map(|each| {
                if trailing {
                    format!("{eol}{indent}{each},")
                } else {
                    format!(",{eol}{indent}{each}")
                }
            })
            .collect()
    } else {
        added
            .iter()
            .map(|each| {
                if trailing {
                    format!(" {each},")
                } else {
                    format!(", {each}")
                }
            })
            .collect()
    };
    Some(Splice {
        start: at,
        end: at,
        text: lines.concat(),
    })
}

fn splices(source: &[u16], object: &LiteralObject, patch: &JsValue) -> Vec<Splice> {
    let mut replaced = Vec::new();
    let mut added = Vec::new();
    for key in PATCH_KEYS {
        let Some(value) = patch.get(key).filter(|value| **value != JsValue::Undefined) else {
            continue;
        };
        let existing = object.properties.iter().rev().find(|each| each.key == key);
        match existing {
            Some(existing) => replaced.push(Splice {
                start: existing.value_start,
                end: existing.value_end,
                text: literal(value),
            }),
            None => added.push(format!("{key}: {}", literal(value))),
        }
    }
    replaced.extend(appendix(source, object, &added));
    replaced
}

/// A rewritten script, or `None` when it is left alone, and the issues of what it holds now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Rewrite {
    /// The script with the fields set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// What is wrong with the `meta` it reads.
    pub issues: Vec<PluginIssue>,
}

/// Sets fields of the `meta` literal and keeps every other byte of the script. A `meta` that is
/// not a literal is left alone (the reasons come back as issues), and so is a script the rewrite
/// would leave without a name or a description: `source` is then `None`. `patch` is the
/// `MetaPatch`: `name`, `description`, `phases` (titles, or objects with a `title`) and
/// `whenToUse`.
#[must_use]
pub fn rewrite_workflow_meta(source: &str, patch: &JsValue) -> Rewrite {
    let head = read_script_head(source, ScriptScope::Head);
    let Some(object) = head.object else {
        return Rewrite {
            source: None,
            issues: head.issues,
        };
    };
    let text = units(source);
    let mut edits = splices(&text, &object, patch);
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.start));
    let edited = edits.iter().fold(text, |text, edit| {
        let mut out = slice(&text, 0, edit.start).to_vec();
        out.extend(edit.text.encode_utf16());
        out.extend_from_slice(slice(&text, edit.end, text.len()));
        out
    });
    let edited = text_of(&edited);
    let check = read_workflow_meta(&edited);
    Rewrite {
        source: check.meta.map(|_| edited),
        issues: check.issues,
    }
}

crate::registry::exports! {
    "format/workflow-meta-rewrite" {
        "rewriteWorkflowMeta" => ok |source: String, patch: JsValue| rewrite_workflow_meta(&source, &patch);
    }
}
