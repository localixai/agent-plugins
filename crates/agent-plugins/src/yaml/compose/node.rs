//! A node from its tokens (`compose/compose-node.js`, `compose/compose-collection.js`,
//! `compose/compose-scalar.js`): an alias, a scalar resolved by its tag, or a collection.

use super::Ctx;
use super::block::{resolve_block_map, resolve_block_seq};
use super::block_scalar::resolve_block_scalar;
use super::end::{empty_scalar_position, resolve_end};
use super::flow::resolve_flow_collection;
use super::flow_scalar::{Resolved, resolve_flow_scalar};
use super::props::Props;
use crate::yaml::cst::{FlowScalar, Kind, SourceToken, Token};
use crate::yaml::nodes::{Body, Node, Scalar};
use crate::yaml::tags::{CORE, Tag};
use crate::yaml::units::{string, unit};
use crate::yaml::{MAX_DEPTH, STACK_EXCEEDED};

/// `anchor.source.substring(1)`.
fn anchor_name(anchor: &SourceToken) -> String {
    string(anchor.source.get(1..).unwrap_or_default())
}

/// `composeNode`.
pub(crate) fn compose_node(ctx: &mut Ctx<'_>, token: &Token, props: &Props<'_>) -> Node {
    let mut node = match token {
        Token::FlowScalar(scalar) if scalar.kind == Kind::Alias => {
            let node = compose_alias(ctx, scalar);
            if props.anchor.is_some() || props.tag.is_some() {
                ctx.error(
                    scalar.offset,
                    "An alias node must not specify any properties",
                );
            }
            Some(node)
        }
        Token::FlowScalar(_) | Token::BlockScalar { .. } => {
            let mut node = compose_scalar(ctx, token, props.tag);
            node.anchor = props.anchor.map(anchor_name);
            Some(node)
        }
        Token::BlockMap { .. } | Token::BlockSeq { .. } | Token::FlowCollection { .. } => {
            if ctx.depth >= MAX_DEPTH {
                // Here the package is out of stack.
                ctx.error(token.offset(), STACK_EXCEEDED);
                None
            } else {
                ctx.depth += 1;
                let mut node = compose_collection(ctx, token, props);
                ctx.depth -= 1;
                node.anchor = props.anchor.map(anchor_name);
                Some(node)
            }
        }
        Token::Error(error) => {
            ctx.error(error.offset, error.message.clone());
            None
        }
        Token::Source(_)
        | Token::Directive { .. }
        | Token::Document { .. }
        | Token::DocEnd { .. } => {
            ctx.error(
                token.offset(),
                format!("Unsupported token (type: {})", token.type_name()),
            );
            None
        }
    }
    .unwrap_or_else(|| compose_empty_node(ctx, token.offset(), None, None, props));
    if let Some(anchor) = props.anchor
        && node.anchor.as_deref() == Some("")
    {
        ctx.error(anchor.offset, "Anchor cannot be an empty string");
    }
    if props.space_before {
        node.space_before = true;
    }
    if !props.comment.is_empty() {
        let empty_scalar = matches!(token, Token::FlowScalar(scalar) if scalar.kind == Kind::Scalar && scalar.source.is_empty());
        if empty_scalar {
            node.comment = Some(props.comment.clone());
        } else {
            node.comment_before = Some(props.comment.clone());
        }
    }
    node
}

/// `composeEmptyNode`.
pub(crate) fn compose_empty_node(
    ctx: &mut Ctx<'_>,
    offset: usize,
    before: Option<&[SourceToken]>,
    pos: Option<usize>,
    props: &Props<'_>,
) -> Node {
    let token = Token::FlowScalar(FlowScalar {
        kind: Kind::Scalar,
        offset: empty_scalar_position(offset, before, pos),
        indent: 0,
        source: Vec::new(),
        end: None,
    });
    let mut node = compose_scalar(ctx, &token, props.tag);
    if let Some(anchor) = props.anchor {
        let name = anchor_name(anchor);
        if name.is_empty() {
            ctx.error(anchor.offset, "Anchor cannot be an empty string");
        }
        node.anchor = Some(name);
    }
    if props.space_before {
        node.space_before = true;
    }
    if !props.comment.is_empty() {
        node.comment = Some(props.comment.clone());
        node.range[2] = props.end;
    }
    node
}

/// `composeAlias`.
fn compose_alias(ctx: &mut Ctx<'_>, token: &FlowScalar) -> Node {
    let source = string(token.source.get(1..).unwrap_or_default());
    if source.is_empty() {
        ctx.error(token.offset, "Alias cannot be an empty string");
    }
    // An alias ending in `:` is a warning.
    let value_end = token.offset + token.source.len();
    let strict = ctx.strict;
    let (comment, end) = resolve_end(ctx, token.end.as_deref(), value_end, strict);
    let mut node = ctx.node(Body::Alias(source));
    node.range = [token.offset, value_end, end];
    if !comment.is_empty() {
        node.comment = Some(comment);
    }
    node
}

