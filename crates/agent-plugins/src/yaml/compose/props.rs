//! What comes before and after a node (`compose/resolve-props.js`, `compose/resolve-end.js`,
//! `compose/util-contains-newline.js`, `compose/util-empty-scalar-position.js`): its anchor, tag,
//! indicator and comments, the comment after it, and where an empty node goes.

use super::Ctx;
use crate::yaml::cst::{Kind, SourceToken, Token};
use crate::yaml::units::{string, unit};

/// The token after a node's properties, as `resolveProps` looks at it: its type, offset and text.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Next<'t> {
    pub(crate) kind: &'static str,
    pub(crate) offset: usize,
    pub(crate) source: Option<&'t [u16]>,
}

impl<'t> Next<'t> {
    pub(crate) fn of(token: Option<&'t Token>) -> Option<Self> {
        token.map(|token| Self {
            kind: token.type_name(),
            offset: token.offset(),
            source: token.source().map(Vec::as_slice),
        })
    }

    pub(crate) fn of_source(token: Option<&'t SourceToken>) -> Option<Self> {
        token.map(|token| Self {
            kind: token.kind.name(),
            offset: token.offset,
            source: Some(token.source.as_slice()),
        })
    }
}

/// How `resolveProps` reads a node's leading tokens.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Options<'t> {
    /// `flow map` or `flow sequence` inside a flow collection.
    pub(crate) flow: Option<&'static str>,
    pub(crate) indicator: Kind,
    pub(crate) next: Option<Next<'t>>,
    pub(crate) offset: usize,
    pub(crate) parent_indent: usize,
    pub(crate) start_on_newline: bool,
}

/// A node's properties.
#[derive(Debug, Clone, Default)]
pub(crate) struct Props<'t> {
    pub(crate) comma: Option<&'t SourceToken>,
    pub(crate) found: Option<&'t SourceToken>,
    pub(crate) space_before: bool,
    pub(crate) comment: String,
    pub(crate) has_newline: bool,
    pub(crate) anchor: Option<&'t SourceToken>,
    pub(crate) tag: Option<&'t SourceToken>,
    pub(crate) newline_after_prop: Option<&'t SourceToken>,
    pub(crate) end: usize,
    pub(crate) start: usize,
}

/// `token.source.substring(1) || ' '`: a comment's text without its `#`.
pub(crate) fn comment_text(source: &[u16]) -> String {
    let text = string(source.get(1..).unwrap_or_default());
    if text.is_empty() {
        " ".to_owned()
    } else {
        text
    }
}

const REQ_SPACE: &str = "Tags and anchors must be separated from the next token by white space";
const TAB: &str = "Tabs are not allowed as indentation";

/// The state `resolveProps` keeps while it reads.
#[derive(Debug, Default)]
struct Reading<'t> {
    props: Props<'t>,
    at_newline: bool,
    has_space: bool,
    comment_sep: String,
    req_space: bool,
    tab: Option<&'t SourceToken>,
    start: Option<usize>,
}

/// `resolveProps`.
pub(crate) fn resolve_props<'t>(
    ctx: &mut Ctx<'_>,
    tokens: &'t [SourceToken],
    options: Options<'_>,
) -> Props<'t> {
    let mut state = Reading {
        at_newline: options.start_on_newline,
        has_space: options.start_on_newline,
        ..Reading::default()
    };
    for token in tokens {
        if state.req_space {
            if !matches!(token.kind, Kind::Space | Kind::Newline | Kind::Comma) {
                ctx.error(token.offset, REQ_SPACE);
            }
            state.req_space = false;
        }
        if let Some(tab) = state.tab.take()
            && state.at_newline
            && !matches!(token.kind, Kind::Comment | Kind::Newline)
        {
            ctx.error(tab.offset, TAB);
        }
        read_token(ctx, &mut state, token, &options);
    }
    let end = tokens
        .last()
        .map_or(options.offset, |last| last.offset + last.source.len());
    if state.req_space
        && let Some(next) = options.next
    {
        let empty_scalar = next.kind == "scalar" && next.source.is_some_and(<[u16]>::is_empty);
        if !matches!(next.kind, "space" | "newline" | "comma") && !empty_scalar {
            ctx.error(next.offset, REQ_SPACE);
        }
    }
    if let Some(tab) = state.tab {
        let block = options
            .next
            .is_some_and(|next| matches!(next.kind, "block-map" | "block-seq"));
        if (state.at_newline && tab.indent <= options.parent_indent) || block {
            ctx.error(tab.offset, TAB);
        }
    }
    let mut props = state.props;
    props.end = end;
    props.start = state.start.unwrap_or(end);
    props
}

