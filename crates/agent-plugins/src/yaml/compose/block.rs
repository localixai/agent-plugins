//! Block collections (`compose/resolve-block-map.js`, `compose/resolve-block-seq.js`): each
//! item's properties, key and value composed, with the errors of a misplaced one.
//!
//! Duplicate keys are not looked for: the reader's options set `uniqueKeys: false`.

use super::Ctx;
use super::end::contains_newline;
use super::node::{compose_empty_node, compose_node};
use super::props::{Next, Options, resolve_props};
use crate::yaml::cst::{Item, Kind, Token};
use crate::yaml::nodes::{Body, Key, Node, Pair, append};

const START_COL: &str = "All mapping items must start at the same column";

/// What `resolveBlockMap`'s loop keeps.
struct MapState {
    indent: usize,
    pairs: Vec<Pair>,
    comment: Option<String>,
    offset: usize,
    comment_end: Option<usize>,
}

/// `resolveBlockMap`.
pub(crate) fn resolve_block_map(ctx: &mut Ctx<'_>, token: &Token) -> Node {
    let Token::BlockMap {
        offset: map_offset,
        indent,
        items,
    } = token
    else {
        return ctx.node(Body::Map {
            items: Vec::new(),
            flow: false,
        });
    };
    ctx.at_root = false;
    let mut state = MapState {
        indent: *indent,
        pairs: Vec::new(),
        comment: None,
        offset: *map_offset,
        comment_end: None,
    };
    for item in items {
        block_map_item(ctx, &mut state, item);
    }
    let MapState {
        pairs,
        comment,
        offset,
        comment_end,
        ..
    } = state;
    if let Some(end) = comment_end.filter(|end| *end != 0 && *end < offset) {
        ctx.error(end, "Map comment with trailing content");
    }
    let mut map = ctx.node(Body::Map {
        items: pairs,
        flow: false,
    });
    map.comment = comment;
    map.range = [*map_offset, offset, comment_end.unwrap_or(offset)];
    map
}

/// One item of `resolveBlockMap`'s loop.
fn block_map_item(ctx: &mut Ctx<'_>, state: &mut MapState, item: &Item) {
    let Item {
        start, key, sep, ..
    } = item;
    let key = key.as_deref();
    let offset = state.offset;
    // The key's properties.
    let key_props = resolve_props(
        ctx,
        start,
        Options {
            flow: None,
            indicator: Kind::ExplicitKeyInd,
            next: Next::of(key).or_else(|| Next::of_source(sep.as_deref().and_then(<[_]>::first))),
            offset,
            parent_indent: state.indent,
            start_on_newline: true,
        },
    );
    let implicit_key = key_props.found.is_none();
    if implicit_key {
        if let Some(key) = key {
            if matches!(key, Token::BlockSeq { .. }) {
                ctx.error(
                    offset,
                    "A block sequence may not be used as an implicit map key",
                );
            } else if key
                .indent()
                .is_some_and(|key_indent| key_indent != state.indent)
            {
                ctx.error(offset, START_COL);
            }
        }
        if key_props.anchor.is_none() && key_props.tag.is_none() && sep.is_none() {
            state.comment_end = Some(key_props.end);
            if !key_props.comment.is_empty() {
                append(&mut state.comment, "\n", &key_props.comment);
            }
            return;
        }
        if key_props.newline_after_prop.is_some() || contains_newline(key) == Some(true) {
            let at = key
                .map(Token::offset)
                .or_else(|| start.last().map(|last| last.offset));
            ctx.error(
                at.unwrap_or(offset),
                "Implicit keys need to be on a single line",
            );
        }
    } else if key_props
        .found
        .is_some_and(|found| found.indent != state.indent)
    {
        ctx.error(offset, START_COL);
    }
    // The key.
    ctx.at_key = true;
    let key_node = match key {
        Some(key) => compose_node(ctx, key, &key_props),
        None => compose_empty_node(ctx, key_props.end, Some(start), None, &key_props),
    };
    ctx.at_key = false;
    block_map_value(ctx, state, item, key_node, (implicit_key, key_props.start));
}

