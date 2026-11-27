//! A string scalar as text (`dist/stringify/stringifyString.js`): plain where it reads back as
//! the same string, else quoted or a block scalar, as the package picks.

mod block;

use std::sync::LazyLock;

use self::block::block_string;
use super::fold::{Mode, fold_flow_lines};
use super::{Context, Out};
use crate::js::{JsValue, Pattern, number_to_string, regex_test};
use crate::yaml::nodes::{Scalar, ScalarType};
use crate::yaml::tags::{CORE, Tag};
use crate::yaml::units::{string, unit, units};

/// `doubleQuotedMinMultiLineLength`'s default.
const DOUBLE_QUOTED_MIN_MULTI_LINE_LENGTH: usize = 40;

/// The characters a plain scalar cannot start or end with, or hold where they would read as
/// something else.
static PLAIN_REFUSED: Pattern = LazyLock::new(|| {
    regex::Regex::new(
        r#"^[\n\t ,\[\]{}#&*!|>'"%@`]|^[?-]$|^[?-][ \t]|[\n:][ \t]|[ \t]\n|[\n\t ]#|[\n\t :]$"#,
    )
    .ok()
});

/// `containsDocumentMarker`: a line that starts with `%`, `---` or `...`.
fn contains_document_marker(text: &str) -> bool {
    text.split(['\n', '\r', '\u{2028}', '\u{2029}'])
        .any(|line| line.starts_with('%') || line.starts_with("---") || line.starts_with("..."))
}

/// `text.replace(/\n+/g, '$&' + after)`: `after` written after each run of line breaks.
fn after_newlines(text: &str, after: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        out.push(character);
        if character == '\n' && chars.peek() != Some(&'\n') {
            out.push_str(after);
        }
    }
    out
}

/// The indent of a quoted or block string at the document's root.
fn root_indent(ctx: &Context<'_>, value: &str, force: bool) -> String {
    if !ctx.indent.is_empty() {
        ctx.indent.clone()
    } else if force || contains_document_marker(value) {
        "  ".to_owned()
    } else {
        String::new()
    }
}

/// `doubleQuotedString`.
fn double_quoted_string(value: &str, ctx: &Context<'_>) -> String {
    let json = units(&JsValue::String(value.to_owned()).to_json_text());
    let indent = root_indent(ctx, value, false);
    let is = |index: usize, character: char| json.get(index) == Some(&unit(character));
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while let Some(&code) = json.get(i) {
        let mut ch = code;
        if ch == unit(' ') && is(i + 1, '\\') && is(i + 2, 'n') {
            // A space before a newline needs to be escaped to not be folded.
            out.extend_from_slice(json.get(start..i).unwrap_or_default());
            out.extend(units("\\ "));
            i += 1;
            start = i;
            ch = unit('\\');
        }
        if ch == unit('\\') {
            match json
                .get(i + 1)
                .and_then(|next| char::from_u32(u32::from(*next)))
            {
                Some('u') => {
                    out.extend_from_slice(json.get(start..i).unwrap_or_default());
                    let code = string(json.get(i + 2..(i + 6).min(json.len())).unwrap_or_default());
                    let short = match code.as_str() {
                        "0000" => Some("\\0".to_owned()),
                        "0007" => Some("\\a".to_owned()),
                        "000b" => Some("\\v".to_owned()),
                        "001b" => Some("\\e".to_owned()),
                        "0085" => Some("\\N".to_owned()),
                        "00a0" => Some("\\_".to_owned()),
                        "2028" => Some("\\L".to_owned()),
                        "2029" => Some("\\P".to_owned()),
                        _ => code.strip_prefix("00").map(|rest| format!("\\x{rest}")),
                    };
                    match short {
                        Some(short) => out.extend(units(&short)),
                        None => out.extend_from_slice(
                            json.get(i..(i + 6).min(json.len())).unwrap_or_default(),
                        ),
                    }
                    i += 5;
                    start = i + 1;
                }
                Some('n') => {
                    if ctx.implicit_key
                        || is(i + 2, '"')
                        || json.len() < DOUBLE_QUOTED_MIN_MULTI_LINE_LENGTH
                    {
                        i += 1;
                    } else {
                        // Folding will eat the first newline.
                        out.extend_from_slice(json.get(start..i).unwrap_or_default());
                        out.extend(units("\n\n"));
                        while is(i + 2, '\\') && is(i + 3, 'n') && !is(i + 4, '"') {
                            out.push(unit('\n'));
                            i += 2;
                        }
                        out.extend(units(&indent));
                        // A space after a newline needs to be escaped to not be folded.
                        if is(i + 2, ' ') {
                            out.push(unit('\\'));
                        }
                        i += 1;
                        start = i + 1;
                    }
                }
                _ => i += 1,
            }
        }
        i += 1;
    }
    let text = if start > 0 {
        out.extend_from_slice(json.get(start..).unwrap_or_default());
        string(&out)
    } else {
        string(&json)
    };
    if ctx.implicit_key {
        text
    } else {
        fold_flow_lines(
            &text,
            &indent,
            Mode::Quoted,
            ctx.indent_at_start,
            ctx.line_width,
        )
        .0
    }
}

