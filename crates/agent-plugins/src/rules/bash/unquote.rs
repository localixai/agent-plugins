//! Ported from `src/rules/bash/unquote.ts`; only the core uses it now, so it has no wrapper.
//!
//! Judgment (spec/judgments.md, oracle 2.1.319 and 2.1.321): permissions.md does not say how a
//! quoted command reads against a rule. Claude Code matches the command name with its quoting
//! removed, so `"rm" -rf x`, `r"m"`, `\rm` and `$'rm'` are all `rm` to a rule, and leaves every
//! argument as written: `echo "x y"` matches `Bash(echo "x y")` and not `Bash(echo x y)`.
//!
//! The word is read as its UTF-16 code units, as the TypeScript indexes it (`js::utf16`).

use serde::Serialize;

use crate::js::utf16::{from_code_point, is_blank_unit, is_unit, slice, text_of, unit_at, units};
use crate::rules::bash::dialect::{BASH_DIALECT, ShellDialect};

/// One shell word as the shell passes it to the command, and what in it the shell still expands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WordValue {
    /// The word with its quotes, escapes and line continuations removed.
    pub value: String,
    /// A `$` or backtick outside single quotes: a variable or substitution may change the value.
    pub expands: bool,
    /// A `*`, `?` or `[` outside quotes: the shell may expand the word to file names.
    pub globs: bool,
}

/// In double quotes a backslash escapes only these (POSIX sh, "Double-Quotes").
const ESCAPED_IN_DOUBLE: [char; 5] = ['$', '`', '"', '\\', '\n'];

/// Bash's `$'...'` escapes (bash(1), "QUOTING"); a `\x`, `\u` or octal escape is read by number.
const ANSI_ESCAPES: [(char, char); 9] = [
    ('n', '\n'),
    ('t', '\t'),
    ('r', '\r'),
    ('a', '\u{0007}'),
    ('b', '\u{0008}'),
    ('e', '\u{001b}'),
    ('E', '\u{001b}'),
    ('f', '\u{000c}'),
    ('v', '\u{000b}'),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    None,
    Single,
    Double,
    Ansi,
}

struct Reader<'a> {
    word: Vec<u16>,
    dialect: &'a ShellDialect,
    at: usize,
    quote: Quote,
    out: Vec<u16>,
    expands: bool,
    globs: bool,
}

/// Whether `unit` is the ASCII character `wanted`.
fn is(unit: Option<u16>, wanted: char) -> bool {
    unit.is_some_and(|unit| u32::from(unit) == u32::from(wanted))
}

/// A `$` before more text, or a Bash backtick: the shell substitutes something here.
fn note_expansion(reader: &mut Reader<'_>, char: u16) {
    let next = unit_at(&reader.word, reader.at + 1);
    let dollar = is(Some(char), '$') && next.is_some() && !is_blank_unit(next);
    let backtick = is(Some(char), '`') && reader.dialect.backtick_substitutes;
    reader.expands |= dollar || backtick;
}

/// The value of an ASCII digit of `radix`, `None` for any other unit.
fn digit(unit: u16, radix: u32) -> Option<u32> {
    char::from_u32(u32::from(unit)).and_then(|character| character.to_digit(radix))
}

/// `[0-9A-Fa-f]{1,max}` (or `[0-7]`) at the start of `rest`: the number and how many digits.
fn digits(rest: &[u16], radix: u32, max: usize) -> Option<(u32, usize)> {
    let mut code = 0;
    let mut length = 0;
    while length < max {
        let Some(value) = rest.get(length).and_then(|&unit| digit(unit, radix)) else {
            break;
        };
        code = code * radix + value;
        length += 1;
    }
    (length > 0).then_some((code, length))
}

/// A `\x`, `\u`, `\U` or octal escape (the text just past its backslash): its code and length.
///
/// `/^(?:x([0-9A-Fa-f]{1,2})|u([0-9A-Fa-f]{1,4})|U([0-9A-Fa-f]{1,8}))/`, then `/^[0-7]{1,3}/`.
fn numbered_escape(rest: &[u16]) -> Option<(u32, usize)> {
    let first = unit_at(rest, 0);
    let max = [('x', 2), ('u', 4), ('U', 8)]
        .into_iter()
        .find_map(|(letter, max)| is(first, letter).then_some(max));
    let hex = max.and_then(|max| digits(slice(rest, 1, rest.len()), 16, max));
    if let Some((code, length)) = hex {
        return Some((code, length + 1));
    }
    digits(rest, 8, 3)
}

