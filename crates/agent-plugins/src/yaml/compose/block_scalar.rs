//! A literal or folded block scalar's value (`compose/resolve-block-scalar.js`): its header read,
//! its lines unindented, folded and chomped, in UTF-16 code units as the package counts them.

mod header;

use self::header::parse_block_scalar_header;
use super::Ctx;
use super::flow_scalar::Resolved;
use crate::yaml::cst::Token;
use crate::yaml::nodes::ScalarType;
use crate::yaml::units::{Units, unit};

/// What the header `|`, `>`, `|2-` or `>+` says.
#[derive(Debug, Clone)]
struct Header {
    folded: bool,
    indent: usize,
    chomp: Option<u16>,
    comment: String,
    length: usize,
}

/// `resolveBlockScalar`.
pub(crate) fn resolve_block_scalar(
    ctx: &mut Ctx<'_>,
    offset: usize,
    indent: usize,
    props: &[Token],
    source: &[u16],
) -> Resolved {
    let start = offset;
    let Some(header) = parse_block_scalar_header(ctx, offset, props) else {
        return Resolved {
            value: Units::new(),
            kind: None,
            comment: String::new(),
            range: [start; 3],
        };
    };
    let kind = if header.folded {
        ScalarType::BlockFolded
    } else {
        ScalarType::BlockLiteral
    };
    let lines = if source.is_empty() {
        Vec::new()
    } else {
        split_lines(source)
    };
    let empty = |content: &[u16]| content.is_empty() || content == [unit('\r')];
    // Determine the end of content and the start of chomping.
    let mut chomp_start = lines.len();
    for (index, (_, content)) in lines.iter().enumerate().rev() {
        if !empty(content) {
            break;
        }
        chomp_start = index;
    }
    let end = start + header.length + source.len();
    // A shortcut for empty contents.
    if chomp_start == 0 {
        let value = if header.chomp == Some(unit('+')) && !lines.is_empty() {
            vec![unit('\n'); lines.len().saturating_sub(1).max(1)]
        } else {
            Units::new()
        };
        return Resolved {
            value,
            kind: Some(kind),
            comment: header.comment,
            range: [start, end, end],
        };
    }
    let Indent {
        trim_indent,
        offset,
        content_start,
    } = find_indent(
        ctx,
        &header,
        indent,
        offset,
        lines.get(..chomp_start).unwrap_or_default(),
    );
    // Include trailing more-indented empty lines in the content.
    if let Some(index) = (chomp_start..lines.len()).rev().find(|index| {
        lines
            .get(*index)
            .is_some_and(|(spaces, _)| spaces.len() > trim_indent)
    }) {
        chomp_start = index + 1;
    }
    let trimmed = |spaces: &[u16]| spaces.get(trim_indent..).unwrap_or_default().to_vec();
    let mut value = Units::new();
    // Leading whitespace is kept intact.
    for (spaces, _) in lines.iter().take(content_start) {
        value.extend(trimmed(spaces));
        value.push(unit('\n'));
    }
    let body = Body {
        header: &header,
        trim_indent,
        offset,
    };
    let lines_of_content = lines.get(content_start..chomp_start).unwrap_or_default();
    body.fold(ctx, lines_of_content, &mut value);
    match header.chomp {
        Some(sign) if sign == unit('-') => {}
        Some(_) => {
            for (spaces, _) in lines.iter().skip(chomp_start) {
                value.push(unit('\n'));
                value.extend(trimmed(spaces));
            }
            if value.last() != Some(&unit('\n')) {
                value.push(unit('\n'));
            }
        }
        None => value.push(unit('\n')),
    }
    Resolved {
        value,
        kind: Some(kind),
        comment: header.comment,
        range: [start, end, end],
    }
}

/// Where `resolveBlockScalar`'s content starts: the indentation trimmed, the offset of the first
/// content line and its index.
struct Indent {
    trim_indent: usize,
    offset: usize,
    content_start: usize,
}

