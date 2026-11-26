//! Ported from `src/rules/bash/scanner.ts`; only the core uses it now, so it has no wrapper.
//!
//! The one shell lexer of the kit: it reads a command line unit by unit (a quoted part, a
//! substitution, an escaped character, a character) and splits it into the commands it runs.
//! Words and redirects are read from those commands by `words.rs`, the public reading is
//! `split.rs`.
//!
//! The text is read as its UTF-16 code units and every position counts them, as the TypeScript's
//! do (`js::utf16`).

use serde::Serialize;

use crate::js::utf16::{
    index_of, is_blank_unit, is_unit, replace_all, slice, starts_with_at, text_of, unit_at, units,
};
use crate::rules::bash::dialect::ShellDialect;
use crate::rules::bash::heredoc::{Heredoc, body_of, heredoc_of, heredoc_opener};

/// Where something sits in the text that was scanned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Span {
    /// Its first code unit.
    pub start: usize,
    /// Past its last code unit.
    pub end: usize,
}

/// Nesting deeper than this is not read: no real command nests substitutions this far.
const MAX_DEPTH: usize = 32;

/// A scan of one command line, as it goes.
#[derive(Debug)]
pub struct Scan<'a> {
    /// The text's code units.
    pub src: Vec<u16>,
    /// How its shell writes what the scanner tells apart.
    pub dialect: &'a ShellDialect,
    /// The commands found, a command before the ones nested in it.
    pub subs: Vec<String>,
    /// Here-documents opened on the current line, whose bodies follow it.
    pub heredocs: Vec<Heredoc>,
    /// Comments, cut from the commands they end.
    pub comments: Vec<Span>,
    /// Here-document bodies.
    pub bodies: Vec<Span>,
    /// The names of the functions the line defines.
    pub functions: Vec<String>,
    /// Where the scan is.
    pub pos: usize,
    /// How deep in substitutions it is.
    pub depth: usize,
    /// Nothing unreadable so far.
    pub ok: bool,
    /// Set when a substitution or expansion is read outside single quotes.
    pub substitutes: bool,
    /// Set when a subshell, a command or process substitution, or an arithmetic expansion is read.
    pub nests: bool,
}

/// A scan of `src` at `depth`, at its start.
#[must_use]
pub fn new_scan<'a>(src: &str, dialect: &'a ShellDialect, depth: usize) -> Scan<'a> {
    Scan {
        src: units(src),
        dialect,
        subs: Vec::new(),
        heredocs: Vec::new(),
        comments: Vec::new(),
        bodies: Vec::new(),
        functions: Vec::new(),
        pos: 0,
        depth,
        ok: true,
        substitutes: false,
        nests: false,
    }
}

/// `/\s/.test(char)` of a character read at a position, none past the end.
#[must_use]
pub fn is_blank(char: Option<u16>) -> bool {
    is_blank_unit(char)
}

/// Whether `unit` is the ASCII character `wanted`.
fn is(unit: Option<u16>, wanted: char) -> bool {
    unit.is_some_and(|unit| u32::from(unit) == u32::from(wanted))
}

/// `$(`, `${` or `$NAME`: a substitution or expansion starts here (`/\$[({A-Za-z_]/y`).
fn expansion_at(src: &[u16], at: usize) -> bool {
    let next = unit_at(src, at + 1).and_then(|unit| char::from_u32(u32::from(unit)));
    is(unit_at(src, at), '$')
        && next.is_some_and(|next| matches!(next, '(' | '{' | 'A'..='Z' | 'a'..='z' | '_'))
}

/// Records an expansion or substitution that starts at `at`; never called inside single quotes.
fn note_expansion(scan: &mut Scan<'_>, at: usize) {
    let backtick = scan.dialect.backtick_substitutes && is(unit_at(&scan.src, at), '`');
    if backtick || expansion_at(&scan.src, at) {
        scan.substitutes = true;
    }
}

fn fail(scan: &mut Scan<'_>) {
    scan.ok = false;
    scan.pos = scan.src.len();
}

/// The character at the scan's position.
fn here(scan: &Scan<'_>) -> Option<u16> {
    unit_at(&scan.src, scan.pos)
}

/// A single-quoted part is literal: `scan.pos` moves past its closing quote, or the scan fails.
fn skip_single(scan: &mut Scan<'_>) {
    match index_of(&scan.src, "'", scan.pos + 1) {
        None => fail(scan),
        Some(close) => scan.pos = close + 1,
    }
}

/// One unit of text that only substitutions run in (a double-quoted part, an unquoted body).
fn skip_text_unit(scan: &mut Scan<'_>) {
    let pos = scan.pos;
    note_expansion(scan, pos);
    if is_unit(here(scan), &scan.dialect.escape) {
        scan.pos += 2;
    } else if starts_with_at(&scan.src, pos, "$((") {
        skip_expansion(scan, 3, "))");
    } else if starts_with_at(&scan.src, pos, "$(") {
        skip_nested(scan, 2, ')');
    } else if starts_with_at(&scan.src, pos, "${") {
        skip_expansion(scan, 2, "}");
    } else if is(here(scan), '`') && scan.dialect.backtick_substitutes {
        skip_backtick(scan);
    } else {
        scan.pos += 1;
    }
}

