//! The chain that turns one gitignore pattern into a regular expression's source (`REPLACERS` and
//! `makeRegexPrefix` in the package), in its order. A step is a matcher and a replacer handed to
//! `String.replace`, skipped outright when a character the matcher cannot match without is not in
//! the string, or plain string work (`Direct`) that takes the source and the pattern.

use std::sync::LazyLock;

use crate::js::code_unit_length;
use crate::js::regexp::{RegExp, RegExpMatch};
use crate::js::utf16::{slice, units};

use super::brackets::{PLACEHOLDER, TRAILING_DOUBLESTAR, TRAILING_WILDCARD, extract_brackets};
use super::{Pattern, group, replace_with, test};

/// One step of the chain.
enum Step {
    /// `[matcher, replacer, required]`: `String.replace`, skipped unless `required` is in the
    /// string.
    Replace {
        matcher: &'static Pattern,
        replacer: fn(&RegExpMatch, &str) -> String,
        required: Option<&'static str>,
    },
    /// `[DIRECT, transform]`: plain string work, taking the source and the pattern.
    Direct(fn(&str, &str) -> String),
}

macro_rules! pattern {
    ($source:expr, $flags:expr) => {{
        static PATTERN: Pattern = LazyLock::new(|| RegExp::new($source, $flags).ok());
        &PATTERN
    }};
}

/// A separator at the beginning or in the middle of a pattern, not at its very end.
static REGEX_INNER_SLASH: Pattern = LazyLock::new(|| RegExp::new(r"\/(?!$)", "").ok());

/// "starting": a pattern with a slash at its beginning or middle is relative to the directory of
/// the `.gitignore` itself, any other may match at any level below it.
fn starting(source: &str, pattern: &str) -> String {
    // Nothing to anchor to, or already anchored.
    if source.is_empty() || source.starts_with('^') {
        return source.to_owned();
    }
    let anchor = if test(&REGEX_INNER_SLASH, pattern) {
        "^"
    } else {
        "(?:^|\\/)"
    };
    format!("{anchor}{source}")
}

/// "two globstars": `/**/` is zero or more directories, a trailing `/**/` at least one, and a
/// trailing `/**` everything inside, left to `_make` as its marker.
fn two_globstars(found: &RegExpMatch, text: &str) -> String {
    let index = found.index;
    if index + 6 < code_unit_length(text) {
        if slice(&units(text), index + 6, usize::MAX) == units("\\/").as_slice() {
            "(?:\\/[^\\/]+)+".to_owned()
        } else {
            "(?:\\/[^\\/]+)*".to_owned()
        }
    } else {
        format!("\\/{TRAILING_DOUBLESTAR}")
    }
}

/// "ending": `js` does not match `js.`; a pattern with a trailing slash matches only a
/// directory, any other a file or a directory. A pending marker is `_make`'s.
fn ending(source: &str, _pattern: &str) -> String {
    match source.chars().last() {
        None | Some(TRAILING_WILDCARD | TRAILING_DOUBLESTAR) => source.to_owned(),
        Some('/') => format!("{source}$"),
        Some(_) => format!("{source}(?=$|\\/$)"),
    }
}

