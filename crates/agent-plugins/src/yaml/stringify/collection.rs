//! Maps and sequences as text (`dist/stringify/stringifyCollection.js`, the `toString` of
//! `YAMLMap` and `YAMLSeq`, and `hasAllNullValues`): block and flow style with the comments and blank lines they kept.

use std::collections::HashSet;

use super::{
    Context, INDENT_STEP, Item, Out, indent_comment, line_comment, stringify, stringify_comment,
};
use crate::js::code_unit_length;
use crate::yaml::nodes::{Body, Key, Node, Pair};

/// How `stringifyCollection` writes a map or a sequence.
struct Style<'s> {
    block_item_prefix: &'s str,
    start: &'s str,
    end: &'s str,
    item_indent: String,
}

/// `YAMLMap.prototype.toString` and `YAMLSeq.prototype.toString`.
pub(crate) fn stringify_collection(
    node: &Node,
    ctx: &mut Context<'_>,
    anchors: &mut HashSet<String>,
    out: &mut Out,
) -> Result<String, String> {
    let mut ctx = ctx.clone();
    let (items, style, flow) = match &node.body {
        Body::Map { items, flow } => {
            if !ctx.all_null_values && items.iter().all(|pair| pair.value.is_none()) {
                ctx.all_null_values = true;
            }
            let style = Style {
                block_item_prefix: "",
                start: "{",
                end: "}",
                item_indent: ctx.indent.clone(),
            };
            (
                items.iter().map(Item::Pair).collect::<Vec<_>>(),
                style,
                *flow,
            )
        }
        Body::Seq { items, flow } => {
            let style = Style {
                block_item_prefix: "- ",
                start: "[",
                end: "]",
                item_indent: format!("{}  ", ctx.indent),
            };
            (items.iter().map(Item::Node).collect(), style, *flow)
        }
        Body::Scalar(_) | Body::Alias(_) => return Ok(String::new()),
    };
    if ctx.in_flow || flow {
        stringify_flow_collection(&items, &ctx, anchors, &style)
    } else {
        stringify_block_collection(node.comment.as_deref(), &items, &ctx, anchors, &style, out)
    }
}

/// The node an item's leading comment and blank line belong to: the item, or a pair's key.
fn leading_node<'n>(item: &Item<'n>) -> Option<&'n Node> {
    match item {
        Item::Node(node) => Some(node),
        Item::Pair(Pair {
            key: Key::Node(key),
            ..
        }) => Some(key),
        Item::Pair(_) | Item::Text(_) | Item::Null => None,
    }
}

pub(super) fn some_text(text: Option<&String>) -> Option<&str> {
    text.map(String::as_str).filter(|text| !text.is_empty())
}

/// `addCommentBefore`.
fn add_comment_before(
    ctx: &Context<'_>,
    lines: &mut Vec<String>,
    comment: Option<&str>,
    chomp_keep: bool,
) {
    let comment = comment.map(|comment| {
        if chomp_keep {
            comment.trim_start_matches('\n')
        } else {
            comment
        }
    });
    if let Some(comment) = comment.filter(|comment| !comment.is_empty()) {
        let text = indent_comment(&stringify_comment(comment), &ctx.indent);
        // Avoid a double indent on the first line.
        lines.push(crate::js::trim_start(&text).to_owned());
    }
}

