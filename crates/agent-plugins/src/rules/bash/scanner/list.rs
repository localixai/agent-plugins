//! The scanner's reading of a list of commands (`scanner.ts`, from `separatorAt` to
//! `readList`): separators, comments and the segments between them. A file of its own only to
//! keep the port within the crate's file size (RS5); the TypeScript has it in `scanner.ts`.

use crate::js::trim;
use crate::js::utf16::{index_of, slice, starts_with_at, text_of, unit_at, units};
use crate::rules::bash::keywords::strip_keywords;

use super::{
    Scan, Span, fail, here, is, is_blank, is_metacharacter, skip_heredoc_bodies, skip_unit,
};

/// The separator at `pos`, ignoring the `&` of a redirect (`2>&1`, `<&3`, `&>file`).
fn separator_at(scan: &Scan<'_>) -> Option<usize> {
    let (src, pos) = (&scan.src, scan.pos);
    let found = scan
        .dialect
        .separators
        .iter()
        .find(|separator| starts_with_at(src, pos, separator))?;
    let length = found.encode_utf16().count();
    if found != "&" {
        return Some(length);
    }
    let before = pos.checked_sub(1).and_then(|before| unit_at(src, before));
    let redirect = is(before, '>') || is(before, '<') || is(unit_at(src, pos + 1), '>');
    (!redirect).then_some(length)
}

/// A comment at `scan.pos`: a `#` that starts a word runs to the end of its line, and
/// PowerShell's `<# ... #>` to its closer. Its end, or `None` when no comment starts here.
fn comment_end(scan: &Scan<'_>) -> Option<usize> {
    let (src, pos) = (&scan.src, scan.pos);
    let before = pos.checked_sub(1).and_then(|before| unit_at(src, before));
    if pos > 0 && !is_blank(before) && !is_metacharacter(before) {
        return None;
    }
    if scan.dialect.block_comments && starts_with_at(src, pos, "<#") {
        return Some(index_of(src, "#>", pos + 2).map_or(src.len() + 1, |close| close + 2));
    }
    if !is(unit_at(src, pos), '#') {
        return None;
    }
    Some(index_of(src, "\n", pos).unwrap_or(src.len()))
}

/// Skips a comment at `scan.pos`, if one starts there; an unclosed block comment is unparseable.
fn skip_comment(scan: &mut Scan<'_>) -> bool {
    let Some(end) = comment_end(scan) else {
        return false;
    };
    if end > scan.src.len() {
        fail(scan);
    } else {
        scan.comments.push(Span {
            start: scan.pos,
            end,
        });
        scan.pos = end;
    }
    true
}

fn skip_blanks(scan: &mut Scan<'_>) {
    while is_blank(here(scan)) {
        scan.pos += 1;
    }
}

/// What the scanner knows about the segment being read.
struct Segment {
    start: usize,
    /// Where its command will be inserted, so a command comes before the ones nested in it.
    slot: usize,
    /// The separator before it was `&&` or `||`: an empty segment here is an unparseable command.
    after_chain: bool,
}

/// The text from `start` to `end` without the comments in it.
fn code_between(scan: &Scan<'_>, start: usize, end: usize) -> String {
    let mut code = Vec::new();
    let mut from = start;
    for comment in &scan.comments {
        if comment.start < from || comment.end > end {
            continue;
        }
        code.extend_from_slice(slice(&scan.src, from, comment.start));
        from = comment.end;
    }
    code.extend_from_slice(slice(&scan.src, from, end));
    text_of(&code)
}

fn end_segment(scan: &mut Scan<'_>, segment: &Segment, end: usize) {
    let code = code_between(scan, segment.start, end);
    let text = trim(&code);
    if text.is_empty() {
        if segment.after_chain {
            scan.ok = false;
        }
        return;
    }
    let command = if scan.dialect.has_keywords {
        strip_keywords(text, &mut scan.functions)
    } else {
        Some(text.to_owned())
    };
    if let Some(command) = command.filter(|command| !command.starts_with('(')) {
        scan.subs.insert(segment.slot.min(scan.subs.len()), command);
    }
}

/// Reads commands up to `closer` (or the end), pushing each segment as a subcommand.
pub fn read_list(scan: &mut Scan<'_>, closer: Option<char>) {
    let mut segment = Segment {
        start: scan.pos,
        slot: scan.subs.len(),
        after_chain: false,
    };
    while scan.pos < scan.src.len() {
        if closer.is_some_and(|closer| is(here(scan), closer)) {
            end_segment(scan, &segment, scan.pos);
            scan.pos += 1;
            return;
        }
        if skip_comment(scan) {
            continue;
        }
        let Some(length) = separator_at(scan) else {
            skip_unit(scan);
            continue;
        };
        end_segment(scan, &segment, scan.pos);
        let separator = slice(&scan.src, scan.pos, scan.pos + length).to_vec();
        scan.pos += length;
        let chains = scan
            .dialect
            .chains
            .iter()
            .any(|chain| units(chain) == separator);
        if chains {
            skip_blanks(scan);
        }
        if separator == [0x0a] {
            skip_heredoc_bodies(scan);
        }
        segment = Segment {
            start: scan.pos,
            slot: scan.subs.len(),
            after_chain: chains,
        };
    }
    if closer.is_some() {
        scan.ok = false;
    }
    end_segment(scan, &segment, scan.src.len());
}
