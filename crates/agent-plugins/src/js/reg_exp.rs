//! `new RegExp(source, 'u')` for a pattern the package takes from input (a schema's
//! `patternProperties`), run by the `regex` crate: the source is read by JavaScript's grammar
//! with the `u` flag (ECMA-262, "Patterns", `[+UnicodeMode]`) and written again in the crate's
//! syntax, every character spelled `\x{…}` so no character means more there than here.
//!
//! What it runs, as JavaScript does: alternation, groups (`(…)`, `(?:…)`, `(?<name>…)`), the
//! quantifiers `*` `+` `?` `{n}` `{n,}` `{n,m}` and their lazy forms, `^` and `$` (of the whole
//! text: no `m` flag), `\b` and `\B` (ASCII words), `.` (no line terminator), classes with
//! ranges and `^`, the class escapes `\d` `\D` `\w` `\W` (ASCII) and `\s` `\S`
//! (`js::is_whitespace`'s set), and the character escapes `\f` `\n` `\r` `\t` `\v` `\0`, `\cX`,
//! `\xHH`, `\uHHHH` (a surrogate pair of two is one character) and `\u{…}`, and a syntax
//! character, `/` or any character past ASCII escaped (the last as JavaScriptCore reads it). A
//! lone surrogate is never in a Rust text, so it matches nothing. `tests/differential/js.rs`
//! holds it to what Bun answered (`scripts/rust/js-oracle/reg-exp.ts`).
//!
//! What it refuses, `Unrun::Unsupported`, though JavaScript runs it: lookahead and lookbehind,
//! backreferences (`\1`, `\k<name>`), `\p{…}` and `\P{…}`, the modifiers `(?i:…)`, a group name
//! that is not ASCII, a name used twice, groups nested past 100, a count past `u32`, and a pattern
//! past the crate's size limit. Everything else JavaScript refuses too: `Unrun::Invalid`.

use regex::Regex;

use super::SPACE_CLASS;

/// Why a pattern does not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unrun {
    /// JavaScript throws a `SyntaxError` for it.
    Invalid,
    /// JavaScript runs it, and the `regex` crate cannot (the module comment lists what).
    Unsupported,
}

const MAX_DEPTH: usize = 100;
const DOT: &str = r"[^\n\r\x{2028}\x{2029}]";
/// The bodies of the class escapes, inside `[…]`.
const DIGIT: &str = "0-9";
const WORD: &str = "A-Za-z0-9_";
const NOTHING: &str = r"[^\x{0}-\x{10FFFF}]";
const ANYTHING: &str = r"[\x{0}-\x{10FFFF}]";
const SYNTAX: &str = "^$\\.*+?()[]{}|/";

/// `new RegExp(source, 'u')`, ready for `test` (`Regex::is_match`).
///
/// # Errors
///
/// `Invalid` where JavaScript throws, `Unsupported` where the crate cannot run what it would.
pub fn unicode_reg_exp(source: &str) -> Result<Regex, Unrun> {
    let mut reader = Reader {
        chars: source.chars().collect(),
        at: 0,
        depth: 0,
        names: Vec::new(),
    };
    let written = reader.disjunction()?;
    if reader.at < reader.chars.len() {
        return Err(Unrun::Invalid);
    }
    Regex::new(&written).map_err(|_too_big| Unrun::Unsupported)
}

/// A class item: a range of code points (lone surrogates already left out), or a class escape.
enum Item {
    Range(u32, u32),
    Class(&'static str, bool),
}

struct Reader {
    chars: Vec<char>,
    at: usize,
    depth: usize,
    names: Vec<String>,
}

fn literal(point: u32) -> String {
    format!("\\x{{{point:X}}}")
}

impl Reader {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn eat(&mut self, wanted: char) -> bool {
        let found = self.peek() == Some(wanted);
        if found {
            self.at += 1;
        }
        found
    }

    fn next(&mut self) -> Result<char, Unrun> {
        let found = self.peek().ok_or(Unrun::Invalid)?;
        self.at += 1;
        Ok(found)
    }

