//! The bracket expressions and escapes of a pattern, held aside before the chain of `replacers.rs`
//! runs (`scanBracket` and `extractBrackets` in the package). gitignore(5) defers to fnmatch(3),
//! which git implements in `wildmatch.c`: a bracket expression has a grammar of its own (a `]`
//! right after `[` or `[!` is a member, `[:alpha:]` names one of twelve POSIX classes, `\` escapes
//! the next member, `*`, `?` and `.` are plain members, and an expression never closed makes the
//! whole pattern match nothing), so it is scanned out and put back once the chain is done.
//!
//! The pattern is read in UTF-16 code units, as the package indexes it.

use crate::js::regexp::RegExp;
use crate::js::utf16::{text_of, unit_at, units};

/// The POSIX classes git knows, ASCII only as its own ctype macros classify (`wildmatch.c`), `/`
/// left out of each, as a bracket expression never matches a separator. git's `sane-ctype.h`
/// classifies `\v` and `\f` as control, not space.
const POSIX_CLASSES: [(&str, &str); 12] = [
    ("alnum", "0-9A-Za-z"),
    ("alpha", "A-Za-z"),
    ("blank", " \\t"),
    ("cntrl", "\\x00-\\x1f\\x7f"),
    ("digit", "0-9"),
    ("graph", "!-.0-~"),
    ("lower", "a-z"),
    ("print", " -.0-~"),
    ("punct", "!-.:-@\\[-`{-~"),
    ("space", " \\t\\n\\r"),
    ("upper", "A-Z"),
    ("xdigit", "0-9A-Fa-f"),
];

const CLASS_MEMBERS_TO_ESCAPE: &str = "\\]^-[";

/// The characters with a meaning of their own in a regular expression, `REGEX_LITERAL_SPECIAL`
/// (`/[.*+?()[\]{}^$|\\/]/`) as the set it tests one character against.
const REGEX_LITERAL_SPECIAL: &str = ".*+?()[]{}^$|\\/";

const ESCAPE: u16 = b'\\' as u16;
const SLASH: u16 = b'/' as u16;
const SPACE: u16 = b' ' as u16;
const STAR: u16 = b'*' as u16;
const OPEN: u16 = b'[' as u16;

/// A member of a class, escaped when the class would read it otherwise.
fn escape_member(member: u16, body: &mut Vec<u16>) {
    if CLASS_MEMBERS_TO_ESCAPE
        .encode_utf16()
        .any(|unit| unit == member)
    {
        body.push(ESCAPE);
    }
    body.push(member);
}

/// > if (matched == negated || ((flags & WM_PATHNAME) && text_ch == '/'))
/// >   return WM_ABORT_TO_STARSTAR;                     (git, `wildmatch.c`)
///
/// A bracket expression never matches a separator: a negated class gets `/` as one more excluded
/// character, a plain one that would let `/` in is guarded with a lookahead.
const NON_SLASH: &str = "(?!\\/)";

/// `classSource`.
fn class_source(negated: bool, body: &[u16]) -> String {
    let body = text_of(body);
    if negated {
        return format!("[^\\/{body}]");
    }
    let source = format!("[{body}]");
    // The members are escaped and the ranges in order, so the class always compiles.
    if RegExp::new(&source, "").is_ok_and(|class| class.test_shared("/")) {
        format!("{NON_SLASH}{source}")
    } else {
        source
    }
}

/// The class a `[:name:]` names.
fn posix_class(name: &str) -> Option<&'static str> {
    POSIX_CLASSES
        .iter()
        .find(|(each, _)| *each == name)
        .map(|(_, class)| *class)
}

/// `scanBracket`: the bracket expression that starts at `pattern[start] === '['`, mirroring the
/// member loop of `wildmatch.c`: where it ends and its source, or `None` when it is never closed
/// or names a class git does not know, which makes the whole pattern match nothing.
fn scan_bracket(pattern: &[u16], start: usize) -> Option<(usize, String)> {
    let length = pattern.len();
    let is = |at: usize, text: &str| unit_at(pattern, at) == text.encode_utf16().next();
    let mut index = start + 1;
    let mut negated = false;
    if is(index, "!") || is(index, "^") {
        negated = true;
        index += 1;
    }
    let mut body: Vec<u16> = Vec::new();
    // The member a `-` could start a range from, or none after a range or a class.
    let mut prev: Option<u16> = None;
    // git scans the members with a do-while, so a leading `]` is a member, not the terminator.
    loop {
        let character = unit_at(pattern, index)?;
        if character == ESCAPE {
            let escaped = unit_at(pattern, index + 1)?;
            escape_member(escaped, &mut body);
            prev = Some(escaped);
            index += 1;
        } else if is(index, "-") && prev.is_some() && index + 1 < length && !is(index + 1, "]") {
            index += 1;
            let mut to = unit_at(pattern, index);
            if to == Some(ESCAPE) {
                index += 1;
                to = unit_at(pattern, index);
            }
            // An out-of-order range matches nothing in git and does not compile in JavaScript, so
            // it is dropped; its lower bound stays a member (`[c-a]` matches `c`).
            if let (Some(from), Some(to)) = (prev, to)
                && from <= to
            {
                body.push(u16::from(b'-'));
                escape_member(to, &mut body);
            }
            prev = None;
        } else if is(index, "[") && is(index + 1, ":") {
            let name_start = index + 2;
            let mut end = name_start;
            while end < length && !is(end, "]") {
                end += 1;
            }
            if end == length {
                return None;
            }
            if end > name_start && is(end - 1, ":") {
                let name = text_of(pattern.get(name_start..end - 1).unwrap_or_default());
                body.extend(units(posix_class(&name)?));
                prev = None;
                index = end;
            } else {
                // No `:]` closes it: the `[` is a plain member and the scan goes on after it.
                escape_member(OPEN, &mut body);
                prev = Some(OPEN);
            }
        } else {
            escape_member(character, &mut body);
            prev = Some(character);
        }
        index += 1;
        if is(index, "]") {
            return Some((index, class_source(negated, &body)));
        }
    }
}

