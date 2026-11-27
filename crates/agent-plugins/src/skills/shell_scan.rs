//! Port of `src/skills/shell-scan.ts`, switched.
//!
//! Where a skill asks for dynamic context (skills.md, "Inject dynamic context"): the inline form
//! `` !`<command>` `` and a block opened with ```` ```! ````. The scan runs on the original text, so
//! a command's output, an argument or a variable can never add a placeholder: "Substitution runs
//! once over the original file ... a command cannot emit a placeholder for a later pass to expand".
//!
//! Two alternatives, so a block is read as a unit and its lines are never scanned again as inline
//! spans. The inline form is "only recognized when `!` appears at the start of a line or
//! immediately after whitespace": `(?<!\S)`. Judgments (spec/judgments.md): a block is opened by a
//! line that is exactly the fence (no indentation) and closed by the first line that is exactly
//! ```` ``` ````; an inline span stays on one line. As Claude Code 2.1.322 does (oracle): any
//! JavaScript whitespace before the `!` counts (a no-break space, a BOM, U+2028), and Markdown code
//! spans and other fences are not skipped.
//!
//! The TypeScript reads the text with one regular expression of two alternatives and the flags `gm`;
//! the Rust has no lookbehind and no lazy group over a line anchor, so each alternative is walked by
//! hand, in the same order, the first that matches at a position winning. Offsets are UTF-16 code
//! units, as `RegExpExecArray.index` is.

use serde::Serialize;

use crate::js::indexing::length_number;
use crate::js::{is_whitespace, trim};

/// Where the placeholder is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InjectionKind {
    /// `` !`command` ``.
    Inline,
    /// A block opened with ```` ```! ````.
    Fence,
}

/// One placeholder.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Injection {
    /// Which form.
    pub kind: InjectionKind,
    /// Offsets in the scanned text; `[start, end)` is the whole placeholder.
    pub start: f64,
    /// The end offset.
    pub end: f64,
    /// What the shell runs, as written: a block's lines without the fences.
    pub command: String,
    /// The text before the command, kept to put a placeholder back as it was written.
    pub open: String,
    /// The text after the command.
    pub close: String,
}

/// The text as characters, with each character's offset in UTF-16 code units.
struct Scanned {
    characters: Vec<char>,
    offsets: Vec<usize>,
}

impl Scanned {
    fn new(content: &str) -> Self {
        let characters: Vec<char> = content.chars().collect();
        let mut offsets = Vec::with_capacity(characters.len() + 1);
        let mut at = 0;
        for character in &characters {
            offsets.push(at);
            at += character.len_utf16();
        }
        offsets.push(at);
        Self {
            characters,
            offsets,
        }
    }

    fn at(&self, index: usize) -> Option<char> {
        self.characters.get(index).copied()
    }

    fn text(&self, from: usize, to: usize) -> String {
        self.characters
            .get(from..to)
            .map(|part| part.iter().collect())
            .unwrap_or_default()
    }

    fn offset(&self, index: usize) -> f64 {
        length_number(self.offsets.get(index).copied().unwrap_or_default())
    }

    /// Whether `literal` is at `index`.
    fn starts_with(&self, index: usize, literal: &str) -> bool {
        literal
            .chars()
            .enumerate()
            .all(|(step, character)| self.at(index + step) == Some(character))
    }

    /// The index after the blanks (space and tab) from `index`.
    fn skip_blanks(&self, mut index: usize) -> usize {
        while matches!(self.at(index), Some(' ' | '\t')) {
            index += 1;
        }
        index
    }

    /// `\r?\n` at `index`: the index after it.
    fn newline(&self, index: usize) -> Option<usize> {
        match (self.at(index), self.at(index + 1)) {
            (Some('\r'), Some('\n')) => Some(index + 2),
            (Some('\n'), _) => Some(index + 1),
            _ => None,
        }
    }

