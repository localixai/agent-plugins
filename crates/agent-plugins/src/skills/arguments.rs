//! Port of `src/skills/arguments.ts`, switched.
//!
//! The arguments a user typed after a skill's name (skills.md, "Available string substitutions":
//! "Indexed arguments use shell-style quoting, so wrap multi-word values in quotes").
//!
//! The docs give one example, `"hello world" second`. The rest is the kit's judgment
//! (spec/judgments.md): single quotes are literal; double quotes take a backslash before `"` and
//! `\`; outside quotes a backslash escapes whitespace, a quote or a backslash and stays as written
//! before anything else (so a Windows path `C:\Users\me` survives); quoted and bare parts of one
//! word join; `""` is an empty argument; a quote never closed takes the rest of the text.

use std::sync::LazyLock;

use regex::Regex;

use crate::js::{Pattern, SPACE_CLASS};

/// One piece of a word: a quoted run (closed, or open to the end), an escape or bare text.
fn piece() -> String {
    format!(
        r#""(?:\\(?s:.)|[^"\\])*"|'[^']*'|"(?:\\(?s:.)|[^"\\])*\\?$|'[^']*$|\\(?s:.)|[^{SPACE_CLASS}"'\\]+|\\"#
    )
}

static WORDS: Pattern = LazyLock::new(|| Regex::new(&format!("(?:{})+", piece())).ok());
static PIECES: Pattern = LazyLock::new(|| Regex::new(&piece()).ok());

/// True when the final `"` of a double-quoted run is itself escaped (an unclosed run).
fn ends_escaped(piece: &str) -> bool {
    let body = piece
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or("");
    let slashes = body.len() - body.trim_end_matches('\\').len();
    slashes % 2 == 1
}

/// `\"` and `\\` of a double-quoted run stand for the character they escape.
fn double_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match (character, characters.peek().copied()) {
            ('\\', Some(next @ ('"' | '\\'))) => {
                out.push(next);
                characters.next();
            }
            _ => out.push(character),
        }
    }
    out
}

/// What a quoted run or an escape stands for, picked by its first character.
fn read_piece(piece: &str) -> String {
    let mut characters = piece.chars();
    let first = characters.next();
    let longer = characters.next().is_some();
    match first {
        Some('"') => {
            let rest = piece.strip_prefix('"').unwrap_or_default();
            let closed = longer && piece.ends_with('"') && !ends_escaped(piece);
            double_escapes(if closed {
                rest.strip_suffix('"').unwrap_or(rest)
            } else {
                rest
            })
        }
        Some('\'') => {
            let rest = piece.strip_prefix('\'').unwrap_or_default();
            if longer && piece.ends_with('\'') {
                rest.strip_suffix('\'').unwrap_or(rest).to_owned()
            } else {
                rest.to_owned()
            }
        }
        Some('\\') => bare_escape(piece).unwrap_or_else(|| piece.to_owned()),
        _ => piece.to_owned(),
    }
}

/// `^\\(["'\\\s])$`: a backslash and one quote, backslash or white-space character.
fn bare_escape(piece: &str) -> Option<String> {
    let mut characters = piece.strip_prefix('\\')?.chars();
    let escaped = characters.next()?;
    let single = characters.next().is_none();
    (single && (matches!(escaped, '"' | '\'' | '\\') || crate::js::is_whitespace(escaped)))
        .then(|| escaped.to_string())
}

fn unquote(word: &str) -> String {
    match PIECES.as_ref() {
        Some(pieces) => pieces
            .replace_all(word, |found: &regex::Captures<'_>| read_piece(&found[0]))
            .into_owned(),
        None => word.to_owned(),
    }
}

/// `raw` split into arguments the way a shell would: `"hello world" second` is two.
#[must_use]
pub fn parse_arguments(raw: &str) -> Vec<String> {
    WORDS
        .as_ref()
        .map(|words| {
            words
                .find_iter(raw)
                .map(|word| unquote(word.as_str()))
                .collect()
        })
        .unwrap_or_default()
}

crate::registry::exports! {
    "skills/arguments" {
        "parseArguments" => ok |raw: String| parse_arguments(&raw);
    }
}