    fn disjunction(&mut self) -> Result<String, Unrun> {
        let mut written = self.alternative()?;
        while self.eat('|') {
            written.push('|');
            written.push_str(&self.alternative()?);
        }
        Ok(written)
    }

    fn alternative(&mut self) -> Result<String, Unrun> {
        let mut written = String::new();
        while let Some(next) = self.peek() {
            if next == '|' || next == ')' {
                break;
            }
            written.push_str(&self.term()?);
        }
        Ok(written)
    }

    fn term(&mut self) -> Result<String, Unrun> {
        if let Some(assertion) = self.assertion() {
            return Ok(assertion.to_owned());
        }
        let atom = self.atom()?;
        Ok(format!("{atom}{}", self.quantifier()?))
    }

    fn assertion(&mut self) -> Option<&'static str> {
        let found = match (self.peek(), self.peek_at(1)) {
            (Some('^'), _) => ("^", 1),
            (Some('$'), _) => ("$", 1),
            (Some('\\'), Some('b')) => (r"(?-u:\b)", 2),
            (Some('\\'), Some('B')) => (r"(?-u:\B)", 2),
            _ => return None,
        };
        self.at += found.1;
        Some(found.0)
    }

    fn atom(&mut self) -> Result<String, Unrun> {
        match self.next()? {
            '.' => Ok(DOT.to_owned()),
            '(' => self.group(),
            '[' => self.class(),
            '\\' => self.atom_escape(),
            '*' | '+' | '?' | '{' | '}' | ']' | ')' | '|' => Err(Unrun::Invalid),
            other => Ok(literal(u32::from(other))),
        }
    }

    fn group(&mut self) -> Result<String, Unrun> {
        if self.eat('?') {
            match self.next()? {
                ':' => {}
                '<' if !matches!(self.peek(), Some('=' | '!')) => self.group_name()?,
                '=' | '!' | '<' => return Err(Unrun::Unsupported),
                _ => return Err(Unrun::Unsupported),
            }
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(Unrun::Unsupported);
        }
        let inner = self.disjunction()?;
        self.depth -= 1;
        if !self.eat(')') {
            return Err(Unrun::Invalid);
        }
        Ok(format!("(?:{inner})"))
    }

    fn group_name(&mut self) -> Result<(), Unrun> {
        let mut name = String::new();
        loop {
            match self.next()? {
                '>' if !name.is_empty() => break,
                c if c.is_ascii_alphabetic() || c == '_' || c == '$' => name.push(c),
                c if c.is_ascii_digit() && !name.is_empty() => name.push(c),
                c if c.is_ascii() && c != '\\' => return Err(Unrun::Invalid),
                _ => return Err(Unrun::Unsupported),
            }
        }
        if self.names.contains(&name) {
            return Err(Unrun::Unsupported);
        }
        self.names.push(name);
        Ok(())
    }

    fn quantifier(&mut self) -> Result<String, Unrun> {
        let mut written = match self.peek() {
            Some(c @ ('*' | '+' | '?')) => {
                self.at += 1;
                c.to_string()
            }
            Some('{') => {
                self.at += 1;
                self.counts()?
            }
            _ => return Ok(String::new()),
        };
        if self.eat('?') {
            written.push('?');
        }
        Ok(written)
    }

    /// `{n}`, `{n,}` or `{n,m}` after its `{`; anything else is no quantifier, which the `u`
    /// flag refuses.
    fn counts(&mut self) -> Result<String, Unrun> {
        let low = self.digits()?.ok_or(Unrun::Invalid)?;
        if self.eat('}') {
            return Ok(format!("{{{low}}}"));
        }
        if !self.eat(',') {
            return Err(Unrun::Invalid);
        }
        let high = self.digits()?;
        if !self.eat('}') {
            return Err(Unrun::Invalid);
        }
        match high {
            None => Ok(format!("{{{low},}}")),
            Some(high) if high < low => Err(Unrun::Invalid),
            Some(high) => Ok(format!("{{{low},{high}}}")),
        }
    }