/// A `$'...'` escape (the text just past its backslash): its character and its length.
fn ansi_escape(rest: &[u16]) -> (Vec<u16>, usize) {
    if let Some((code, length)) = numbered_escape(rest) {
        return (from_code_point(code.min(0x0010_ffff)), length);
    }
    let Some(char) = unit_at(rest, 0) else {
        return (Vec::new(), 1);
    };
    let mapped = ANSI_ESCAPES
        .iter()
        .find(|(name, _)| is(Some(char), *name))
        .map_or_else(|| vec![char], |&(_, text)| units(&text.to_string()));
    (mapped, 1)
}

/// An escape character: the next character literally (when `escapes` it), a newline not at all.
fn read_escaped(reader: &mut Reader<'_>, escapes: impl Fn(u16) -> bool) {
    let next = unit_at(&reader.word, reader.at + 1);
    let Some(next) = next.filter(|&next| escapes(next)) else {
        reader.out.extend(unit_at(&reader.word, reader.at));
        reader.at += 1;
        return;
    };
    if !is(Some(next), '\n') {
        reader.out.push(next);
    }
    reader.at += 2;
}

/// Plain text: a character that may expand or glob.
fn read_plain(reader: &mut Reader<'_>, char: u16, globs: bool) {
    note_expansion(reader, char);
    reader.globs |= globs && ['*', '?', '['].iter().any(|&glob| is(Some(char), glob));
    reader.out.push(char);
    reader.at += 1;
}

/// The quote a `$'` (ANSI-C) or `$"` (locale) opens in Bash, or `None`.
fn dollar_quotes(unit: Option<u16>) -> Option<Quote> {
    [('\'', Quote::Ansi), ('"', Quote::Double)]
        .into_iter()
        .find_map(|(mark, quote)| is(unit, mark).then_some(quote))
}

fn quotes(unit: Option<u16>) -> Option<Quote> {
    [('\'', Quote::Single), ('"', Quote::Double)]
        .into_iter()
        .find_map(|(mark, quote)| is(unit, mark).then_some(quote))
}

/// What one character does in each quoting state (`STEP`).
fn step(reader: &mut Reader<'_>, char: u16) {
    match reader.quote {
        Quote::None => step_none(reader, char),
        Quote::Single => step_single(reader, char),
        Quote::Double => step_double(reader, char),
        Quote::Ansi => step_ansi(reader, char),
    }
}

fn step_none(reader: &mut Reader<'_>, char: u16) {
    let dollar = is(Some(char), '$') && reader.dialect.has_keywords;
    let opened = if dollar {
        dollar_quotes(unit_at(&reader.word, reader.at + 1))
    } else {
        quotes(Some(char))
    };
    if let Some(opened) = opened {
        reader.quote = opened;
        reader.at += if dollar { 2 } else { 1 };
    } else if is_unit(Some(char), &reader.dialect.escape) {
        read_escaped(reader, |_| true);
    } else {
        read_plain(reader, char, true);
    }
}

fn step_single(reader: &mut Reader<'_>, char: u16) {
    let closes = is(Some(char), '\'');
    reader.quote = if closes { Quote::None } else { Quote::Single };
    if !closes {
        reader.out.push(char);
    }
    reader.at += 1;
}

fn step_double(reader: &mut Reader<'_>, char: u16) {
    let has_keywords = reader.dialect.has_keywords;
    if is(Some(char), '"') {
        reader.quote = Quote::None;
        reader.at += 1;
    } else if is_unit(Some(char), &reader.dialect.escape) {
        read_escaped(reader, |next| {
            !has_keywords
                || ESCAPED_IN_DOUBLE
                    .iter()
                    .any(|&escaped| is(Some(next), escaped))
        });
    } else {
        read_plain(reader, char, false);
    }
}

fn step_ansi(reader: &mut Reader<'_>, char: u16) {
    let closes = is(Some(char), '\'');
    if closes {
        reader.quote = Quote::None;
    }
    let escape = is(Some(char), '\\')
        .then(|| ansi_escape(slice(&reader.word, reader.at + 1, reader.word.len())));
    match &escape {
        Some((text, _)) => reader.out.extend_from_slice(text),
        None if closes => {}
        None => reader.out.push(char),
    }
    reader.at += 1 + escape.map_or(0, |(_, length)| length);
}

/// One shell word read the way the shell reads it (`WordValue`).
#[must_use]
pub fn read_word(word: &str, dialect: Option<&ShellDialect>) -> WordValue {
    let bash = BASH_DIALECT;
    let mut reader = Reader {
        word: units(word),
        dialect: dialect.unwrap_or(&bash),
        at: 0,
        quote: Quote::None,
        out: Vec::new(),
        expands: false,
        globs: false,
    };
    while let Some(char) = unit_at(&reader.word, reader.at) {
        step(&mut reader, char);
    }
    WordValue {
        value: text_of(&reader.out),
        expands: reader.expands,
        globs: reader.globs,
    }
}
