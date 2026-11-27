//! A plain, single-quoted or double-quoted scalar's value (`compose/resolve-flow-scalar.js`):
//! lines folded, quotes and escapes resolved, in UTF-16 code units as the package counts them.

mod double;

use self::double::double_quoted_value;
use super::Ctx;
use super::end::resolve_end;
use crate::yaml::cst::{FlowScalar, Kind};
use crate::yaml::nodes::ScalarType;
use crate::yaml::units::{Units, unit};

/// What a scalar resolves to: its text, how it was written, the comment after it and its range.
#[derive(Debug, Clone)]
pub(crate) struct Resolved {
    pub(crate) value: Units,
    pub(crate) kind: Option<ScalarType>,
    pub(crate) comment: String,
    pub(crate) range: [usize; 3],
}

/// `resolveFlowScalar`.
pub(crate) fn resolve_flow_scalar(
    ctx: &mut Ctx<'_>,
    scalar: &FlowScalar,
    strict: bool,
) -> Resolved {
    let FlowScalar {
        offset,
        kind,
        source,
        end,
        ..
    } = scalar;
    let (kind, value) = match kind {
        Kind::SingleQuotedScalar => (
            ScalarType::QuoteSingle,
            single_quoted_value(ctx, *offset, source),
        ),
        Kind::DoubleQuotedScalar => (
            ScalarType::QuoteDouble,
            double_quoted_value(ctx, *offset, source),
        ),
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
        | Kind::Comma
        | Kind::Space
        | Kind::Comment
        | Kind::DirectiveLine
        | Kind::Alias
        | Kind::Anchor
        | Kind::Tag
        | Kind::BlockScalarHeader => (ScalarType::Plain, plain_value(ctx, *offset, source)),
    };
    let value_end = offset + source.len();
    let (comment, end) = resolve_end(ctx, end.as_deref(), value_end, strict);
    Resolved {
        value,
        kind: Some(kind),
        comment,
        range: [*offset, value_end, end],
    }
}

fn plain_value(ctx: &mut Ctx<'_>, offset: usize, source: &[u16]) -> Units {
    let first = source
        .first()
        .and_then(|first| char::from_u32(u32::from(*first)));
    let bad = match first {
        Some('\t') => "a tab character".to_owned(),
        Some(',') => "flow indicator character ,".to_owned(),
        Some('%') => "directive indicator character %".to_owned(),
        Some(character @ ('|' | '>')) => format!("block scalar indicator {character}"),
        Some(character @ ('@' | '`')) => format!("reserved character {character}"),
        _ => String::new(),
    };
    if !bad.is_empty() {
        ctx.error(offset, format!("Plain value cannot start with {bad}"));
    }
    unfold_lines(source)
}

fn single_quoted_value(ctx: &mut Ctx<'_>, offset: usize, source: &[u16]) -> Units {
    if source.last() != Some(&unit('\'')) || source.len() == 1 {
        ctx.error(offset + source.len(), "Missing closing 'quote");
    }
    let inner = source
        .get(1..source.len().saturating_sub(1))
        .unwrap_or_default();
    let unfolded = unfold_lines(inner);
    let mut out = Units::with_capacity(unfolded.len());
    let mut index = 0;
    while let Some(&code) = unfolded.get(index) {
        out.push(code);
        index += if code == unit('\'') && unfolded.get(index + 1) == Some(&code) {
            2
        } else {
            1
        };
    }
    out
}

const fn is_blank(code: u16) -> bool {
    code == unit(' ') || code == unit('\t')
}

/// `[ \t]+$` removed.
fn trim_end_blanks(line: &[u16]) -> &[u16] {
    let end = line
        .iter()
        .rposition(|code| !is_blank(*code))
        .map_or(0, |at| at + 1);
    line.get(..end).unwrap_or_default()
}

/// `^[ \t]+` removed.
fn trim_start_blanks(line: &[u16]) -> &[u16] {
    let start = line
        .iter()
        .position(|code| !is_blank(*code))
        .unwrap_or(line.len());
    line.get(start..).unwrap_or_default()
}

/// `unfoldLines`: each line break folded to a space, a run of them to one break less, the blanks
/// around them dropped.
fn unfold_lines(source: &[u16]) -> Units {
    // `/(.*?)\r?\n/sy`: each line up to its break, without a `\r` before it.
    let mut lines = Vec::new();
    let mut pos = 0;
    while let Some(break_at) = source
        .get(pos..)
        .and_then(|rest| rest.iter().position(|code| *code == unit('\n')))
    {
        let end = pos + break_at;
        let content_end = if end > pos && source.get(end - 1) == Some(&unit('\r')) {
            end - 1
        } else {
            end
        };
        lines.push(source.get(pos..content_end).unwrap_or_default());
        pos = end + 1;
    }
    let Some((first, rest)) = lines.split_first() else {
        return source.to_vec();
    };
    let mut res = trim_end_blanks(first).to_vec();
    let mut sep = unit(' ');
    for line in rest {
        let line = trim_end_blanks(trim_start_blanks(line));
        if line.is_empty() {
            if sep == unit('\n') {
                res.push(sep);
            } else {
                sep = unit('\n');
            }
        } else {
            res.push(sep);
            res.extend_from_slice(line);
            sep = unit(' ');
        }
    }
    res.push(sep);
    res.extend_from_slice(trim_start_blanks(source.get(pos..).unwrap_or_default()));
    res
}

#[cfg(test)]
mod tests {
    use super::unfold_lines;
    use crate::yaml::units::{string, units};

    /// What the package answers: `bun -e "import { parse } from 'yaml'; for (const s of
    /// ['a\n b', 'a \n\n  b  \n c', 'x\r\ny']) console.log(JSON.stringify(parse(s)))"`.
    #[test]
    fn unfolds_as_the_package_does() {
        assert_eq!(string(&unfold_lines(&units("a\n b"))), "a b");
        assert_eq!(string(&unfold_lines(&units("a \n\n  b  \n c"))), "a\nb c");
        assert_eq!(string(&unfold_lines(&units("x\r\ny"))), "x y");
    }
}