/// A double-quoted part; a quote that never closes is unparseable.
fn skip_double(scan: &mut Scan<'_>) {
    scan.pos += 1;
    while scan.pos < scan.src.len() && !is(here(scan), '"') {
        skip_text_unit(scan);
    }
    if scan.pos >= scan.src.len() {
        return fail(scan);
    }
    scan.pos += 1;
}

/// A backtick substitution: what is between the backticks is a command line of its own.
fn skip_backtick(scan: &mut Scan<'_>) {
    let mut i = scan.pos + 1;
    while i < scan.src.len() && !is(unit_at(&scan.src, i), '`') {
        i += if is(unit_at(&scan.src, i), '\\') {
            2
        } else {
            1
        };
    }
    if i >= scan.src.len() {
        return fail(scan);
    }
    let inner_text = replace_all(slice(&scan.src, scan.pos + 1, i), "\\`", "`");
    let mut inner = new_scan("", scan.dialect, 0);
    inner.src = inner_text;
    inner.depth = scan.depth + 1;
    read_list(&mut inner, None);
    scan.subs.append(&mut inner.subs);
    scan.functions.append(&mut inner.functions);
    scan.ok &= inner.ok;
    scan.nests = true;
    scan.pos = i + 1;
}

/// `$(...)`, `(...)` and, where braces hold commands, `{...}`: a list of its own up to `closer`.
fn skip_nested(scan: &mut Scan<'_>, opener: usize, closer: char) {
    scan.pos += opener;
    scan.depth += 1;
    scan.nests |= closer == ')';
    if scan.depth > MAX_DEPTH {
        return fail(scan);
    }
    read_list(scan, Some(closer));
    scan.depth -= 1;
}

/// `${name:-$(cmd)}` and `$((1 + 2))` are expansions: text, except for what is substituted
/// inside them, which runs. Skipped up to `end`, reading each unit so a nested substitution is
/// found.
fn skip_expansion(scan: &mut Scan<'_>, opener: usize, end: &str) {
    scan.pos += opener;
    scan.depth += 1;
    scan.nests |= end == "))";
    if scan.depth > MAX_DEPTH {
        return fail(scan);
    }
    while scan.pos < scan.src.len() && !starts_with_at(&scan.src, scan.pos, end) {
        skip_unit(scan);
    }
    if scan.pos >= scan.src.len() {
        return fail(scan);
    }
    scan.pos += end.len();
    scan.depth -= 1;
}

/// What opens a unit that is skipped whole, and how: longest opener first.
type Opener = (&'static str, fn(&mut Scan<'_>));
const OPENERS: [Opener; 6] = [
    ("$((", |scan| skip_expansion(scan, 3, "))")),
    ("$(", |scan| skip_nested(scan, 2, ')')),
    ("${", |scan| skip_expansion(scan, 2, "}")),
    ("(", |scan| skip_nested(scan, 1, ')')),
    ("'", skip_single),
    ("\"", skip_double),
];

/// What ends a word besides a blank: an operator character (`/[;&|()<>]/`).
pub(crate) fn is_metacharacter(char: Option<u16>) -> bool {
    [';', '&', '|', '(', ')', '<', '>']
        .iter()
        .any(|&metacharacter| is(char, metacharacter))
}

/// A here-document opener at `scan.pos`: its delimiter word is read, its body skipped at the
/// line's end.
fn note_heredoc(scan: &mut Scan<'_>) -> bool {
    let Some(opener) = heredoc_opener(&scan.src, scan.pos) else {
        return false;
    };
    scan.pos += opener.length;
    let start = scan.pos;
    while scan.pos < scan.src.len() && !is_blank(here(scan)) {
        if is_metacharacter(here(scan)) {
            break;
        }
        skip_unit(scan);
    }
    if scan.pos == start {
        fail(scan);
    } else {
        let word = text_of(slice(&scan.src, start, scan.pos));
        scan.heredocs.push(heredoc_of(&word, opener.indented));
    }
    true
}

/// After a newline: skips the bodies of the here-documents opened on the line just ended.
fn skip_heredoc_bodies(scan: &mut Scan<'_>) {
    for heredoc in std::mem::take(&mut scan.heredocs) {
        let body = body_of(&scan.src, scan.pos, &heredoc);
        scan.bodies.push(Span {
            start: scan.pos,
            end: body.end,
        });
        if !heredoc.quoted {
            while scan.pos < body.end {
                skip_text_unit(scan);
            }
            if scan.pos > body.end {
                return fail(scan);
            }
        }
        scan.pos = body.next;
    }
}

/// Advances `scan.pos` over one unit: an escaped character, a quoted or nested part, or a
/// character.
pub fn skip_unit(scan: &mut Scan<'_>) {
    let pos = scan.pos;
    if is_unit(here(scan), &scan.dialect.escape) {
        scan.pos += 2;
        return;
    }
    note_expansion(scan, pos);
    if is(here(scan), '`') && scan.dialect.backtick_substitutes {
        return skip_backtick(scan);
    }
    if is(here(scan), '{') && scan.dialect.braces_hold_commands {
        return skip_nested(scan, 1, '}');
    }
    if is(here(scan), '<') && scan.dialect.has_keywords && note_heredoc(scan) {
        return;
    }
    match OPENERS
        .iter()
        .find(|(open, _)| starts_with_at(&scan.src, pos, open))
    {
        None => scan.pos += 1,
        Some((_, skip)) => skip(scan),
    }
}

mod list;

pub use list::read_list;
