//! Long lines folded (`dist/stringify/foldFlowLines.js` and `lineLengthOverLimit` of
//! `stringifyString.js`), in UTF-16 code units as JavaScript counts them. The frontmatter writer
//! sets `lineWidth: 0`, which folds nothing; a collection key's text is written at the default
//! width of 80.

use std::collections::HashSet;

use crate::yaml::units::{Units, at, signed, string, unit, units, unsigned};

/// `minContentWidth`'s default.
const MIN_CONTENT_WIDTH: isize = 20;

/// `FOLD_FLOW`, `FOLD_BLOCK` and `FOLD_QUOTED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Flow,
    Block,
    Quoted,
}

/// `lineLengthOverLimit`: whether a line is longer than the width leaves after the indent.
pub(crate) fn line_length_over_limit(text: &str, line_width: usize, indent_length: usize) -> bool {
    if line_width == 0 {
        return false;
    }
    let text = units(text);
    let limit = signed(line_width) - signed(indent_length);
    let length = signed(text.len());
    if length <= limit {
        return false;
    }
    let mut start = 0;
    for (index, code) in text.iter().enumerate() {
        let index = signed(index);
        if *code == unit('\n') {
            if index - start > limit {
                return true;
            }
            start = index + 1;
            if length - start <= limit {
                return false;
            }
        }
    }
    true
}

/// `consumeMoreIndentedLines`: past the more-indented lines after `i + 1`, to the last line
/// break among them.
fn consume_more_indented_lines(text: &[u16], mut i: isize, indent: isize) -> isize {
    let blank = |code: Option<u16>| code == Some(unit(' ')) || code == Some(unit('\t'));
    let mut end = i;
    let mut start = i + 1;
    let mut ch = at(text, start);
    while blank(ch) {
        if i < start + indent {
            i += 1;
            ch = at(text, i);
        } else {
            loop {
                i += 1;
                ch = at(text, i);
                if ch.is_none_or(|code| code == unit('\n')) {
                    break;
                }
            }
            end = i;
            start = i + 1;
            ch = at(text, start);
        }
    }
    end
}

/// `foldFlowLines(text, indent, mode, { indentAtStart, lineWidth })`: the text with lines folded
/// at spaces, and whether a line could not be folded short enough (`onOverflow`).
pub(crate) fn fold_flow_lines(
    text: &str,
    indent: &str,
    mode: Mode,
    indent_at_start: Option<usize>,
    line_width: usize,
) -> (String, bool) {
    if line_width == 0 {
        return (text.to_owned(), false);
    }
    let source = units(text);
    let width = signed(line_width);
    let indent_length = signed(units(indent).len());
    let min_content = if width < MIN_CONTENT_WIDTH {
        0
    } else {
        MIN_CONTENT_WIDTH
    };
    let end_step = (1 + min_content).max(1 + width - indent_length);
    if signed(source.len()) <= end_step {
        return (text.to_owned(), false);
    }
    let mut folds: Vec<isize> = Vec::new();
    let mut escaped: HashSet<isize> = HashSet::new();
    let mut end = width - indent_length;
    if let Some(start) = indent_at_start.map(signed) {
        if start > width - min_content.max(2) {
            folds.push(0);
        } else {
            end = width - start;
        }
    }
    let mut split: Option<isize> = None;
    let mut prev: Option<u16> = None;
    let mut overflow = false;
    let mut i: isize = -1;
    let mut esc_start: isize = -1;
    let mut esc_end: isize = -1;
    if mode == Mode::Block {
        i = consume_more_indented_lines(&source, i, indent_length);
        if i != -1 {
            end = i + end_step;
        }
    }
    let blank = |code: Option<u16>| code == Some(unit(' ')) || code == Some(unit('\t'));
    loop {
        i += 1;
        let Some(mut ch) = at(&source, i) else { break };
        if mode == Mode::Quoted && ch == unit('\\') {
            esc_start = i;
            i += escape_length(at(&source, i + 1));
            esc_end = i;
        }
        if ch == unit('\n') {
            if mode == Mode::Block {
                i = consume_more_indented_lines(&source, i, indent_length);
            }
            end = i + indent_length + end_step;
            split = None;
        } else {
            if ch == unit(' ') && prev.is_some_and(|prev| !blank(Some(prev)) && prev != unit('\n'))
            {
                // A space surrounded by non-space can be replaced with a newline and the indent.
                let next = at(&source, i + 1);
                if next.is_some_and(|next| !blank(Some(next)) && next != unit('\n')) {
                    split = Some(i);
                }
            }
            if i >= end {
                if let Some(at_split) = split.take() {
                    folds.push(at_split);
                    end = at_split + end_step;
                } else if mode == Mode::Quoted {
                    // White space collected at the end may stretch past the line width.
                    while blank(prev) {
                        prev = Some(ch);
                        i += 1;
                        ch = at(&source, i).unwrap_or(0);
                        overflow = true;
                    }
                    // Account for a newline escape, but don't break a preceding escape.
                    let j = if i > esc_end + 1 {
                        i - 2
                    } else {
                        esc_start - 1
                    };
                    // Bail out if the line width is shorter than an escape.
                    if escaped.contains(&j) {
                        return (text.to_owned(), overflow);
                    }
                    folds.push(j);
                    escaped.insert(j);
                    end = j + end_step;
                    split = None;
                } else {
                    overflow = true;
                }
            }
        }
        prev = Some(ch);
    }
    if folds.is_empty() {
        return (text.to_owned(), overflow);
    }
    (
        apply_folds(&source, &folds, &escaped, indent, mode),
        overflow,
    )
}

/// How far past a `\` an escape goes, by the code unit after it: `\xXX`, `\uXXXX`,
/// `\UXXXXXXXX` or one character.
fn escape_length(next: Option<u16>) -> isize {
    match next.and_then(|next| char::from_u32(u32::from(next))) {
        Some('x') => 3,
        Some('u') => 5,
        Some('U') => 9,
        _ => 1,
    }
}

/// The text with a line break and the indent at each fold.
fn apply_folds(
    source: &[u16],
    folds: &[isize],
    escaped: &HashSet<isize>,
    indent: &str,
    mode: Mode,
) -> String {
    let slice = |from: isize, to: isize| -> Units {
        let from = unsigned(from.max(0)).min(source.len());
        let to = unsigned(to.max(0)).min(source.len());
        source.get(from..to.max(from)).unwrap_or_default().to_vec()
    };
    let length = signed(source.len());
    let mut res = slice(0, folds.first().copied().unwrap_or(0));
    let indent = units(indent);
    for (index, fold) in folds.iter().enumerate() {
        let end = folds
            .get(index + 1)
            .copied()
            .filter(|next| *next != 0)
            .unwrap_or(length);
        if *fold == 0 {
            res = vec![unit('\n')];
            res.extend_from_slice(&indent);
            res.extend(slice(0, end));
        } else {
            if mode == Mode::Quoted && escaped.contains(fold) {
                res.extend(at(source, *fold));
                res.push(unit('\\'));
            }
            res.push(unit('\n'));
            res.extend_from_slice(&indent);
            res.extend(slice(fold + 1, end));
        }
    }
    string(&res)
}