static REPLACERS: LazyLock<[Step; 15]> = LazyLock::new(|| {
    [
        // `\ ` is a space: a run of backslashes is taken whole, so no match starts inside it.
        Step::Replace {
            matcher: pattern!(r"(\\+)( ?)", "g"),
            replacer: |found, _| {
                let run = group(found, 1);
                if group(found, 2).is_empty() {
                    run.to_owned()
                } else {
                    format!(
                        "{} ",
                        run.get(..run.len() - run.len() % 2).unwrap_or_default()
                    )
                }
            },
            required: Some("\\ "),
        },
        // A run of three or more asterisks that fills a segment is one globstar.
        Step::Replace {
            matcher: pattern!(r"(^|\/)\*{3,}(?=\/|$)", "g"),
            replacer: |found, _| format!("{}**", group(found, 1)),
            required: Some("*"),
        },
        // Escape the metacharacters.
        Step::Replace {
            matcher: pattern!(r"[\\$.|*+(){^]", "g"),
            replacer: |found, _| format!("\\{}", group(found, 0)),
            required: None,
        },
        // > a question mark (?) matches a single character
        Step::Replace {
            matcher: pattern!(r"(?!\\)\?", "g"),
            replacer: |_, _| "[^/]".to_owned(),
            required: Some("?"),
        },
        // > A leading slash matches the beginning of the pathname.
        Step::Replace {
            matcher: pattern!(r"^\/", ""),
            replacer: |_, _| "^".to_owned(),
            required: Some("/"),
        },
        // The slashes after the leading one.
        Step::Replace {
            matcher: pattern!(r"\/", "g"),
            replacer: |_, _| "\\/".to_owned(),
            required: Some("/"),
        },
        // > A leading "**" followed by a slash means match in all directories.
        Step::Replace {
            matcher: pattern!(r"^\^*(?:\\\*\\\*\\\/)+", ""),
            replacer: |_, _| "^(?:.*\\/)?".to_owned(),
            required: Some("*"),
        },
        Step::Direct(starting),
        Step::Replace {
            matcher: pattern!(r"\\\/\\\*\\\*(?=\\\/|$)", "g"),
            replacer: two_globstars,
            required: Some("*"),
        },
        // > An asterisk "*" matches anything except a slash; a trailing one is the next step's.
        Step::Replace {
            matcher: pattern!(r"(^|[^\\]+)(\\\*)+(?=.+)", "g"),
            replacer: |found, _| {
                let unescaped = replace_with(pattern!(r"\\\*", "g"), group(found, 2), |_| {
                    "[^\\/]*".to_owned()
                });
                format!("{}{unescaped}", group(found, 1))
            },
            required: Some("*"),
        },
        // A trailing wildcard becomes `_make`'s marker; a trailing literal star stays.
        Step::Replace {
            matcher: pattern!(r"(^|[^\\])((?:\\\\)*)\\\*$", ""),
            replacer: |found, _| {
                if group(found, 2).is_empty() {
                    format!("{}{TRAILING_WILDCARD}", group(found, 1))
                } else {
                    group(found, 0).to_owned()
                }
            },
            required: Some("*"),
        },
        // Unescape: the escaper's doubling taken back out, but for a backslash.
        Step::Replace {
            matcher: pattern!(r"\\\\\\(?=[$.|*+(){^])", "g"),
            replacer: |_, _| "\\".to_owned(),
            required: Some("\\\\"),
        },
        Step::Replace {
            matcher: pattern!(r"\\\\", "g"),
            replacer: |_, _| "\\".to_owned(),
            required: Some("\\\\"),
        },
        // The only `[` left is an escaped, literal one.
        Step::Replace {
            matcher: pattern!(r"\\\[([^\]/]*?)($|\])", "g"),
            replacer: |found, _| format!("\\[{}{}", group(found, 1), group(found, 2)),
            required: Some("["),
        },
        Step::Direct(ending),
    ]
});

/// `REGEX_ANCHORED_GLOBSTAR`: `/` and a run of two or more asterisks.
static REGEX_ANCHORED_GLOBSTAR: Pattern = LazyLock::new(|| RegExp::new(r"^\/\*{2,}$", "").ok());

/// `REGEX_RESTORE_PLACEHOLDER`.
static REGEX_RESTORE_PLACEHOLDER: LazyLock<Option<RegExp>> =
    LazyLock::new(|| RegExp::new(&format!("{PLACEHOLDER}(\\d+){PLACEHOLDER}"), "g").ok());

/// `makeRegexPrefix`: the source of a pattern, its markers still pending. `/**` matches every
/// path, as `**` does.
pub(super) fn make_regex_prefix(body: &str) -> String {
    let pattern = if test(&REGEX_ANCHORED_GLOBSTAR, body) {
        "**"
    } else {
        body
    };
    let extracted = extract_brackets(pattern);
    let mut replaced = extracted.source;
    for step in REPLACERS.iter() {
        replaced = match step {
            Step::Direct(transform) => transform(&replaced, pattern),
            Step::Replace {
                matcher,
                replacer,
                required,
            } => {
                if required.is_some_and(|required| !replaced.contains(required)) {
                    continue;
                }
                let whole = replaced.clone();
                replace_with(matcher, &replaced, |found| replacer(found, &whole))
            }
        };
    }
    if extracted.sources.is_empty() {
        return replaced;
    }
    replace_with(&REGEX_RESTORE_PLACEHOLDER, &replaced, |found| {
        let index: usize = group(found, 1).parse().unwrap_or(usize::MAX);
        extracted
            .sources
            .get(index)
            .cloned()
            .unwrap_or_else(|| "undefined".to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_package_s_patterns_compile() {
        assert!(REGEX_INNER_SLASH.is_some());
        assert!(REGEX_ANCHORED_GLOBSTAR.is_some());
        assert!(REGEX_RESTORE_PLACEHOLDER.is_some());
        for step in REPLACERS.iter() {
            if let Step::Replace { matcher, .. } = step {
                assert!(matcher.is_some());
            }
        }
    }

    #[test]
    fn a_pattern_becomes_the_package_s_source() {
        assert_eq!(make_regex_prefix("a"), "(?:^|\\/)a(?=$|\\/$)");
        assert_eq!(make_regex_prefix("/a/"), "^a\\/$");
        assert_eq!(make_regex_prefix("*.js"), "(?:^|\\/)[^\\/]*\\.js(?=$|\\/$)");
        assert_eq!(make_regex_prefix("a/**"), "^a\\/\u{E001}");
        assert_eq!(make_regex_prefix("**/a"), "^(?:.*\\/)?a(?=$|\\/$)");
        assert_eq!(make_regex_prefix("a*"), "(?:^|\\/)a\u{E000}");
    }
}
