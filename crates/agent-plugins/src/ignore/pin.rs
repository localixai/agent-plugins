//! A rule's source made final for `ignores` (`_make` in `regex` mode in the package): the pending
//! trailing wildcard and trailing `/**` expanded, then each wildcard of a run split by fixed pieces
//! pinned (`pinWildcards`).
//!
//! Where a run of non-slash wildcards is split by fixed pieces (`[^/]*ab[^/]*cd`), the engine has
//! many equivalent ways to lay the input across the wildcards and, on input that does not match,
//! works through all of them. Every wildcard but the last is pinned to stop where the whole piece
//! after it begins, which leaves one way and changes no match. The source is read one token at a
//! time, in UTF-16 code units.

use std::sync::LazyLock;

use crate::js::regexp::RegExp;
use crate::js::utf16::{slice, starts_with_at, text_of, unit_at, units};

use super::brackets::{TRAILING_DOUBLESTAR, TRAILING_WILDCARD};
use super::{Pattern, group, replace_with};

/// `REGEX_REPLACE_TRAILING_WILDCARD`.
static REGEX_REPLACE_TRAILING_WILDCARD: Pattern =
    LazyLock::new(|| RegExp::new(&format!("(^|\\\\\\/)?{TRAILING_WILDCARD}$"), "").ok());

/// `REGEX_REPLACE_TRAILING_DOUBLESTAR`.
static REGEX_REPLACE_TRAILING_DOUBLESTAR: Pattern =
    LazyLock::new(|| RegExp::new(&format!("{TRAILING_DOUBLESTAR}$"), "").ok());

/// `replaceTrailingWildcard`: `/*` matches neither nothing nor everything, `abc/*` not `abc/`
/// itself, and `a*` matches `a`.
fn replace_trailing_wildcard(p1: &str) -> String {
    let prefix = if p1.is_empty() {
        "[^/]*".to_owned()
    } else {
        format!("{p1}[^/]+")
    };
    format!("{prefix}(?=$|\\/$)")
}

/// `abc/**` matches everything under `abc`, but not `abc/` itself (#21).
const REPLACE_TRAILING_DOUBLESTAR: &str = ".+(?=$|\\/$)";

const WILDCARD: &str = "[^\\/]*";

/// A token of a source: a wildcard, a piece of one character (a literal, an escape, a class), or
/// a group or anchor that ends a run.
enum Token {
    Wildcard,
    Single(Vec<u16>),
    Boundary(Vec<u16>),
}

/// The fixed piece between the wildcard at `at` and the next wildcard of the run.
fn separator_after(run: &[Token], at: usize) -> Vec<u16> {
    let mut separator = Vec::new();
    for token in run.iter().skip(at + 1) {
        match token {
            Token::Wildcard => break,
            Token::Single(single) | Token::Boundary(single) => separator.extend(single),
        }
    }
    separator
}

/// The end of the class that starts at `index`.
fn class_end(source: &[u16], index: usize) -> usize {
    let is = |at: usize, text: &str| unit_at(source, at) == text.encode_utf16().next();
    let mut end = index + 1;
    if is(end, "^") {
        end += 1;
    }
    if is(end, "]") {
        end += 1;
    }
    while end < source.len() && !is(end, "]") {
        end += if is(end, "\\") { 2 } else { 1 };
    }
    end + 1
}

/// The end of the group that starts at `index`, with a quantifier after it.
fn group_end(source: &[u16], index: usize) -> usize {
    let is = |at: usize, text: &str| unit_at(source, at) == text.encode_utf16().next();
    let mut depth: i64 = 0;
    let mut end = index;
    loop {
        if is(end, "\\") {
            end += 1;
        } else if is(end, "(") {
            depth += 1;
        } else if is(end, ")") {
            depth -= 1;
        }
        end += 1;
        if !(end < source.len() && depth > 0) {
            break;
        }
    }
    if is(end, "*") || is(end, "+") || is(end, "?") {
        end += 1;
    }
    end
}

fn tokens_of(source: &[u16]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while let Some(character) = unit_at(source, index) {
        let end = if starts_with_at(source, index, WILDCARD) {
            tokens.push(Token::Wildcard);
            index += units(WILDCARD).len();
            continue;
        } else if character == u16::from(b'[') {
            class_end(source, index)
        } else if character == u16::from(b'\\') {
            index + 2
        } else if character == u16::from(b'(') {
            let end = group_end(source, index);
            tokens.push(Token::Boundary(slice(source, index, end).to_vec()));
            index = end;
            continue;
        } else if character == u16::from(b'^') || character == u16::from(b'$') {
            tokens.push(Token::Boundary(vec![character]));
            index += 1;
            continue;
        } else {
            index + 1
        };
        tokens.push(Token::Single(slice(source, index, end).to_vec()));
        index = end;
    }
    tokens
}

/// The tokens of one run written out, every wildcard but the last pinned to the piece after it.
fn flush(run: &mut Vec<Token>, out: &mut Vec<u16>) {
    let last_wildcard = run
        .iter()
        .rposition(|token| matches!(token, Token::Wildcard));
    for (at, token) in run.iter().enumerate() {
        match token {
            Token::Single(single) | Token::Boundary(single) => out.extend(single),
            Token::Wildcard if Some(at) == last_wildcard => out.extend(units(WILDCARD)),
            Token::Wildcard => {
                out.extend(units("(?:(?!"));
                out.extend(separator_after(run, at));
                out.extend(units(")[^\\/])*"));
            }
        }
    }
    run.clear();
}

/// `pinWildcards`.
fn pin_wildcards(text: &str) -> String {
    if !text.contains(WILDCARD) {
        return text.to_owned();
    }
    let source = units(text);
    let mut out = Vec::new();
    let mut run = Vec::new();
    for token in tokens_of(&source) {
        match token {
            Token::Boundary(boundary) => {
                flush(&mut run, &mut out);
                out.extend(boundary);
            }
            other @ (Token::Wildcard | Token::Single(_)) => run.push(other),
        }
    }
    flush(&mut run, &mut out);
    text_of(&out)
}

/// The source `ignores` tests a path with, from a rule's prefix.
pub(super) fn ignore_source(prefix: &str) -> String {
    let source = replace_with(&REGEX_REPLACE_TRAILING_WILDCARD, prefix, |found| {
        replace_trailing_wildcard(group(found, 1))
    });
    let source = replace_with(&REGEX_REPLACE_TRAILING_DOUBLESTAR, &source, |_| {
        REPLACE_TRAILING_DOUBLESTAR.to_owned()
    });
    pin_wildcards(&source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_package_s_patterns_compile() {
        assert!(REGEX_REPLACE_TRAILING_WILDCARD.is_some());
        assert!(REGEX_REPLACE_TRAILING_DOUBLESTAR.is_some());
    }

    #[test]
    fn the_markers_are_expanded_and_the_wildcards_pinned() {
        assert_eq!(
            ignore_source("(?:^|\\/)a\u{E000}"),
            "(?:^|\\/)a[^/]*(?=$|\\/$)"
        );
        assert_eq!(ignore_source("^a\\/\u{E000}"), "^a\\/[^/]+(?=$|\\/$)");
        assert_eq!(ignore_source("^a\\/\u{E001}"), "^a\\/.+(?=$|\\/$)");
        assert_eq!(
            ignore_source("^[^\\/]*o\\/[^\\/]*\\/x"),
            "^(?:(?!o\\/)[^\\/])*o\\/[^\\/]*\\/x"
        );
    }
}