/// The value of an item of `resolveBlockMap`, after its key: `(implicit key, where the key's
/// properties start)`.
fn block_map_value(
    ctx: &mut Ctx<'_>,
    state: &mut MapState,
    item: &Item,
    mut key_node: Node,
    (implicit_key, key_start): (bool, usize),
) {
    let key = item.key.as_deref();
    let value = item.value.as_deref();
    let sep = item.sep.as_deref();
    let value_props = resolve_props(
        ctx,
        sep.unwrap_or_default(),
        Options {
            flow: None,
            indicator: Kind::MapValueInd,
            next: Next::of(value),
            offset: key_node.range[2],
            parent_indent: state.indent,
            start_on_newline: key.is_none_or(|key| matches!(key, Token::BlockScalar { .. })),
        },
    );
    state.offset = value_props.end;
    let Some(found) = value_props.found else {
        // A key with no value.
        if implicit_key {
            ctx.error(
                key_node.range[0],
                "Implicit map keys need to be followed by map values",
            );
        }
        if !value_props.comment.is_empty() {
            append(&mut key_node.comment, "\n", &value_props.comment);
        }
        state.pairs.push(Pair {
            key: Key::Node(Box::new(key_node)),
            value: None,
        });
        return;
    };
    if implicit_key {
        if matches!(value, Some(Token::BlockMap { .. })) && !value_props.has_newline {
            ctx.error(
                state.offset,
                "Nested mappings are not allowed in compact mappings",
            );
        }
        if ctx.strict && key_start + 1024 < found.offset {
            ctx.error(
                key_node.range[0],
                "The : indicator must be at most 1024 chars after the start of an implicit block mapping key",
            );
        }
    }
    // The value.
    let value_node = match value {
        Some(value) => compose_node(ctx, value, &value_props),
        None => compose_empty_node(ctx, state.offset, sep, None, &value_props),
    };
    state.offset = value_node.range[2];
    state.pairs.push(Pair {
        key: Key::Node(Box::new(key_node)),
        value: Some(value_node),
    });
}

/// `resolveBlockSeq`.
pub(crate) fn resolve_block_seq(ctx: &mut Ctx<'_>, token: &Token) -> Node {
    let Token::BlockSeq {
        offset: seq_offset,
        indent,
        items,
    } = token
    else {
        return ctx.node(Body::Seq {
            items: Vec::new(),
            flow: false,
        });
    };
    ctx.at_root = false;
    ctx.at_key = false;
    let mut nodes = Vec::new();
    let mut seq_comment = None;
    let mut offset = *seq_offset;
    let mut comment_end = None;
    for Item { start, value, .. } in items {
        let value = value.as_deref();
        let props = resolve_props(
            ctx,
            start,
            Options {
                flow: None,
                indicator: Kind::SeqItemInd,
                next: Next::of(value),
                offset,
                parent_indent: *indent,
                start_on_newline: true,
            },
        );
        if props.found.is_none() {
            if props.anchor.is_some() || props.tag.is_some() || value.is_some() {
                if matches!(value, Some(Token::BlockSeq { .. })) {
                    ctx.error(
                        props.end,
                        "All sequence items must start at the same column",
                    );
                } else {
                    ctx.error(offset, "Sequence item without - indicator");
                }
            } else {
                comment_end = Some(props.end);
                if !props.comment.is_empty() {
                    seq_comment = Some(props.comment.clone());
                }
                continue;
            }
        }
        let node = match value {
            Some(value) => compose_node(ctx, value, &props),
            None => compose_empty_node(ctx, props.end, Some(start), None, &props),
        };
        offset = node.range[2];
        nodes.push(node);
    }
    let mut seq = ctx.node(Body::Seq {
        items: nodes,
        flow: false,
    });
    seq.comment = seq_comment;
    seq.range = [*seq_offset, offset, comment_end.unwrap_or(offset)];
    seq
}