/// `singleQuotedString`.
fn single_quoted_string(value: &str, ctx: &Context<'_>) -> String {
    let around_newline = value.contains(" \n")
        || value.contains("\t\n")
        || value.contains("\n ")
        || value.contains("\n\t");
    if (ctx.implicit_key && value.contains('\n')) || around_newline {
        // A single-quoted string can't have leading or trailing whitespace around a newline.
        return double_quoted_string(value, ctx);
    }
    let indent = root_indent(ctx, value, false);
    let quoted = format!(
        "'{}'",
        after_newlines(&value.replace('\'', "''"), &format!("\n{indent}"))
    );
    if ctx.implicit_key {
        quoted
    } else {
        fold_flow_lines(
            &quoted,
            &indent,
            Mode::Flow,
            ctx.indent_at_start,
            ctx.line_width,
        )
        .0
    }
}

/// `quotedString`: single quotes for a text with double quotes only, else double quotes.
fn quoted_string(value: &str, ctx: &Context<'_>) -> String {
    if value.contains('"') && !value.contains('\'') {
        single_quoted_string(value, ctx)
    } else {
        double_quoted_string(value, ctx)
    }
}

/// `plainString`.
fn plain_string(
    scalar: &Scalar,
    value: &str,
    comment: Option<&str>,
    ctx: &mut Context<'_>,
    out: &mut Out,
) -> String {
    let flow_chars = value.contains(['[', ']', '{', '}', ',']);
    if (ctx.implicit_key && value.contains('\n')) || (ctx.in_flow && flow_chars) {
        return quoted_string(value, ctx);
    }
    if regex_test(&PLAIN_REFUSED, value) {
        return if ctx.implicit_key || ctx.in_flow || !value.contains('\n') {
            quoted_string(value, ctx)
        } else {
            block_string(scalar, value, comment, ctx, out)
        };
    }
    if !ctx.implicit_key
        && !ctx.in_flow
        && scalar.kind != Some(ScalarType::Plain)
        && value.contains('\n')
    {
        // Where allowed and the type is not set explicitly, prefer block style for multiline
        // strings.
        return block_string(scalar, value, comment, ctx, out);
    }
    if contains_document_marker(value) {
        if ctx.indent.is_empty() {
            ctx.force_block_indent = true;
            return block_string(scalar, value, comment, ctx, out);
        } else if ctx.implicit_key && ctx.indent == super::INDENT_STEP {
            return quoted_string(value, ctx);
        }
    }
    let text = after_newlines(value, &format!("\n{}", ctx.indent));
    // Verify that the output will be read as a string: plain numbers and booleans are read as
    // those types in YAML 1.2 (`42`, `true`, `0.9e-3`).
    if ctx.actual_string
        && CORE
            .into_iter()
            .any(|tag| tag != Tag::Str && tag.test(&text))
    {
        return quoted_string(value, ctx);
    }
    if ctx.implicit_key {
        text
    } else {
        fold_flow_lines(
            &text,
            &ctx.indent,
            Mode::Flow,
            ctx.indent_at_start,
            ctx.line_width,
        )
        .0
    }
}

/// `String(value)` of a scalar's value.
fn as_text(value: &JsValue) -> String {
    match value {
        JsValue::String(text) => text.clone(),
        JsValue::Number(number) => number_to_string(*number),
        JsValue::Bool(flag) => flag.to_string(),
        JsValue::Null => "null".to_owned(),
        JsValue::Undefined => "undefined".to_owned(),
        JsValue::Array(_) | JsValue::Object(_) => value.to_json_text(),
    }
}

/// `stringifyString`.
pub(crate) fn stringify_string(
    scalar: &Scalar,
    comment: Option<&str>,
    ctx: &mut Context<'_>,
    out: &mut Out,
) -> String {
    let value = as_text(&scalar.value);
    let mut kind = scalar.kind;
    let control = value
        .chars()
        .any(|character| matches!(character, '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{7f}'..='\u{9f}'));
    if kind != Some(ScalarType::QuoteDouble) && control {
        // Force double quotes on control characters.
        kind = Some(ScalarType::QuoteDouble);
    }
    match kind {
        Some(ScalarType::BlockFolded | ScalarType::BlockLiteral) => {
            if ctx.implicit_key || ctx.in_flow {
                // Blocks are not valid inside flow containers.
                quoted_string(&value, ctx)
            } else {
                block_string(scalar, &value, comment, ctx, out)
            }
        }
        Some(ScalarType::QuoteDouble) => double_quoted_string(&value, ctx),
        Some(ScalarType::QuoteSingle) => single_quoted_string(&value, ctx),
        Some(ScalarType::Plain) | None => plain_string(scalar, &value, comment, ctx, out),
    }
}
