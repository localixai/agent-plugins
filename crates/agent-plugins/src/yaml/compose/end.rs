//! Where a node ends (`compose/resolve-end.js`, `compose/util-contains-newline.js` and
//! `compose/util-empty-scalar-position.js`): the comment after it, whether a key spans lines,
//! and where an empty scalar sits.

use super::Ctx;
use super::props::comment_text;
use crate::yaml::cst::{Kind, SourceToken, Token};
use crate::yaml::units::{string, unit};

/// `resolveEnd`: the comment after a node, and where the node ends with it.
pub(crate) fn resolve_end(
    ctx: &mut Ctx<'_>,
    end: Option<&[SourceToken]>,
    mut offset: usize,
    req_space: bool,
) -> (String, usize) {
    let mut comment = String::new();
    let mut has_space = false;
    let mut sep = String::new();
    for token in end.unwrap_or_default() {
        match token.kind {
            Kind::Space => has_space = true,
            Kind::Comment => {
                if req_space && !has_space {
                    ctx.error(
                        token.offset,
                        "Comments must be separated from other tokens by white space characters",
                    );
                }
                let text = comment_text(&token.source);
                if comment.is_empty() {
                    comment = text;
                } else {
                    comment.push_str(&sep);
                    comment.push_str(&text);
                }
                sep.clear();
            }
            Kind::Newline => {
                if !comment.is_empty() {
                    sep.push_str(&string(&token.source));
                }
                has_space = true;
            }
            kind @ Kind::ByteOrderMark
            | kind @ Kind::DocMode
            | kind @ Kind::FlowErrorEnd
            | kind @ Kind::Scalar
            | kind @ Kind::DocStart
            | kind @ Kind::DocEnd
            | kind @ Kind::SeqItemInd
            | kind @ Kind::ExplicitKeyInd
            | kind @ Kind::MapValueInd
            | kind @ Kind::FlowMapStart
            | kind @ Kind::FlowMapEnd
            | kind @ Kind::FlowSeqStart
            | kind @ Kind::FlowSeqEnd
            | kind @ Kind::Comma
            | kind @ Kind::DirectiveLine
            | kind @ Kind::Alias
            | kind @ Kind::Anchor
            | kind @ Kind::Tag
            | kind @ Kind::SingleQuotedScalar
            | kind @ Kind::DoubleQuotedScalar
            | kind @ Kind::BlockScalarHeader => ctx.error(
                token.offset,
                format!("Unexpected {} at node end", kind.name()),
            ),
        }
        offset += token.source.len();
    }
    (comment, offset)
}

/// `containsNewline`: whether a key spans lines; `None` for no key.
pub(crate) fn contains_newline(key: Option<&Token>) -> Option<bool> {
    let newline = |tokens: &[SourceToken]| tokens.iter().any(|token| token.kind == Kind::Newline);
    Some(match key? {
        Token::FlowScalar(scalar) => {
            scalar.source.contains(&unit('\n')) || scalar.end.as_deref().is_some_and(newline)
        }
        Token::FlowCollection { items, .. } => items.iter().any(|item| {
            newline(&item.start)
                || item.sep.as_deref().is_some_and(newline)
                || contains_newline(item.key.as_deref()) == Some(true)
                || contains_newline(item.value.as_deref()) == Some(true)
        }),
        Token::Source(_)
        | Token::Error(_)
        | Token::Directive { .. }
        | Token::Document { .. }
        | Token::DocEnd { .. }
        | Token::BlockScalar { .. }
        | Token::BlockMap { .. }
        | Token::BlockSeq { .. } => true,
    })
}

/// `emptyScalarPosition`: where an empty node goes, after the blanks before `pos`.
pub(crate) fn empty_scalar_position(
    mut offset: usize,
    before: Option<&[SourceToken]>,
    pos: Option<usize>,
) -> usize {
    let Some(before) = before else {
        return offset;
    };
    let pos = pos.unwrap_or(before.len());
    for index in (0..pos).rev() {
        let Some(token) = before.get(index) else {
            continue;
        };
        if matches!(token.kind, Kind::Space | Kind::Comment | Kind::Newline) {
            offset = offset.saturating_sub(token.source.len());
            continue;
        }
        // Technically, an empty scalar is immediately after the last non-empty node, but it's
        // more useful to place it after any whitespace.
        for token in before.iter().skip(index + 1) {
            if token.kind != Kind::Space {
                break;
            }
            offset += token.source.len();
        }
        break;
    }
    offset
}
