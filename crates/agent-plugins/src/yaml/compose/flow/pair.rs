//! The pairs of a flow collection (the pair branch of `resolveFlowCollection`): a key and its
//! value, the errors of a misplaced `:`, and a comment moved onto the item before.

use super::{BLOCK_MSG, Flow, is_block};
use crate::yaml::compose::Ctx;
use crate::yaml::compose::node::{compose_empty_node, compose_node, starts_with_colon};
use crate::yaml::compose::props::{Next, Options, Props, resolve_props};
use crate::yaml::cst::{Item, Kind, SourceToken};
use crate::yaml::nodes::{Body, Key, Node, Pair, append};
use crate::yaml::units::{string, units};

/// A comment on the line of the previous item, moved to it.
pub(super) fn move_previous_comment(flow: &mut Flow, start: &[SourceToken], props: &mut Props<'_>) {
    let mut previous_comment = String::new();
    for token in start {
        match token.kind {
            Kind::Comma | Kind::Space => {}
            Kind::Comment => {
                previous_comment = string(token.source.get(1..).unwrap_or_default());
                break;
            }
            Kind::ByteOrderMark
            | Kind::DocMode
            | Kind::FlowErrorEnd
            | Kind::Scalar
            | Kind::DocStart
            | Kind::DocEnd
            | Kind::Newline
            | Kind::SeqItemInd
            | Kind::ExplicitKeyInd
            | Kind::MapValueInd
            | Kind::FlowMapStart
            | Kind::FlowMapEnd
            | Kind::FlowSeqStart
            | Kind::FlowSeqEnd
            | Kind::DirectiveLine
            | Kind::Alias
            | Kind::Anchor
            | Kind::Tag
            | Kind::SingleQuotedScalar
            | Kind::DoubleQuotedScalar
            | Kind::BlockScalarHeader => break,
        }
    }
    if previous_comment.is_empty() {
        return;
    }
    let previous = if flow.is_map {
        flow.items.last_mut().and_then(pair_comment_node)
    } else {
        flow.values.last_mut()
    };
    if let Some(previous) = previous {
        append(&mut previous.comment, "\n", &previous_comment);
    }
    let comment = units(&props.comment);
    let skip = units(&previous_comment).len() + 1;
    props.comment = string(comment.get(skip..).unwrap_or_default());
}

/// `prev.value ?? prev.key` of a pair.
fn pair_comment_node(pair: &mut Pair) -> Option<&mut Node> {
    match (&mut pair.value, &mut pair.key) {
        (Some(value), _) => Some(value),
        (None, Key::Node(key)) => Some(key),
        (None, Key::Text(_)) => None,
    }
}

/// An item that is a key and a value: a pair of the map, or a one-pair map in a sequence.
pub(super) fn flow_pair(ctx: &mut Ctx<'_>, flow: &mut Flow, item: &Item, props: &Props<'_>) {
    let Item {
        start,
        key,
        sep,
        value,
        ..
    } = item;
    let key = key.as_deref();
    let value = value.as_deref();
    ctx.at_key = true;
    let key_start = props.end;
    let mut key_node = match key {
        Some(key) => compose_node(ctx, key, props),
        None => compose_empty_node(ctx, key_start, Some(start), None, props),
    };
    if is_block(key) {
        ctx.error(key_node.range[0], BLOCK_MSG);
    }
    ctx.at_key = false;
    let value_props = resolve_props(
        ctx,
        sep.as_deref().unwrap_or_default(),
        Options {
            flow: Some(flow.name),
            indicator: Kind::MapValueInd,
            next: Next::of(value),
            offset: key_node.range[2],
            parent_indent: flow.indent,
            start_on_newline: false,
        },
    );
    pair_value_errors(ctx, flow, item, props, &value_props);
    let value_node = match value {
        Some(value) => Some(compose_node(ctx, value, &value_props)),
        None if value_props.found.is_some() => Some(compose_empty_node(
            ctx,
            value_props.end,
            sep.as_deref(),
            None,
            &value_props,
        )),
        None => None,
    };
    match &value_node {
        Some(node) => {
            if is_block(value) {
                ctx.error(node.range[0], BLOCK_MSG);
            }
        }
        None if !value_props.comment.is_empty() => {
            append(&mut key_node.comment, "\n", &value_props.comment)
        }
        None => {}
    }
    flow.offset = value_node
        .as_ref()
        .map_or(value_props.end, |node| node.range[2]);
    if flow.is_map {
        flow.items.push(Pair {
            key: Key::Node(Box::new(key_node)),
            value: value_node,
        });
    } else {
        let end_range = value_node
            .as_ref()
            .map_or(key_node.range, |node| node.range);
        let first = key_node.range[0];
        let mut map = ctx.node(Body::Map {
            items: Vec::new(),
            flow: true,
        });
        map.range = [first, end_range[1], end_range[2]];
        if let Body::Map { items, .. } = &mut map.body {
            items.push(Pair {
                key: Key::Node(Box::new(key_node)),
                value: value_node,
            });
        }
        flow.values.push(map);
    }
}

/// The errors of a flow pair's `:` placed wrong, or missing between a key and a value.
fn pair_value_errors(
    ctx: &mut Ctx<'_>,
    flow: &Flow,
    item: &Item,
    props: &Props<'_>,
    value_props: &Props<'_>,
) {
    let sep = item.sep.as_deref();
    let value = item.value.as_deref();
    if let Some(found) = value_props.found {
        if !flow.is_map && props.found.is_none() && ctx.strict {
            for token in sep.unwrap_or_default() {
                if std::ptr::eq(token, found) {
                    break;
                }
                if token.kind == Kind::Newline {
                    ctx.error(
                        token.offset,
                        "Implicit keys of flow sequence pairs need to be on a single line",
                    );
                    break;
                }
            }
            if props.start + 1024 < found.offset {
                ctx.error(
                    found.offset,
                    "The : indicator must be at most 1024 chars after the start of an implicit flow sequence key",
                );
            }
        }
    } else if let Some(value) = value {
        if starts_with_colon(value) {
            ctx.error(
                value.offset(),
                format!("Missing space after : in {}", flow.name),
            );
        } else {
            ctx.error(
                value_props.start,
                format!("Missing , or : between {} items", flow.name),
            );
        }
    }
}
