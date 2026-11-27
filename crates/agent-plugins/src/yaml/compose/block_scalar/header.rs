//! A block scalar's header (`parseBlockScalarHeader` of `compose/resolve-block-scalar.js`):
//! `|` or `>`, the indentation and chomping indicators, and the comment after them.

use super::Header;
use crate::yaml::compose::Ctx;
use crate::yaml::cst::{Kind, Token};
use crate::yaml::units::{string, unit};

/// `parseBlockScalarHeader`.
pub(super) fn parse_block_scalar_header(
    ctx: &mut Ctx<'_>,
    offset: usize,
    props: &[Token],
) -> Option<Header> {
    let Some(Token::Source(first)) = props.first().filter(
        |first| matches!(first, Token::Source(token) if token.kind == Kind::BlockScalarHeader),
    ) else {
        ctx.error(
            props.first().map_or(offset, Token::offset),
            "Block scalar header not found",
        );
        return None;
    };
    let source = &first.source;
    let mut header = Header {
        folded: source.first() == Some(&unit('>')),
        indent: 0,
        chomp: None,
        comment: String::new(),
        length: source.len(),
    };
    let mut error = None;
    for (index, code) in source.iter().enumerate().skip(1) {
        if header.chomp.is_none() && (*code == unit('-') || *code == unit('+')) {
            header.chomp = Some(*code);
            continue;
        }
        let digit = char::from_u32(u32::from(*code)).and_then(|character| character.to_digit(10));
        match digit {
            Some(digit @ 1..=9) if header.indent == 0 => {
                header.indent = usize::try_from(digit).unwrap_or(0)
            }
            _ => {
                error.get_or_insert(offset + index);
            }
        }
    }
    if let Some(error) = error {
        ctx.error(
            error,
            format!(
                "Block scalar header includes extra characters: {}",
                string(source)
            ),
        );
    }
    let mut has_space = false;
    for token in props.iter().skip(1) {
        match token {
            Token::Source(token) if matches!(token.kind, Kind::Space | Kind::Newline) => {
                has_space |= token.kind == Kind::Space;
                header.length += token.source.len();
            }
            Token::Source(token) if token.kind == Kind::Comment => {
                if ctx.strict && !has_space {
                    ctx.error(
                        token.offset,
                        "Comments must be separated from other tokens by white space characters",
                    );
                }
                header.length += token.source.len();
                header.comment = string(token.source.get(1..).unwrap_or_default());
            }
            Token::Error(token) => {
                ctx.error(token.offset, token.message.clone());
                header.length += token.source.len();
            }
            token @ Token::Source(_)
            | token @ Token::Directive { .. }
            | token @ Token::Document { .. }
            | token @ Token::DocEnd { .. }
            | token @ Token::FlowScalar(_)
            | token @ Token::BlockScalar { .. }
            | token @ Token::BlockMap { .. }
            | token @ Token::BlockSeq { .. }
            | token @ Token::FlowCollection { .. } => {
                let message = format!(
                    "Unexpected token in block scalar header: {}",
                    token.type_name()
                );
                ctx.error(token.offset(), message);
                header.length += token.source().map_or(0, Vec::len);
            }
        }
    }
    Some(header)
}
