//! A pair as text (`dist/stringify/stringifyPair.js` and `Pair.prototype.toString`): the key,
//! explicit where it has to be, the `:` and the value with the space between.

use std::collections::HashSet;

use super::collection::{is_collection, some_text};
use super::{
    Context, INDENT_STEP, Item, Out, indent_comment, line_comment, stringify, stringify_comment,
};
use crate::js::code_unit_length;
use crate::yaml::nodes::{Body, Key, Node, Pair, ScalarType};

/// `!key`, a block scalar key, or an alias (an object that is no scalar): a key written explicitly.
fn odd_key(key: &Key) -> bool {
    match key {
        Key::Text(text) => text.is_empty(),
        Key::Node(key) => match &key.body {
            Body::Scalar(scalar) => matches!(
                scalar.kind,
                Some(ScalarType::BlockFolded | ScalarType::BlockLiteral)
            ),
            Body::Alias(_) => true,
            Body::Map { .. } | Body::Seq { .. } => false,
        },
    }
}

/// `stringifyPair`.
pub(crate) fn stringify_pair(
    pair: &Pair,
    ctx: &Context<'_>,
    anchors: &mut HashSet<String>,
    out: &mut Out,
) -> Result<String, String> {
    let (key_item, key_node) = match &pair.key {
        Key::Node(key) => (Item::Node(key), Some(&**key)),
        Key::Text(text) => (Item::Text(text), None),
    };
    let mut key_comment = key_node.and_then(|key| some_text(key.comment.as_ref()));
    let mut explicit_key = (key_comment.is_some() && pair.value.is_none() && !ctx.in_flow)
        || is_collection(key_node)
        || odd_key(&pair.key);
    let indent = ctx.indent.clone();
    let mut ctx = ctx.clone();
    ctx.implicit_key = !explicit_key && !ctx.all_null_values;
    let all_null_values = ctx.all_null_values;
    ctx.all_null_values = false;
    ctx.indent = format!("{indent}{INDENT_STEP}");
    let mut key_out = Out::default();
    let mut text = stringify(key_item, &mut ctx, anchors, &mut key_out)?;
    if !explicit_key && !ctx.in_flow && code_unit_length(&text) > 1024 {
        explicit_key = true;
    }
    if ctx.in_flow {
        if all_null_values || pair.value.is_none() {
            out.commented |= key_out.commented;
            return Ok(if text.is_empty() {
                "?".to_owned()
            } else if explicit_key {
                format!("? {text}")
            } else {
                text
            });
        }
    } else if all_null_values || (pair.value.is_none() && explicit_key) {
        text = format!("? {text}");
        if let Some(comment) = key_comment.filter(|_| !key_out.commented) {
            text.push_str(&line_comment(
                &text,
                &ctx.indent,
                &stringify_comment(comment),
            ));
        } else if key_out.chomp_keep {
            out.chomp_keep = true;
        }
        return Ok(text);
    }
    if key_out.commented {
        key_comment = None;
    }
    text = key_with_indicator(text, explicit_key, key_comment, &ctx.indent, &indent);
    let value = pair.value.as_ref();
    let space_before = value.is_some_and(|value| value.space_before);
    let comment_before = value.and_then(|value| some_text(value.comment_before.as_ref()));
    let value_comment = value.and_then(|value| some_text(value.comment.as_ref()));
    ctx.implicit_key = false;
    if !explicit_key
        && key_comment.is_none()
        && value.is_some_and(|value| value.as_scalar().is_some())
    {
        ctx.indent_at_start = Some(code_unit_length(&text) + 1);
    }
    let mut value_out = Out::default();
    let value_item = value.map_or(Item::Null, Item::Node);
    let value_text = stringify(value_item, &mut ctx, anchors, &mut value_out)?;
    let ws = value_space(
        &ctx,
        explicit_key,
        key_comment.is_some(),
        value,
        &value_text,
        [space_before, comment_before.is_some()],
    )
    .unwrap_or_else(|| {
        commented_value_space(
            &ctx,
            space_before,
            comment_before,
            value_comment.is_some(),
            &value_text,
        )
    });
    text.push_str(&ws);
    text.push_str(&value_text);
    if ctx.in_flow {
        out.commented |= value_out.commented;
    } else if let Some(comment) = value_comment.filter(|_| !value_out.commented) {
        text.push_str(&line_comment(
            &text,
            &ctx.indent,
            &stringify_comment(comment),
        ));
    } else if value_out.chomp_keep {
        out.chomp_keep = true;
    }
    Ok(text)
}