/// `ctx.directives.tagName(tagToken.source, …)`.
fn tag_name(ctx: &mut Ctx<'_>, tag: &SourceToken) -> Option<String> {
    let mut messages = Vec::new();
    let name = ctx
        .directives
        .tag_name(&string(&tag.source), &mut |message| messages.push(message));
    for message in messages {
        ctx.error(tag.offset, message);
    }
    name
}

/// `composeCollection`.
fn compose_collection(ctx: &mut Ctx<'_>, token: &Token, props: &Props<'_>) -> Node {
    let tag_name = props.tag.and_then(|tag| tag_name(ctx, tag));
    if matches!(token, Token::BlockSeq { .. }) {
        let last_prop = match (props.anchor, props.tag) {
            (Some(anchor), Some(tag)) => Some(if anchor.offset > tag.offset {
                anchor
            } else {
                tag
            }),
            (anchor, tag) => anchor.or(tag),
        };
        if let Some(last) = last_prop
            && props
                .newline_after_prop
                .is_none_or(|newline| newline.offset < last.offset)
        {
            ctx.error(last.offset, "Missing newline after block sequence props");
        }
    }
    // A tag of the core schema that names another kind of collection, or one it does not know,
    // is a warning: the collection keeps that tag.
    let mut coll = match token {
        Token::BlockMap { .. } => resolve_block_map(ctx, token),
        Token::BlockSeq { .. } => resolve_block_seq(ctx, token),
        Token::Source(_)
        | Token::Error(_)
        | Token::Directive { .. }
        | Token::Document { .. }
        | Token::DocEnd { .. }
        | Token::FlowScalar(_)
        | Token::BlockScalar { .. }
        | Token::FlowCollection { .. } => resolve_flow_collection(ctx, token),
    };
    let own = match coll.body {
        Body::Map { .. } => Tag::Map.tag(),
        Body::Scalar(_) | Body::Seq { .. } | Body::Alias(_) => Tag::Seq.tag(),
    };
    match tag_name.as_deref() {
        Some("!") => coll.tag = Some(own.to_owned()),
        Some(name) => coll.tag = Some(name.to_owned()),
        None => {}
    }
    coll
}

/// `composeScalar`.
pub(crate) fn compose_scalar(
    ctx: &mut Ctx<'_>,
    token: &Token,
    tag_token: Option<&SourceToken>,
) -> Node {
    let Resolved {
        value,
        kind,
        comment,
        range,
    } = match token {
        Token::BlockScalar {
            offset,
            indent,
            props,
            source,
        } => resolve_block_scalar(ctx, *offset, *indent, props, source),
        Token::FlowScalar(scalar) => {
            let strict = ctx.strict;
            resolve_flow_scalar(ctx, scalar, strict)
        }
        Token::Source(_)
        | Token::Error(_)
        | Token::Directive { .. }
        | Token::Document { .. }
        | Token::DocEnd { .. }
        | Token::BlockMap { .. }
        | Token::BlockSeq { .. }
        | Token::FlowCollection { .. } => Resolved {
            value: Vec::new(),
            kind: None,
            comment: String::new(),
            range: [token.offset(); 3],
        },
    };
    let value = string(&value);
    let tag_name = tag_token.and_then(|tag| tag_name(ctx, tag));
    let plain = matches!(token, Token::FlowScalar(scalar) if scalar.kind == Kind::Scalar);
    let tag = match tag_name.as_deref() {
        Some(name) => find_scalar_tag_by_name(&value, name),
        None if plain => find_scalar_tag_by_test(&value),
        None => Tag::Str,
    };
    let resolved = tag.resolve(&value);
    let mut node = ctx.node(Body::Scalar(Scalar {
        source: Some(value),
        kind,
        format: tag.format(),
        ..resolved
    }));
    node.range = range;
    node.tag = tag_name;
    if !comment.is_empty() {
        node.comment = Some(comment);
    }
    node
}

/// `findScalarTagByName`. A tag the core schema lacks resolves as a string, which is a warning.
fn find_scalar_tag_by_name(value: &str, tag_name: &str) -> Tag {
    if tag_name == "!" {
        // The non-specific tag.
        return Tag::Str;
    }
    let named = || {
        CORE.into_iter()
            .filter(|tag| tag.collection().is_none() && tag.tag() == tag_name)
    };
    if let Some(tag) = named().find(|tag| !tag.has_test()) {
        return tag;
    }
    named().find(|tag| tag.test(value)).unwrap_or(Tag::Str)
}

/// `findScalarTagByTest`.
fn find_scalar_tag_by_test(value: &str) -> Tag {
    CORE.into_iter()
        .find(|tag| tag.test(value))
        .unwrap_or(Tag::Str)
}

/// Whether a token's text starts with `:`.
pub(crate) fn starts_with_colon(token: &Token) -> bool {
    token
        .source()
        .is_some_and(|source| source.first() == Some(&unit(':')))
}