    /// `^` of the flag `m`: the start of the text or just after a line terminator.
    fn at_line_start(&self, index: usize) -> bool {
        index == 0
            || self
                .at(index - 1)
                .is_some_and(|before| matches!(before, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    }

    /// `$` of the flag `m`: the end of the text or just before a line terminator.
    fn at_line_end(&self, index: usize) -> bool {
        self.at(index)
            .is_none_or(|next| matches!(next, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    }
}

/// The closing line of a block at `index`: three backticks, blanks, and the end of the line. The
/// index after it.
fn closing_fence(scanned: &Scanned, index: usize) -> Option<usize> {
    if !scanned.starts_with(index, "```") {
        return None;
    }
    let end = scanned.skip_blanks(index + 3);
    scanned.at_line_end(end).then_some(end)
}

/// The block that opens at `start` (the alternative `fenceOpen`): the group of the body and its
/// closing line is tried first, with the shortest body, then the block with no body.
fn fence_at(scanned: &Scanned, start: usize) -> Option<(Injection, usize)> {
    if !scanned.at_line_start(start) || !scanned.starts_with(start, "```!") {
        return None;
    }
    let after_blanks = scanned.skip_blanks(start + 4);
    let body_start = scanned.newline(after_blanks)?;
    let open = scanned.text(start, body_start);
    for body_end in body_start..=scanned.characters.len() {
        let Some(after_newline) = scanned.newline(body_end) else {
            continue;
        };
        if let Some(end) = closing_fence(scanned, after_newline) {
            let injection = Injection {
                kind: InjectionKind::Fence,
                start: scanned.offset(start),
                end: scanned.offset(end),
                command: scanned.text(body_start, body_end),
                open,
                close: scanned.text(body_end, end),
            };
            return Some((injection, end));
        }
    }
    let end = closing_fence(scanned, body_start)?;
    let injection = Injection {
        kind: InjectionKind::Fence,
        start: scanned.offset(start),
        end: scanned.offset(end),
        command: String::new(),
        open,
        close: scanned.text(body_start, end),
    };
    Some((injection, end))
}

/// The inline span at `start` (the alternative `inlineOpen`): `(?<!\S)` before it, then `` !` ``, a
/// run of characters that are no backtick and no newline, and the closing backtick.
fn inline_at(scanned: &Scanned, start: usize) -> Option<(Injection, usize)> {
    let spaced = start == 0 || scanned.at(start - 1).is_some_and(is_whitespace);
    if !spaced || !scanned.starts_with(start, "!`") {
        return None;
    }
    let body_start = start + 2;
    let mut body_end = body_start;
    while scanned
        .at(body_end)
        .is_some_and(|character| character != '`' && character != '\n')
    {
        body_end += 1;
    }
    if body_end == body_start || scanned.at(body_end) != Some('`') {
        return None;
    }
    let injection = Injection {
        kind: InjectionKind::Inline,
        start: scanned.offset(start),
        end: scanned.offset(body_end + 1),
        command: scanned.text(body_start, body_end),
        open: scanned.text(start, body_start),
        close: "`".to_owned(),
    };
    Some((injection, body_end + 1))
}

/// The placeholders of `content`, in order, without overlap.
#[must_use]
pub fn find_injections(content: &str) -> Vec<Injection> {
    let scanned = Scanned::new(content);
    let mut injections = Vec::new();
    let mut at = 0;
    while at < scanned.characters.len() {
        let found = fence_at(&scanned, at);
        let found = found.or_else(|| inline_at(&scanned, at));
        let Some((injection, next)) = found else {
            at += 1;
            continue;
        };
        // A blank command runs nothing: it is left as written.
        if !trim(&injection.command).is_empty() {
            injections.push(injection);
        }
        at = next.max(at + 1);
    }
    injections
}

crate::registry::exports! {
    "skills/shell-scan" {
        "findInjections" => ok |content: String| find_injections(&content);
    }
}