/// An empty class, which never matches: how a pattern git gives up on (`WM_ABORT_ALL`) is spelled.
pub(super) const NEVER_MATCH: &str = "[]";

/// The placeholder a held source stands behind: a NUL is in neither a line nor a path, and a
/// literal one in the pattern is held aside all the same.
pub(super) const PLACEHOLDER: char = '\u{0}';

/// The trailing `*` the chain leaves for `_make` (`pin.rs`), as a private-use marker, so it is
/// never mistaken for an escaped literal `\*`.
pub(super) const TRAILING_WILDCARD: char = '\u{E000}';

/// A trailing `/**`, deferred the same way.
pub(super) const TRAILING_DOUBLESTAR: char = '\u{E001}';

/// The three characters that mean something to the chain: one that came out of the pattern is held
/// aside as a one-member class, so it never lands where a marker sits.
fn is_reserved(character: u16) -> bool {
    [PLACEHOLDER, TRAILING_WILDCARD, TRAILING_DOUBLESTAR]
        .iter()
        .any(|reserved| u32::from(character) == u32::from(*reserved))
}

/// A pattern with its bracket expressions and its quoted characters held aside (`source`), and
/// what each placeholder stands for (`sources`).
pub(super) struct Extracted {
    pub(super) source: String,
    pub(super) sources: Vec<String>,
}

/// `extractBrackets`. A backslash quotes the next character, whatever it is, so `\d` is a literal
/// `d` and `\?` a literal `?`; held aside as its literal, the escape never reaches the replacers.
/// Three escapes are left for the chain: `\*` (a literal star), `\[` (the one `[` the bracket
/// replacer still expects) and `\ ` (a quoted space, which one pass rewrites in bulk).
pub(super) fn extract_brackets(text: &str) -> Extracted {
    let pattern = units(text);
    let mut sources: Vec<String> = Vec::new();
    let mut hold = |source: String, out: &mut Vec<u16>| {
        let held = format!("{PLACEHOLDER}{}{PLACEHOLDER}", sources.len());
        sources.push(source);
        out.extend(held.encode_utf16());
    };
    let length = pattern.len();
    let mut out: Vec<u16> = Vec::new();
    let mut index = 0;
    while let Some(character) = unit_at(&pattern, index) {
        if character == ESCAPE {
            let escaped = unit_at(&pattern, index + 1);
            match escaped {
                // Git removes a final directory slash before matching: an escape with no character
                // after that removal makes the pattern invalid.
                None => hold(NEVER_MATCH.to_owned(), &mut out),
                Some(SLASH) if index + 2 == length => hold(NEVER_MATCH.to_owned(), &mut out),
                Some(quoted) if quoted == STAR || quoted == OPEN || quoted == SPACE => {
                    out.extend([character, quoted]);
                }
                Some(quoted) => {
                    let literal = text_of(&[quoted]);
                    let source = if REGEX_LITERAL_SPECIAL
                        .encode_utf16()
                        .any(|unit| unit == quoted)
                    {
                        format!("\\{literal}")
                    } else if is_reserved(quoted) {
                        format!("[{literal}]")
                    } else {
                        literal
                    };
                    hold(source, &mut out);
                }
            }
            index += 2;
        } else if is_reserved(character) {
            hold(format!("[{}]", text_of(&[character])), &mut out);
            index += 1;
        } else if character == OPEN {
            match scan_bracket(&pattern, index) {
                // git gives up on the whole pattern, so what follows cannot make it match either.
                None => {
                    hold(NEVER_MATCH.to_owned(), &mut out);
                    index = length;
                }
                Some((end, source)) => {
                    hold(source, &mut out);
                    index = end + 1;
                }
            }
        } else {
            out.push(character);
            index += 1;
        }
    }
    Extracted {
        source: text_of(&out),
        sources,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extracted(pattern: &str) -> (String, Vec<String>) {
        let found = extract_brackets(pattern);
        (found.source, found.sources)
    }

    #[test]
    fn a_bracket_expression_is_held_aside_as_its_class() {
        assert_eq!(
            extracted("[a-z]x"),
            ("\u{0}0\u{0}x".into(), vec!["[a-z]".into()])
        );
        assert_eq!(
            extracted("[!a]"),
            ("\u{0}0\u{0}".into(), vec!["[^\\/a]".into()])
        );
        assert_eq!(extracted("[]a]").1, vec!["[\\]a]"]);
        assert_eq!(extracted("[--/]").1, vec!["(?!\\/)[\\--/]"]);
        assert_eq!(extracted("[z-a]").1, vec!["[z]"]);
        assert_eq!(extracted("[[:digit:]]").1, vec!["[0-9]"]);
        assert_eq!(extracted("[[:nope:]]").1, vec![NEVER_MATCH]);
        assert_eq!(extracted("[a").1, vec![NEVER_MATCH]);
    }

    #[test]
    fn an_escape_is_held_as_its_literal_but_three() {
        assert_eq!(extracted("\\d").1, vec!["d"]);
        assert_eq!(extracted("\\?").1, vec!["\\?"]);
        assert_eq!(extracted("\\*\\[\\ ").0, "\\*\\[\\ ");
        assert_eq!(extracted("a\\").1, vec![NEVER_MATCH]);
        assert_eq!(extracted("a\\/").1, vec![NEVER_MATCH]);
    }
}
