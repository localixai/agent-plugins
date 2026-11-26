//! Ported from `src/rules/bash/heredoc.ts`; only the core uses it now, so it has no wrapper.
//!
//! Here-documents (bash(1), "Here Documents"): `<<word` or `<<-word`. The delimiter is the whole
//! word with its quotes removed, so `'E'OF` ends at a line `EOF`; any quoted part makes the body
//! plain text, otherwise the body still undergoes "parameter and variable expansion, command
//! substitution, and arithmetic expansion". The body starts on the line after the opener and ends
//! at the first line that is the delimiter (after leading tabs for `<<-`).
//!
//! Positions are UTF-16 code units of the text, as the TypeScript counts them (`js::utf16`).

use serde::{Deserialize, Serialize};

use crate::js::utf16::{index_of, slice, unit_at, units};
use crate::rules::bash::unquote::read_word;

/// One here-document opened on a line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heredoc {
    /// The delimiter word with its quoting removed.
    pub delimiter: String,
    /// `<<-` lets the closing line be indented with tabs.
    pub indented: bool,
    /// A quote or backslash in the delimiter word: the body is literal text.
    pub quoted: bool,
}

/// The opener a text holds at a position: its length and whether it is `<<-`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Opener {
    /// The opener's length, the blanks after it included.
    pub length: usize,
    /// It is `<<-`.
    pub indented: bool,
}

/// Where a body ends and what follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Body {
    /// The closing line's start (the text's end when no line closes it).
    pub end: usize,
    /// What follows the closing line.
    pub next: usize,
}

const LESS: u16 = 0x3c;
const DASH: u16 = 0x2d;
const TAB: u16 = 0x09;
const SPACE: u16 = 0x20;

/// The opener at `pos`: its length and whether it is `<<-`, or `None`.
///
/// `<<` or `<<-` and the blanks after it; the `<<<` of a here-string is no opener:
/// `/(?<!<)<<(?!<)(-?)[ \t]*/y`.
#[must_use]
pub fn heredoc_opener(src: &[u16], pos: usize) -> Option<Opener> {
    let after_less = pos.checked_sub(1).and_then(|before| unit_at(src, before)) == Some(LESS);
    if after_less || unit_at(src, pos) != Some(LESS) || unit_at(src, pos + 1) != Some(LESS) {
        return None;
    }
    if unit_at(src, pos + 2) == Some(LESS) {
        return None;
    }
    let indented = unit_at(src, pos + 2) == Some(DASH);
    let mut end = pos + 2 + usize::from(indented);
    while matches!(unit_at(src, end), Some(TAB | SPACE)) {
        end += 1;
    }
    Some(Opener {
        length: end - pos,
        indented,
    })
}

/// The here-document a delimiter word (as written) opens.
#[must_use]
pub fn heredoc_of(word: &str, indented: bool) -> Heredoc {
    Heredoc {
        delimiter: read_word(word, None).value,
        indented,
        quoted: word.contains(['\'', '"', '\\']),
    }
}

/// Where a body that starts at `from` ends (`end`, the closing line's start) and what follows it
/// (`next`).
#[must_use]
pub fn body_of(src: &[u16], from: usize, heredoc: &Heredoc) -> Body {
    let delimiter = units(&heredoc.delimiter);
    let mut at = from;
    while at < src.len() {
        let end = index_of(src, "\n", at).unwrap_or(src.len());
        let mut line = slice(src, at, end);
        if heredoc.indented {
            let tabs = line.iter().take_while(|&&unit| unit == TAB).count();
            line = slice(line, tabs, line.len());
        }
        if line == delimiter.as_slice() {
            return Body {
                end: at,
                next: (end + 1).min(src.len()),
            };
        }
        at = end + 1;
    }
    Body {
        end: src.len(),
        next: src.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::utf16::text_of;

    #[test]
    fn a_delimiter_compares_as_units() {
        let src = units("a\n\tEOF\nb");
        let heredoc = heredoc_of("'EOF'", true);
        assert_eq!(text_of(&src), "a\n\tEOF\nb");
        assert_eq!(body_of(&src, 0, &heredoc), Body { end: 2, next: 7 });
    }
}