/// The key with its `:`, or as `? key` with the `:` on the next line, and the key's comment.
fn key_with_indicator(
    mut text: String,
    explicit_key: bool,
    key_comment: Option<&str>,
    ctx_indent: &str,
    indent: &str,
) -> String {
    if explicit_key {
        if let Some(comment) = key_comment {
            text.push_str(&line_comment(
                &text,
                ctx_indent,
                &stringify_comment(comment),
            ));
        }
        return format!("? {text}\n{indent}:");
    }
    text.push(':');
    if let Some(comment) = key_comment {
        text.push_str(&line_comment(
            &text,
            ctx_indent,
            &stringify_comment(comment),
        ));
    }
    text
}

/// The space between `key:` and the value when the key has a comment, or a blank line or a
/// comment comes before the value.
fn commented_value_space(
    ctx: &Context<'_>,
    space_before: bool,
    comment_before: Option<&str>,
    value_comment: bool,
    value_text: &str,
) -> String {
    let mut ws = if space_before {
        "\n".to_owned()
    } else {
        String::new()
    };
    if let Some(before) = comment_before {
        ws.push('\n');
        ws.push_str(&indent_comment(&stringify_comment(before), &ctx.indent));
    }
    if value_text.is_empty() && !ctx.in_flow {
        if ws == "\n" && value_comment {
            "\n\n".clone_into(&mut ws);
        }
    } else {
        ws.push('\n');
        ws.push_str(&ctx.indent);
    }
    ws
}

/// The space between `key:` and the value when no comment or blank line comes before the value;
/// `None` when one does.
fn value_space(
    ctx: &Context<'_>,
    explicit_key: bool,
    key_comment: bool,
    value: Option<&Node>,
    value_text: &str,
    [space_before, comment_before]: [bool; 2],
) -> Option<String> {
    if key_comment || space_before || comment_before {
        return None;
    }
    let mut ws = " ".to_owned();
    if let (
        false,
        Some(
            node @ Node {
                body: Body::Map { items: _, flow } | Body::Seq { items: _, flow },
                ..
            },
        ),
    ) = (explicit_key, value)
    {
        let empty = match &node.body {
            Body::Map { items, .. } => items.is_empty(),
            Body::Seq { items, .. } => items.is_empty(),
            Body::Scalar(_) | Body::Alias(_) => false,
        };
        let newline = value_text.find('\n');
        let flow = ctx.in_flow || *flow || empty;
        if newline.is_some() || !flow {
            let props_line = newline.is_some_and(|newline| has_props_line(value_text, newline));
            if !props_line {
                ws = format!("\n{}", ctx.indent);
            }
        }
    } else if value_text.is_empty() || value_text.starts_with('\n') {
        ws = String::new();
    }
    Some(ws)
}

/// Whether a collection's text starts with a line of its anchor and tag alone.
fn has_props_line(text: &str, newline: usize) -> bool {
    let first = text.chars().next();
    if !matches!(first, Some('&' | '!')) {
        return false;
    }
    let mut space = text.find(' ');
    if first == Some('&')
        && let Some(at) = space.filter(|at| *at < newline)
        && text.get(at + 1..).is_some_and(|rest| rest.starts_with('!'))
    {
        space = text
            .get(at + 1..)
            .and_then(|rest| rest.find(' '))
            .map(|found| found + at + 1);
    }
    space.is_none_or(|space| newline < space)
}
