//! Gitignore patterns as the npm package `ignore` 7.0.14 matches them, which the path rules were
//! matched by (`src/rules/path/glob.ts`): `ignore({ ignorecase }).add(lines).ignores(path)`. Each
//! line becomes a JavaScript regular expression by the package's own chain, mirrored here step for
//! step and in its order (`brackets.rs`, `replacers.rs`, `pin.rs`, `rule.rs`), and the source it
//! builds runs as JavaScript's `RegExp` (`js::regexp`), so the semantics are the package's and not
//! a reading of gitignore(5) of our own. `tests/differential/ignore.rs` holds it to what Bun
//! answered (`scripts/rust/js-oracle/ignore.ts`).
//!
//! Only what `glob.ts` asks is here: `add` of an array of lines and `ignores`, with the package's
//! strict path check. Left out: `checkIgnore` and its `checkRegex`, `test`, `filter`,
//! `createFilter`, a line that is an `Ignore` itself, and the conversion the package makes of
//! `\` to `/` when its process runs on Windows: the core has no process, and the rules hand it
//! paths in POSIX form already (`posix.rs`), so it answers as the package does off Windows.
//!
//! Decision: the package keeps every answer of `ignores` by path, a cache that changes no answer;
//! the core keeps none, and walks a path's parents again on each call. A rule's regular expression
//! is still made on its first use and kept, as the package's getter makes it: a line whose
//! expression JavaScriptCore refuses throws only when a path reaches it.
//!
//! The package is MIT licensed; this port keeps its notice:
//!
//! Copyright (c) 2013 Kael Zhang <i@kael.me>, contributors
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy of this software
//! and associated documentation files (the "Software"), to deal in the Software without
//! restriction, including without limitation the rights to use, copy, modify, merge, publish,
//! distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
//! Software is furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all copies or
//! substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
//! BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
//! NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

mod brackets;
mod pin;
mod replacers;
mod rule;

use std::sync::LazyLock;

use crate::js::regexp::{RegExp, RegExpMatch};

use rule::{IgnoreRule, create_rule, matches_basename};

/// A pattern of the package's own, compiled once as JavaScript's `RegExp`; `None` only for one
/// that does not compile, which the unit tests rule out.
type Pattern = LazyLock<Option<RegExp>>;

/// `pattern.test(text)`.
fn test(pattern: &Pattern, text: &str) -> bool {
    pattern
        .as_ref()
        .is_some_and(|pattern| pattern.test_shared(text))
}

/// `text.replace(pattern, replacer)`, on a copy of the shared pattern (a global one writes its
/// `lastIndex`).
fn replace_with(
    pattern: &Pattern,
    text: &str,
    replacer: impl FnMut(&RegExpMatch) -> String,
) -> String {
    match pattern.as_ref() {
        Some(pattern) => pattern.clone().replace_with(text, replacer),
        None => text.to_owned(),
    }
}

/// The text of capture `index` of a match, `''` for a group that took no part (where the package
/// reads `undefined` only as falsy).
fn group(found: &RegExpMatch, index: usize) -> &str {
    found
        .captures
        .get(index)
        .and_then(Option::as_deref)
        .unwrap_or_default()
}

const SLASH: char = '/';
const DOUBLE_SLASH: &str = "//";

/// `REGEX_INVALID_TRAILING_BACKSLASH`.
static REGEX_INVALID_TRAILING_BACKSLASH: Pattern =
    LazyLock::new(|| RegExp::new(r"(?:[^\\]|^)\\$", "").ok());

/// The last segment of a path, keeping a trailing slash, because a pattern that ends in one
/// matches only a directory. `a/b/c.js` is `c.js`, `a/b/` is `b/`, `c.js` is itself.
fn basename_of(path: &str) -> &str {
    let from = if path.ends_with(SLASH) {
        path.len().saturating_sub(2)
    } else {
        path.len().saturating_sub(1)
    };
    match path.get(..=from).and_then(|head| head.rfind(SLASH)) {
        Some(index) => path.get(index + 1..).unwrap_or_default(),
        None => path,
    }
}

/// The parent directory of a path, with its trailing separator, or `''` when the path has none:
/// `a/b/c` is `a/b/`, `a/b/` is `a/`, `a` and `a/` are `''`. A path holding an empty segment is
/// taken apart, because its ancestors are not prefixes of it (`a//b` is below `a/`).
fn parent_of(path: &str) -> String {
    if path.starts_with(SLASH) || path.contains(DOUBLE_SLASH) {
        let mut slices: Vec<&str> = path
            .split(SLASH)
            .filter(|slice| !slice.is_empty())
            .collect();
        slices.pop();
        return if slices.is_empty() {
            String::new()
        } else {
            format!("{}{SLASH}", slices.join("/"))
        };
    }
    let from = if path.ends_with(SLASH) {
        path.len().saturating_sub(2)
    } else {
        path.len().saturating_sub(1)
    };
    match path.get(..=from).and_then(|head| head.rfind(SLASH)) {
        Some(cut) => path.get(..=cut).unwrap_or_default().to_owned(),
        None => String::new(),
    }
}