    fn digits(&mut self) -> Result<Option<u32>, Unrun> {
        let mut value: Option<u32> = None;
        while let Some(digit) = self.peek().and_then(|c| c.to_digit(10)) {
            self.at += 1;
            let shifted = value.unwrap_or(0).checked_mul(10);
            value = Some(
                shifted
                    .and_then(|v| v.checked_add(digit))
                    .ok_or(Unrun::Unsupported)?,
            );
        }
        Ok(value)
    }

    fn atom_escape(&mut self) -> Result<String, Unrun> {
        match self.peek() {
            Some('1'..='9' | 'k' | 'p' | 'P') => Err(Unrun::Unsupported),
            _ => match self.class_escape(false)? {
                Item::Class(class, inverted) => Ok(nested(class, inverted)),
                Item::Range(low, _) if low > 0x10_FFFF => Ok(NOTHING.to_owned()),
                Item::Range(low, _) => Ok(literal(low)),
            },
        }
    }

    /// An escape after its `\`, in a class or not: a character as a one-point range (a lone
    /// surrogate as a point past U+10FFFF, which matches nothing), or a class escape.
    fn class_escape(&mut self, in_class: bool) -> Result<Item, Unrun> {
        let point = match self.next()? {
            'd' => return Ok(Item::Class(DIGIT, false)),
            'D' => return Ok(Item::Class(DIGIT, true)),
            'w' => return Ok(Item::Class(WORD, false)),
            'W' => return Ok(Item::Class(WORD, true)),
            's' => return Ok(Item::Class(SPACE_CLASS, false)),
            'S' => return Ok(Item::Class(SPACE_CLASS, true)),
            'p' | 'P' => return Err(Unrun::Unsupported),
            'f' => 0x0C,
            'n' => 0x0A,
            'r' => 0x0D,
            't' => 0x09,
            'v' => 0x0B,
            'b' if in_class => 0x08,
            '-' if in_class => u32::from('-'),
            '0' if !self.peek().is_some_and(|c| c.is_ascii_digit()) => 0,
            'c' => match self.next()? {
                letter if letter.is_ascii_alphabetic() => u32::from(letter) % 32,
                _ => return Err(Unrun::Invalid),
            },
            'x' => self.hex(2)?,
            'u' => self.unicode_escape()?,
            c if SYNTAX.contains(c) => u32::from(c),
            // JavaScriptCore takes any character past ASCII as itself, where the standard
            // refuses it with the `u` flag (Bun: `/\é/u.test('é')`).
            c if !c.is_ascii() => u32::from(c),
            _ => return Err(Unrun::Invalid),
        };
        Ok(Item::Range(point, point))
    }

    fn hex(&mut self, count: usize) -> Result<u32, Unrun> {
        let mut value = 0;
        for _ in 0..count {
            let digit = self.next()?.to_digit(16).ok_or(Unrun::Invalid)?;
            value = value * 16 + digit;
        }
        Ok(value)
    }

    /// `\uHHHH` (with a second `\uHHHH` that completes a surrogate pair) or `\u{…}`; a lone
    /// surrogate answers a point past U+10FFFF.
    fn unicode_escape(&mut self) -> Result<u32, Unrun> {
        if self.eat('{') {
            let mut value: u32 = 0;
            let mut any = false;
            while let Some(digit) = self.peek().and_then(|c| c.to_digit(16)) {
                self.at += 1;
                any = true;
                value = value.saturating_mul(16).saturating_add(digit);
            }
            if !any || !self.eat('}') || value > 0x10_FFFF {
                return Err(Unrun::Invalid);
            }
            return Ok(lone_surrogate_past(value));
        }
        let lead = self.hex(4)?;
        if (0xD800..0xDC00).contains(&lead)
            && self.peek() == Some('\\')
            && self.peek_at(1) == Some('u')
        {
            let back = self.at;
            self.at += 2;
            match self.hex(4) {
                Ok(trail) if (0xDC00..0xE000).contains(&trail) => {
                    return Ok(0x1_0000 + ((lead - 0xD800) << 10) + (trail - 0xDC00));
                }
                _ => self.at = back,
            }
        }
        Ok(lone_surrogate_past(lead))
    }

