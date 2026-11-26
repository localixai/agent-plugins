//! The package's patterns (`internal/re.js`): each token's source built as it builds it, the
//! "safe" copy made by bounding `\s*`, `\s+`, `\d*`, `\d+` and the letter-digit-dash class as it
//! bounds them (`makeSafeRegex`), and compiled as JavaScript's `RegExp` (`js::regexp`) with the
//! package's flags: none, or `g` for a token it creates global. Only the strict tokens the ranges
//! and versions read are here.
//!
//! A compiled token is shared, and `exec` and `replace` write a `RegExp`'s `lastIndex`; so each
//! use runs on a copy of it (`exec`, `replace`, `replace_with` below), which no other use sees.

use std::sync::LazyLock;

use crate::js::regexp::{RegExp, RegExpMatch};

/// `MAX_LENGTH`: the longest version text.
pub(super) const MAX_LENGTH: usize = 256;
/// `MAX_SAFE_BUILD_LENGTH`: the longest build identifier, `MAX_LENGTH` less `0.0.0+`.
pub(super) const MAX_SAFE_BUILD_LENGTH: usize = MAX_LENGTH - 6;

const LETTERDASHNUMBER: &str = "[a-zA-Z0-9-]";

/// `makeSafeRegex`: each token's `*` and `+` bounded, `\s` to one, `\d` to `MAX_LENGTH` and the
/// class to `MAX_SAFE_BUILD_LENGTH`, in that order.
fn make_safe(value: &str) -> String {
    let replacements = [
        ("\\s", 1),
        ("\\d", MAX_LENGTH),
        (LETTERDASHNUMBER, MAX_SAFE_BUILD_LENGTH),
    ];
    let mut value = value.to_owned();
    for (token, max) in replacements {
        value = value
            .replace(&format!("{token}*"), &format!("{token}{{0,{max}}}"))
            .replace(&format!("{token}+"), &format!("{token}{{1,{max}}}"));
    }
    value
}

/// The sources the tokens are built from, in the package's order.
struct Sources {
    build: String,
    full_plain: String,
    gtlt: String,
    xrange_plain: String,
}

fn sources() -> Sources {
    let numeric_identifier = "0|[1-9]\\d*";
    let nonnumeric_identifier = format!("\\d*[a-zA-Z-]{LETTERDASHNUMBER}*");
    let main_version =
        format!("({numeric_identifier})\\.({numeric_identifier})\\.({numeric_identifier})");
    let prerelease_identifier = format!("(?:{nonnumeric_identifier}|{numeric_identifier})");
    let prerelease = format!("(?:-({prerelease_identifier}(?:\\.{prerelease_identifier})*))");
    let build_identifier = format!("{LETTERDASHNUMBER}+");
    let build = format!("(?:\\+({build_identifier}(?:\\.{build_identifier})*))");
    let full_plain = format!("v?{main_version}{prerelease}?{build}?");
    let xrange_identifier = format!("{numeric_identifier}|x|X|\\*");
    let xrange_plain = format!(
        "[v=\\s]*({xrange_identifier})(?:\\.({xrange_identifier})(?:\\.({xrange_identifier})(?:{prerelease})?{build}?)?)?"
    );
    Sources {
        build,
        full_plain,
        gtlt: "((?:<|>)?=?)".to_owned(),
        xrange_plain,
    }
}

static SOURCES: LazyLock<Sources> = LazyLock::new(sources);

/// A token's safe pattern, compiled with no flags; `None` only if it does not compile (a bug the
/// unit test below would show).
fn safe(source: &str) -> Option<RegExp> {
    RegExp::new(&make_safe(source), "").ok()
}

/// A global token's safe pattern, compiled with `g`.
fn safe_global(source: &str) -> Option<RegExp> {
    RegExp::new(&make_safe(source), "g").ok()
}

/// A compiled token.
pub(super) type Token = LazyLock<Option<RegExp>>;

/// `text.match(token)` of a token that is not global: its `exec`.
pub(super) fn exec(token: &Token, text: &str) -> Option<RegExpMatch> {
    token.as_ref()?.clone().exec(text)
}