/// `checkPattern`: a line that is no text, empty, or ends in one backslash is no rule; blank
/// lines and comments are told by `create_rule`, once the pattern is normalized.
fn check_pattern(pattern: &str) -> bool {
    !pattern.is_empty() && !test(&REGEX_INVALID_TRAILING_BACKSLASH, pattern)
}

/// What a path tested to (`TestResult`): excluded by a rule, or included again by a `!` rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TestResult {
    ignored: bool,
    unignored: bool,
}

/// `RuleManager`: the rules in the order added.
#[derive(Debug)]
struct RuleManager {
    ignore_case: bool,
    rules: Vec<IgnoreRule>,
    /// How many of the rules git would tag `EXC_FLAG_NODIR`: whether handing those the basename
    /// is worth what it costs the others is decided once for the whole set, and changes no answer.
    basename_count: usize,
}

impl RuleManager {
    fn add(&mut self, pattern: &str) {
        let rule = if check_pattern(pattern) {
            create_rule(pattern, self.ignore_case)
        } else {
            None
        };
        if let Some(rule) = rule {
            if matches_basename(&rule.body) {
                self.basename_count += 1;
            }
            self.rules.push(rule);
        }
    }

    /// One path tested without its parents; the last rule to match decides, as in git.
    fn test(&self, path: &str) -> Result<TestResult, String> {
        let shortcut = self.basename_count * 2 >= self.rules.len();
        let basename = if shortcut { basename_of(path) } else { path };
        for rule in self.rules.iter().rev() {
            let text = if shortcut && rule.basename_only() {
                basename
            } else {
                path
            };
            if rule.regex()?.test_shared(text) {
                return Ok(TestResult {
                    ignored: !rule.negative,
                    unignored: rule.negative,
                });
            }
        }
        Ok(TestResult::default())
    }
}

/// `checkPath.isNotRelative`: a path that begins with a separator, `./` or `../`, or is `.` or
/// `..`.
fn is_not_relative(path: &str) -> bool {
    path.starts_with(SLASH)
        || path == "."
        || path == ".."
        || path.starts_with("./")
        || path.starts_with("../")
}

/// `checkPath` with the package's strict check: what it throws for a path it does not test.
fn check_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("path must not be empty".to_owned());
    }
    if is_not_relative(path) {
        return Err(format!(
            "path should be a `path.relative()`d string, but got \"{path}\""
        ));
    }
    Ok(())
}

/// What `ignore({ ignorecase })` makes, its lines added.
#[derive(Debug)]
pub struct Ignore {
    rules: RuleManager,
}

impl Ignore {
    /// `ignore({ ignorecase })`.
    #[must_use]
    pub const fn new(ignorecase: bool) -> Self {
        Self {
            rules: RuleManager {
                ignore_case: ignorecase,
                rules: Vec::new(),
                basename_count: 0,
            },
        }
    }

    /// `.add(lines)`: each line that is a rule, in order.
    #[must_use]
    #[expect(
        clippy::should_implement_trait,
        reason = "the package's own name for adding lines, which is no sum"
    )]
    pub fn add(mut self, lines: &[String]) -> Self {
        for line in lines {
            self.rules.add(line);
        }
        self
    }

    /// `_t`: a path whose parent directory is excluded stays excluded ("It is not possible to
    /// re-include a file if a parent directory of that file is excluded").
    fn test_with_parents(&self, path: &str) -> Result<TestResult, String> {
        let parent_path = parent_of(path);
        if !parent_path.is_empty() {
            let parent = self.test_with_parents(&parent_path)?;
            if parent.ignored {
                return Ok(parent);
            }
        }
        self.rules.test(path)
    }

    /// `.ignores(path)`: whether the lines exclude `path`, a `path.relative()`d text; the `Err`
    /// is what the package throws, for a path it refuses or a rule's expression JavaScriptCore
    /// refuses.
    pub fn ignores(&self, path: &str) -> Result<bool, String> {
        check_path(path)?;
        Ok(self.test_with_parents(path)?.ignored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_package_s_patterns_compile() {
        assert!(REGEX_INVALID_TRAILING_BACKSLASH.is_some());
    }

    #[test]
    fn a_path_is_taken_apart_as_the_package_takes_it() {
        assert_eq!(basename_of("a/b/c.js"), "c.js");
        assert_eq!(basename_of("a/b/"), "b/");
        assert_eq!(basename_of("c.js"), "c.js");
        assert_eq!(parent_of("a/b/c"), "a/b/");
        assert_eq!(parent_of("a/b/"), "a/");
        assert_eq!(parent_of("a"), "");
        assert_eq!(parent_of("a/"), "");
        assert_eq!(parent_of("a//b"), "a/");
    }

    #[test]
    fn a_directory_excluded_keeps_its_files_excluded() {
        let lines = ["a/".to_owned(), "!a/b".to_owned()];
        let ignore = Ignore::new(false).add(&lines);
        assert_eq!(ignore.ignores("a/b"), Ok(true));
        assert_eq!(ignore.ignores("b"), Ok(false));
        assert!(ignore.ignores("./a").is_err());
    }
}