/// `resolveBlockScalar`'s "find the indentation level to trim from the start", over the lines
/// before chomping.
fn find_indent(
    ctx: &mut Ctx<'_>,
    header: &Header,
    indent: usize,
    offset: usize,
    lines: &[(&[u16], &[u16])],
) -> Indent {
    let empty = |content: &[u16]| content.is_empty() || content == [unit('\r')];
    let mut trim_indent = indent + header.indent;
    let mut offset = offset + header.length;
    let mut content_start = 0;
    for (index, (spaces, content)) in lines.iter().enumerate() {
        if empty(content) {
            if header.indent == 0 && spaces.len() > trim_indent {
                trim_indent = spaces.len();
            }
        } else {
            if spaces.len() < trim_indent {
                ctx.error(
                    offset + spaces.len(),
                    "Block scalars with more-indented leading empty lines must use an explicit indentation indicator",
                );
            }
            if header.indent == 0 {
                trim_indent = spaces.len();
            }
            content_start = index;
            if trim_indent == 0 && !ctx.at_root {
                ctx.error(
                    offset,
                    "Block scalar values in collections must be indented",
                );
            }
            break;
        }
        offset += spaces.len() + content.len() + 1;
    }
    Indent {
        trim_indent,
        offset,
        content_start,
    }
}

/// The content lines' pass of `resolveBlockScalar`.
struct Body<'h> {
    header: &'h Header,
    trim_indent: usize,
    offset: usize,
}

impl Body<'_> {
    fn fold(mut self, ctx: &mut Ctx<'_>, lines: &[(&[u16], &[u16])], value: &mut Units) {
        let mut sep = Units::new();
        let mut prev_more_indented = false;
        let newline = [unit('\n')];
        for (spaces, content) in lines {
            let (mut spaces, mut content) = (*spaces, *content);
            self.offset += spaces.len() + content.len() + 1;
            let crlf = content.last() == Some(&unit('\r'));
            if crlf {
                content = content.get(..content.len() - 1).unwrap_or_default();
            }
            if !content.is_empty() && spaces.len() < self.trim_indent {
                let what = if self.header.indent > 0 {
                    "explicit indentation indicator"
                } else {
                    "first line"
                };
                let at = self
                    .offset
                    .saturating_sub(content.len() + if crlf { 2 } else { 1 });
                ctx.error(
                    at,
                    format!("Block scalar lines must not be less indented than their {what}"),
                );
                spaces = &[];
            }
            let indented = spaces.get(self.trim_indent..).unwrap_or_default();
            if !self.header.folded {
                value.extend_from_slice(&sep);
                value.extend_from_slice(indented);
                value.extend_from_slice(content);
                sep = newline.to_vec();
            } else if spaces.len() > self.trim_indent || content.first() == Some(&unit('\t')) {
                // More-indented content within a folded block.
                if sep == [unit(' ')] {
                    sep = newline.to_vec();
                } else if !prev_more_indented && sep == newline {
                    sep = vec![unit('\n'); 2];
                }
                value.extend_from_slice(&sep);
                value.extend_from_slice(indented);
                value.extend_from_slice(content);
                sep = newline.to_vec();
                prev_more_indented = true;
            } else if content.is_empty() {
                // An empty line.
                if sep == newline {
                    value.push(unit('\n'));
                } else {
                    sep = newline.to_vec();
                }
            } else {
                value.extend_from_slice(&sep);
                value.extend_from_slice(content);
                sep = vec![unit(' ')];
                prev_more_indented = false;
            }
        }
    }
}

/// `splitLines`: each line as its leading spaces and the rest.
fn split_lines(source: &[u16]) -> Vec<(&[u16], &[u16])> {
    let spaces_of = |line: &[u16]| line.iter().take_while(|code| **code == unit(' ')).count();
    let mut lines = Vec::new();
    for (index, line) in source.split(|code| *code == unit('\n')).enumerate() {
        let count = if index == 0 || !line.is_empty() {
            spaces_of(line)
        } else {
            0
        };
        let (spaces, content) = line.split_at(count.min(line.len()));
        lines.push((spaces, content));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::split_lines;
    use crate::yaml::units::{string, units};

    /// `source.split(/\n( *)/)` paired as the package pairs it: the first line split at its
    /// spaces, every later one at the spaces after its break.
    #[test]
    fn splits_lines_at_their_spaces() {
        let source = units("  a\n b\n\n   ");
        let lines: Vec<(String, String)> = split_lines(&source)
            .into_iter()
            .map(|(spaces, content)| (string(spaces), string(content)))
            .collect();
        let expected = [("  ", "a"), (" ", "b"), ("", ""), ("   ", "")];
        assert_eq!(lines, expected.map(|(a, b)| (a.to_owned(), b.to_owned())));
    }
}