/// `stringifyBlockCollection`.
fn stringify_block_collection(
    comment: Option<&str>,
    items: &[Item<'_>],
    ctx: &Context<'_>,
    anchors: &mut HashSet<String>,
    style: &Style<'_>,
    out: &mut Out,
) -> Result<String, String> {
    let mut item_ctx = ctx.clone();
    item_ctx.indent.clone_from(&style.item_indent);
    let mut chomp_keep = false;
    let mut lines = Vec::new();
    for item in items {
        let mut item_comment = None;
        if let Some(node) = leading_node(item) {
            if !chomp_keep && node.space_before {
                lines.push(String::new());
            }
            add_comment_before(
                ctx,
                &mut lines,
                some_text(node.comment_before.as_ref()),
                chomp_keep,
            );
            if matches!(item, Item::Node(_)) {
                item_comment = some_text(node.comment.as_ref());
            }
        }
        let mut item_out = Out::default();
        let mut text = stringify(*item, &mut item_ctx, anchors, &mut item_out)?;
        if item_out.commented {
            item_comment = None;
        }
        chomp_keep = item_out.chomp_keep;
        if let Some(comment) = item_comment {
            text.push_str(&line_comment(
                &text,
                &style.item_indent,
                &stringify_comment(comment),
            ));
            chomp_keep = false;
        }
        lines.push(format!("{}{text}", style.block_item_prefix));
    }
    let mut text = match lines.split_first() {
        None => format!("{}{}", style.start, style.end),
        Some((first, rest)) => {
            let mut text = first.clone();
            for line in rest {
                if line.is_empty() {
                    text.push('\n');
                } else {
                    text.push('\n');
                    text.push_str(&ctx.indent);
                    text.push_str(line);
                }
            }
            text
        }
    };
    if let Some(comment) = comment.filter(|comment| !comment.is_empty()) {
        text.push('\n');
        text.push_str(&indent_comment(&stringify_comment(comment), &ctx.indent));
        out.commented = true;
    } else if chomp_keep {
        out.chomp_keep = true;
    }
    Ok(text)
}

/// `stringifyFlowCollection`.
fn stringify_flow_collection(
    items: &[Item<'_>],
    ctx: &Context<'_>,
    anchors: &mut HashSet<String>,
    style: &Style<'_>,
) -> Result<String, String> {
    let item_indent = format!("{}{INDENT_STEP}", style.item_indent);
    let mut item_ctx = ctx.clone();
    item_ctx.indent.clone_from(&item_indent);
    item_ctx.in_flow = true;
    let mut req_newline = false;
    let mut lines_at_value = 0;
    let mut lines: Vec<String> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let mut comment = None;
        match item {
            Item::Node(node) => {
                if node.space_before {
                    lines.push(String::new());
                }
                add_comment_before(
                    ctx,
                    &mut lines,
                    some_text(node.comment_before.as_ref()),
                    false,
                );
                comment = some_text(node.comment.as_ref());
            }
            Item::Pair(pair) => {
                let key = match &pair.key {
                    Key::Node(key) => Some(&**key),
                    Key::Text(_) => None,
                };
                if let Some(key) = key {
                    if key.space_before {
                        lines.push(String::new());
                    }
                    add_comment_before(
                        ctx,
                        &mut lines,
                        some_text(key.comment_before.as_ref()),
                        false,
                    );
                    req_newline |= some_text(key.comment.as_ref()).is_some();
                }
                match &pair.value {
                    Some(value) => {
                        comment = some_text(value.comment.as_ref());
                        req_newline |= some_text(value.comment_before.as_ref()).is_some();
                    }
                    None => comment = key.and_then(|key| some_text(key.comment.as_ref())),
                }
            }
            Item::Text(_) | Item::Null => {}
        }
        req_newline |= comment.is_some();
        let mut item_out = Out::default();
        let mut text = stringify(*item, &mut item_ctx, anchors, &mut item_out)?;
        if item_out.commented {
            comment = None;
        }
        req_newline = req_newline || lines.len() > lines_at_value || text.contains('\n');
        if index + 1 < items.len() {
            text.push(',');
        }
        if let Some(comment) = comment {
            text.push_str(&line_comment(
                &text,
                &item_indent,
                &stringify_comment(comment),
            ));
        }
        lines.push(text);
        lines_at_value = lines.len();
    }
    if lines.is_empty() {
        return Ok(format!("{}{}", style.start, style.end));
    }
    if !req_newline {
        let length = lines
            .iter()
            .map(|line| code_unit_length(line) + 2)
            .sum::<usize>()
            + 2;
        req_newline = ctx.line_width > 0 && length > ctx.line_width;
    }
    if req_newline {
        let mut text = style.start.to_owned();
        for line in &lines {
            text.push('\n');
            if !line.is_empty() {
                text.push_str(INDENT_STEP);
                text.push_str(&ctx.indent);
                text.push_str(line);
            }
        }
        Ok(format!("{text}\n{}{}", ctx.indent, style.end))
    } else {
        Ok(format!("{} {} {}", style.start, lines.join(" "), style.end))
    }
}

/// Whether a value is a node that is a collection.
pub(super) const fn is_collection(node: Option<&Node>) -> bool {
    matches!(
        node,
        Some(Node {
            body: Body::Map { .. } | Body::Seq { .. },
            ..
        })
    )
}