/// One token of `resolveProps`' loop. The indicator is never one of the other kinds the loop
/// names, so it is looked at first.
fn read_token<'t>(
    ctx: &mut Ctx<'_>,
    state: &mut Reading<'t>,
    token: &'t SourceToken,
    options: &Options<'_>,
) {
    if token.kind == options.indicator {
        read_indicator(ctx, state, token, options);
        return;
    }
    match token.kind {
        Kind::Space => {
            // At the doc level, tabs at line start may be parsed as leading white space rather
            // than indentation. In a flow collection, only the parser handles indent.
            let flow_doc = options.indicator == Kind::DocStart
                && options
                    .next
                    .is_some_and(|next| next.kind == "flow-collection");
            if options.flow.is_none() && !flow_doc && token.source.contains(&unit('\t')) {
                state.tab = Some(token);
            }
            state.has_space = true;
        }
        Kind::Comment => read_comment(ctx, state, token),
        Kind::Newline => read_newline(state, token, options),
        Kind::Anchor | Kind::Tag => read_prop(ctx, state, token),
        Kind::Comma if options.flow.is_some() => {
            if state.props.comma.is_some() {
                let flow = options.flow.unwrap_or_default();
                ctx.error(token.offset, format!("Unexpected , in {flow}"));
            }
            state.props.comma = Some(token);
            state.at_newline = false;
            state.has_space = false;
        }
        kind @ (Kind::ByteOrderMark
        | Kind::DocMode
        | Kind::FlowErrorEnd
        | Kind::Scalar
        | Kind::DocStart
        | Kind::DocEnd
        | Kind::SeqItemInd
        | Kind::ExplicitKeyInd
        | Kind::MapValueInd
        | Kind::FlowMapStart
        | Kind::FlowMapEnd
        | Kind::FlowSeqStart
        | Kind::FlowSeqEnd
        | Kind::Comma
        | Kind::DirectiveLine
        | Kind::Alias
        | Kind::SingleQuotedScalar
        | Kind::DoubleQuotedScalar
        | Kind::BlockScalarHeader) => {
            ctx.error(token.offset, format!("Unexpected {} token", kind.name()));
            state.at_newline = false;
            state.has_space = false;
        }
    }
}

/// The `comment` case of `resolveProps`' loop.
fn read_comment(ctx: &mut Ctx<'_>, state: &mut Reading<'_>, token: &SourceToken) {
    if !state.has_space {
        ctx.error(
            token.offset,
            "Comments must be separated from other tokens by white space characters",
        );
    }
    let text = comment_text(&token.source);
    let props = &mut state.props;
    if props.comment.is_empty() {
        props.comment = text;
    } else {
        props.comment.push_str(&state.comment_sep);
        props.comment.push_str(&text);
    }
    state.comment_sep.clear();
    state.at_newline = false;
}

/// The `newline` case of `resolveProps`' loop.
fn read_newline<'t>(state: &mut Reading<'t>, token: &'t SourceToken, options: &Options<'_>) {
    let props = &mut state.props;
    if state.at_newline {
        if !props.comment.is_empty() {
            props.comment.push_str(&string(&token.source));
        } else if props.found.is_none() || options.indicator != Kind::SeqItemInd {
            props.space_before = true;
        }
    } else {
        state.comment_sep.push_str(&string(&token.source));
    }
    state.at_newline = true;
    props.has_newline = true;
    if props.anchor.is_some() || props.tag.is_some() {
        props.newline_after_prop = Some(token);
    }
    state.has_space = true;
}

/// The `anchor` and `tag` cases of `resolveProps`' loop. An anchor ending in `:` is a warning.
fn read_prop<'t>(ctx: &mut Ctx<'_>, state: &mut Reading<'t>, token: &'t SourceToken) {
    let (slot, message) = if token.kind == Kind::Anchor {
        (
            &mut state.props.anchor,
            "A node can have at most one anchor",
        )
    } else {
        (&mut state.props.tag, "A node can have at most one tag")
    };
    if slot.is_some() {
        ctx.error(token.offset, message);
    }
    *slot = Some(token);
    state.start.get_or_insert(token.offset);
    state.at_newline = false;
    state.has_space = false;
    state.req_space = true;
}

/// The indicator case of `resolveProps`' loop.
fn read_indicator<'t>(
    ctx: &mut Ctx<'_>,
    state: &mut Reading<'t>,
    token: &'t SourceToken,
    options: &Options<'_>,
) {
    let props = &mut state.props;
    let text = string(&token.source);
    if props.anchor.is_some() || props.tag.is_some() {
        ctx.error(
            token.offset,
            format!("Anchors and tags must be after the {text} indicator"),
        );
    }
    if props.found.is_some() {
        let place = options.flow.unwrap_or("collection");
        ctx.error(token.offset, format!("Unexpected {text} in {place}"));
    }
    props.found = Some(token);
    state.at_newline = matches!(options.indicator, Kind::SeqItemInd | Kind::ExplicitKeyInd);
    state.has_space = false;
}