/// `text.replace(token, replacement)`, `$1` and the like read as JavaScript reads them; the text
/// as it is when the token did not compile.
pub(super) fn replace(token: &Token, text: &str, replacement: &str) -> String {
    token.as_ref().map_or_else(
        || text.to_owned(),
        |token| token.clone().replace(text, replacement),
    )
}

/// `text.replace(token, (match, ...groups) => …)`.
pub(super) fn replace_with(
    token: &Token,
    text: &str,
    replacement: impl FnMut(&RegExpMatch) -> String,
) -> String {
    token.as_ref().map_or_else(
        || text.to_owned(),
        |token| token.clone().replace_with(text, replacement),
    )
}

/// The text of a capture group of a match, `None` when it took no part (`undefined`).
pub(super) fn group(found: &RegExpMatch, index: usize) -> Option<&str> {
    found.captures.get(index).and_then(Option::as_deref)
}

/// `FULL`: a whole strict version.
pub(super) static FULL: Token = LazyLock::new(|| safe(&format!("^{}$", SOURCES.full_plain)));
/// `BUILD`: build metadata, `+` and its identifiers.
pub(super) static BUILD: Token = LazyLock::new(|| safe(&SOURCES.build));
/// The unbounded `BUILD` `parseRange` strips every build with (`BUILDSTRIPRE`, not the safe copy,
/// `new RegExp(src[t.BUILD], 'g')`).
pub(super) static BUILD_STRIP: Token = LazyLock::new(|| RegExp::new(&SOURCES.build, "g").ok());
/// `XRANGE`: an operator and a partial version.
pub(super) static XRANGE: Token =
    LazyLock::new(|| safe(&format!("^{}\\s*{}$", SOURCES.gtlt, SOURCES.xrange_plain)));
/// `TILDETRIM`: the blanks after a `~`, replaced by `$1~`.
pub(super) static TILDE_TRIM: Token = LazyLock::new(|| safe_global("(\\s*)(?:~>?)\\s+"));
/// `TILDE`: a tilde range.
pub(super) static TILDE: Token =
    LazyLock::new(|| safe(&format!("^(?:~>?){}$", SOURCES.xrange_plain)));
/// `CARETTRIM`: the blanks after a `^`, replaced by `$1^`.
pub(super) static CARET_TRIM: Token = LazyLock::new(|| safe_global("(\\s*)(?:\\^)\\s+"));
/// `CARET`: a caret range.
pub(super) static CARET: Token =
    LazyLock::new(|| safe(&format!("^(?:\\^){}$", SOURCES.xrange_plain)));
/// `COMPARATOR`: an operator and a strict version, or nothing.
pub(super) static COMPARATOR: Token = LazyLock::new(|| {
    safe(&format!(
        "^{}\\s*({})$|^$",
        SOURCES.gtlt, SOURCES.full_plain
    ))
});
/// `HYPHENRANGE`: `1.2.3 - 2.3.4`.
pub(super) static HYPHEN_RANGE: Token = LazyLock::new(|| {
    let plain = &SOURCES.xrange_plain;
    safe(&format!("^\\s*({plain})\\s+-\\s+({plain})\\s*$"))
});
/// `STAR`: `*` with any operator.
pub(super) static STAR: Token = LazyLock::new(|| safe("(<|>)?=?\\s*\\*"));
/// `GTE0`: `>=0.0.0`, which is any version.
pub(super) static GTE0: Token = LazyLock::new(|| safe("^\\s*>=\\s*0\\.0\\.0\\s*$"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_token_compiles_and_is_bounded() {
        for token in [
            &FULL,
            &BUILD,
            &BUILD_STRIP,
            &XRANGE,
            &TILDE_TRIM,
            &TILDE,
            &CARET_TRIM,
            &CARET,
            &COMPARATOR,
            &HYPHEN_RANGE,
            &STAR,
            &GTE0,
        ] {
            assert!(token.is_some());
        }
        assert_eq!(make_safe("0|[1-9]\\d*"), "0|[1-9]\\d{0,256}");
        assert_eq!(make_safe("(\\s*)~\\s+"), "(\\s{0,1})~\\s{1,1}");
        assert_eq!(make_safe("[a-zA-Z0-9-]+"), "[a-zA-Z0-9-]{1,250}");
    }
}
