//! Flow collections (`compose/resolve-flow-collection.js`): `{…}` and `[…]` with their items,
//! pairs in a sequence as one-pair maps, and the errors of a missing comma, colon or end.
//!
//! Duplicate keys are not looked for: the reader's options set `uniqueKeys: false`.

mod pair;

use self::pair::{flow_pair, move_previous_comment};
use super::Ctx;
use super::end::{contains_newline, resolve_end};
use super::node::{compose_empty_node, compose_node};
use super::props::{Next, Options, resolve_props};
use crate::yaml::cst::{Item, Kind, SourceToken, Token};
use crate::yaml::nodes::{Body, Node, Pair, append};
use crate::yaml::units::unit;

const BLOCK_MSG: &str = "Block collections are not allowed within flow collections";

const fn is_block(token: Option<&Token>) -> bool {
    matches!(token, Some(Token::BlockMap { .. } | Token::BlockSeq { .. }))
}

/// What the loop over a flow collection's items keeps.
struct Flow {
    is_map: bool,
    name: &'static str,
    items: Vec<Pair>,
    values: Vec<Node>,
    comment: Option<String>,
    offset: usize,
    indent: usize,
    count: usize,
}

/// `resolveFlowCollection`.
pub(crate) fn resolve_flow_collection(ctx: &mut Ctx<'_>, token: &Token) -> Node {
    let Token::FlowCollection {
        offset: fc_offset,
        indent,
        start,
        items,
        end,
    } = token
    else {
        return ctx.node(Body::Seq {
            items: Vec::new(),
            flow: true,
        });
    };
    let is_map = start.source == [unit('{')];
    let mut flow = Flow {
        is_map,
        name: if is_map { "flow map" } else { "flow sequence" },
        items: Vec::new(),
        values: Vec::new(),
        comment: None,
        offset: fc_offset + start.source.len(),
        indent: *indent,
        count: items.len(),
    };
    let at_root = ctx.at_root;
    ctx.at_root = false;
    ctx.at_key = false;
    for (index, item) in items.iter().enumerate() {
        flow_item(ctx, &mut flow, index, item);
    }
    let expected_end = if is_map { '}' } else { ']' };
    let mut rest: Vec<SourceToken> = end.iter().skip(1).cloned().collect();
    let first_end = end.first();
    let mut ce_pos = flow.offset;
    match first_end {
        Some(ce) if ce.source == [unit(expected_end)] => ce_pos = ce.offset + ce.source.len(),
        _ => {
            let name = if is_map { "Flow map" } else { "Flow sequence" };
            let message = if at_root {
                format!("{name} must end with a {expected_end}")
            } else {
                format!(
                    "{name} in block collection must be sufficiently indented and end with a {expected_end}"
                )
            };
            ctx.error(flow.offset, message);
            if let Some(ce) = first_end.filter(|ce| ce.source.len() != 1) {
                rest.insert(0, ce.clone());
            }
        }
    }
    let body = if is_map {
        Body::Map {
            items: flow.items,
            flow: true,
        }
    } else {
        Body::Seq {
            items: flow.values,
            flow: true,
        }
    };
    let mut coll = ctx.node(body);
    coll.comment = flow.comment;
    if rest.is_empty() {
        coll.range = [*fc_offset, ce_pos, ce_pos];
    } else {
        let strict = ctx.strict;
        let (comment, end) = resolve_end(ctx, Some(&rest), ce_pos, strict);
        if !comment.is_empty() {
            append(&mut coll.comment, "\n", &comment);
        }
        coll.range = [*fc_offset, ce_pos, end];
    }
    coll
}

/// One item of `resolveFlowCollection`'s loop.
fn flow_item(ctx: &mut Ctx<'_>, flow: &mut Flow, index: usize, item: &Item) {
    let Item {
        start,
        key,
        sep,
        value,
        ..
    } = item;
    let key = key.as_deref();
    let value = value.as_deref();
    let mut props = resolve_props(
        ctx,
        start,
        Options {
            flow: Some(flow.name),
            indicator: Kind::ExplicitKeyInd,
            next: Next::of(key).or_else(|| Next::of_source(sep.as_deref().and_then(<[_]>::first))),
            offset: flow.offset,
            parent_indent: flow.indent,
            start_on_newline: false,
        },
    );
    if props.found.is_none() {
        if props.anchor.is_none() && props.tag.is_none() && sep.is_none() && value.is_none() {
            if let (0, Some(comma)) = (index, props.comma) {
                ctx.error(comma.offset, format!("Unexpected , in {}", flow.name));
            } else if index + 1 < flow.count {
                ctx.error(
                    props.start,
                    format!("Unexpected empty item in {}", flow.name),
                );
            }
            if !props.comment.is_empty() {
                append(&mut flow.comment, "\n", &props.comment);
            }
            flow.offset = props.end;
            return;
        }
        if !flow.is_map && ctx.strict && contains_newline(key) == Some(true) {
            let at = key.map_or(flow.offset, Token::offset);
            ctx.error(
                at,
                "Implicit keys of flow sequence pairs need to be on a single line",
            );
        }
    }
    if index == 0 {
        if let Some(comma) = props.comma {
            ctx.error(comma.offset, format!("Unexpected , in {}", flow.name));
        }
    } else {
        if props.comma.is_none() {
            ctx.error(
                props.start,
                format!("Missing , between {} items", flow.name),
            );
        }
        if !props.comment.is_empty() {
            move_previous_comment(flow, start, &mut props);
        }
    }
    if !flow.is_map && sep.is_none() && props.found.is_none() {
        // An item that is a value in a sequence: key and sep are empty, start has no `?` or `:`.
        let value_node = match value {
            Some(value) => compose_node(ctx, value, &props),
            None => compose_empty_node(ctx, props.end, sep.as_deref(), None, &props),
        };
        flow.offset = value_node.range[2];
        if is_block(value) {
            ctx.error(value_node.range[0], BLOCK_MSG);
        }
        flow.values.push(value_node);
    } else {
        flow_pair(ctx, flow, item, &props);
    }
}
