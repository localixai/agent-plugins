//! One line made a rule (`IgnoreRule` and `createRule` in the package): its BOM, comment, `!` and
//! trailing blanks read, its regular expression's source built once (`replacers.rs`) and the
//! expression made on its first use.

use std::sync::{LazyLock, OnceLock};

use crate::js::regexp::RegExp;

use super::pin::ignore_source;
use super::replacers::make_regex_prefix;
use super::{Pattern, SLASH};

const BOM: char = '\u{FEFF}';
const SPACE: char = ' ';

/// `REGEX_REPLACE_LEADING_EXCAPED_EXCLAMATION`.
static REGEX_REPLACE_LEADING_EXCAPED_EXCLAMATION: Pattern =
    LazyLock::new(|| RegExp::new(r"^\\!", "").ok());
/// `REGEX_REPLACE_LEADING_EXCAPED_HASH`.
static REGEX_REPLACE_LEADING_EXCAPED_HASH: Pattern =
    LazyLock::new(|| RegExp::new(r"^\\#", "").ok());

/// A rule: the line it came from read, and its expression.
#[derive(Debug)]
pub(super) struct IgnoreRule {
    /// The line once its BOM, `!` and trailing blanks are read.
    pub(super) body: String,
    ignore_case: bool,
    /// Whether the rule is a `!` negation.
    pub(super) negative: bool,
    regex_prefix: String,
    /// The expression of `regex`, made on first use and kept (the package's `_regex`).
    regex: OnceLock<Result<RegExp, String>>,
}

impl IgnoreRule {
    /// `_basenameOnly`.
    pub(super) fn basename_only(&self) -> bool {
        matches_basename(&self.body)
    }

    /// `regex`: `_make` in `regex` mode, `i` when the case is ignored; the `Err` is the
    /// `SyntaxError` JavaScriptCore throws for it.
    pub(super) fn regex(&self) -> Result<&RegExp, String> {
        self.regex
            .get_or_init(|| {
                let flags = if self.ignore_case { "i" } else { "" };
                RegExp::new(&ignore_source(&self.regex_prefix), flags)
                    .map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(Clone::clone)
    }
}

/// Whether a pattern can only describe a path's last segment: no separator in it, or only a
/// trailing one, which restricts the match to a directory without reaching across one.
pub(super) fn matches_basename(body: &str) -> bool {
    match body.find(SLASH) {
        None => true,
        Some(index) => index + 1 == body.len(),
    }
}

const fn is_line_end(character: u8) -> bool {
    character == b'\r' || character == b'\n'
}

/// A line terminator dropped, then the trailing spaces; a space escaped by an odd run of
/// backslashes stays, unescaped, as the last one. `a  ` is `a`, `a\ ` is `a `, `a\\ ` is `a\\`,
/// `a\t` stays.
fn trim_end(body: &str) -> String {
    let bytes = body.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes.get(end - 1).copied().is_some_and(is_line_end) {
        end -= 1;
    }
    let line_end = end;
    while end > 0 && bytes.get(end - 1) == Some(&b' ') {
        end -= 1;
    }
    let kept = |end: usize| body.get(..end).unwrap_or_default().to_owned();
    if end == line_end {
        return kept(end);
    }
    let mut backslashes = 0;
    while backslashes < end && bytes.get(end - backslashes - 1) == Some(&b'\\') {
        backslashes += 1;
    }
    if backslashes % 2 == 1 {
        format!("{}{SPACE}", kept(end - 1))
    } else {
        kept(end)
    }
}

/// `createRule`: `None` for a comment and for a line left empty (a lone `!` included), which an
/// expression would make match every path.
pub(super) fn create_rule(pattern: &str, ignore_case: bool) -> Option<IgnoreRule> {
    let mut body = pattern.strip_prefix(BOM).unwrap_or(pattern).to_owned();
    // > A line starting with # serves as a comment.
    if body.starts_with('#') {
        return None;
    }
    // > An optional prefix "!" which negates the pattern;
    let negative = body.starts_with('!');
    if negative {
        body.remove(0);
    }
    // Normalized before the slashes that anchor it are read: `bar/ ` holds no inner slash.
    if body.ends_with([SPACE, '\r', '\n']) {
        body = trim_end(&body);
    }
    // > A blank line matches no files, so it can serve as a separator for readability.
    if body.is_empty() {
        return None;
    }
    // > Put a backslash ("\") in front of the first "!" for patterns that begin with a literal
    // > "!", for example, `"\!important!.txt"`; and in front of the first hash.
    let body = super::replace_with(&REGEX_REPLACE_LEADING_EXCAPED_EXCLAMATION, &body, |_| {
        "!".to_owned()
    });
    let body = super::replace_with(&REGEX_REPLACE_LEADING_EXCAPED_HASH, &body, |_| {
        "#".to_owned()
    });
    let regex_prefix = make_regex_prefix(&body);
    Some(IgnoreRule {
        body,
        ignore_case,
        negative,
        regex_prefix,
        regex: OnceLock::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_package_s_patterns_compile() {
        assert!(REGEX_REPLACE_LEADING_EXCAPED_EXCLAMATION.is_some());
        assert!(REGEX_REPLACE_LEADING_EXCAPED_HASH.is_some());
    }

    #[test]
    fn trailing_spaces_are_trimmed_unless_escaped() {
        assert_eq!(trim_end("a  "), "a");
        assert_eq!(trim_end("a\\ "), "a ");
        assert_eq!(trim_end("a\\\\ "), "a\\\\");
        assert_eq!(trim_end("a\r\n"), "a");
        assert_eq!(trim_end("a\t"), "a\t");
    }

    #[test]
    fn a_comment_and_an_empty_line_are_no_rules() {
        assert!(create_rule("#a", false).is_none());
        assert!(create_rule("!", false).is_none());
        assert!(create_rule("\u{FEFF}#a", false).is_none());
        let rule = create_rule("!\\#a", false).unwrap();
        assert!(rule.negative);
        assert_eq!(rule.body, "#a");
    }
}