    fn class(&mut self) -> Result<String, Unrun> {
        let negated = self.eat('^');
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None => return Err(Unrun::Invalid),
                Some(']') => {
                    self.at += 1;
                    break;
                }
                Some(_) => items.push(self.class_range()?),
            }
        }
        Ok(class_text(negated, &items))
    }

    fn class_range(&mut self) -> Result<Item, Unrun> {
        let first = self.class_atom()?;
        if self.peek() != Some('-') || matches!(self.peek_at(1), Some(']') | None) {
            return Ok(first);
        }
        self.at += 1;
        let last = self.class_atom()?;
        match (first, last) {
            (Item::Range(low, _), Item::Range(high, _)) if real(low) <= real(high) => {
                Ok(Item::Range(low, high))
            }
            _ => Err(Unrun::Invalid),
        }
    }

    fn class_atom(&mut self) -> Result<Item, Unrun> {
        match self.next()? {
            '\\' => self.class_escape(true),
            c => Ok(Item::Range(u32::from(c), u32::from(c))),
        }
    }
}

/// A surrogate code point as the point past U+10FFFF that stands for it here.
fn lone_surrogate_past(point: u32) -> u32 {
    if (0xD800..0xE000).contains(&point) {
        0x11_0000 + point
    } else {
        point
    }
}

/// The code point a point stands for, a lone surrogate's own value.
fn real(point: u32) -> u32 {
    if point > 0x10_FFFF {
        point - 0x11_0000
    } else {
        point
    }
}

/// A class escape as a class of its own: `\D` is `[^0-9]`.
fn nested(class: &str, inverted: bool) -> String {
    format!("[{}{class}]", if inverted { "^" } else { "" })
}

/// A class as the crate writes it: each range without its surrogates, each class escape nested.
fn class_text(negated: bool, items: &[Item]) -> String {
    let mut parts = String::new();
    for item in items {
        match item {
            Item::Class(class, inverted) => parts.push_str(&nested(class, *inverted)),
            Item::Range(low, high) => {
                let (low, high) = (real(*low), real(*high));
                for (from, to) in [(low, high.min(0xD7FF)), (low.max(0xE000), high)] {
                    if from <= to {
                        parts.push_str(&literal(from));
                        parts.push('-');
                        parts.push_str(&literal(to));
                    }
                }
            }
        }
    }
    match (parts.is_empty(), negated) {
        (true, false) => NOTHING.to_owned(),
        (true, true) => ANYTHING.to_owned(),
        (false, negated) => format!("[{}{parts}]", if negated { "^" } else { "" }),
    }
}

#[cfg(test)]
mod tests {
    use super::{Unrun, unicode_reg_exp};

    fn test(source: &str, text: &str) -> Result<bool, Unrun> {
        unicode_reg_exp(source).map(|regex| regex.is_match(text))
    }

    // `new RegExp(source, 'u').test(text)` in Bun.
    #[test]
    fn it_matches_as_javascript_does() {
        assert_eq!(test("^x$", "x"), Ok(true));
        assert_eq!(test("^x$", "xy"), Ok(false));
        assert_eq!(test("[^]", "\n"), Ok(true));
        assert_eq!(test("[]", "a"), Ok(false));
        assert_eq!(test(r"^\w+$", "é"), Ok(false));
        assert_eq!(test(r"\s", "\u{FEFF}"), Ok(true));
        assert_eq!(test(r"\uD83D\uDE00", "😀"), Ok(true));
        assert_eq!(test(r"^[a-c-]{2}$", "b-"), Ok(true));
        assert_eq!(test(r"[\b]", "\u{8}"), Ok(true));
        assert_eq!(test("a.b", "a\nb"), Ok(false));
    }

    #[test]
    fn it_refuses_what_javascript_refuses() {
        for source in [
            "(", "a{", "}", "]", r"\a", r"\-", "*", "a{2,1}", r"[\d-z]", "[z-a]",
        ] {
            assert_eq!(test(source, ""), Err(Unrun::Invalid), "{source}");
        }
    }

    #[test]
    fn it_says_what_it_cannot_run() {
        for source in ["(?=a)", r"(a)\1", r"\p{L}", "(?i:a)", "(?<a>x)(?<a>y)"] {
            assert_eq!(test(source, ""), Err(Unrun::Unsupported), "{source}");
        }
    }
}
