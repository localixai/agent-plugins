//! What turns a range's sugar into plain comparators (`classes/range.js`, from `parseComparator`
//! to `hyphenReplace`): carets, tildes, x-ranges, stars, `>=0.0.0` and hyphen ranges, with the
//! default options (no `includePrerelease`, so the `-0` it adds to lower bounds is never added).
//! A number the package adds one to (`+M + 1`) is a JavaScript number, written as JavaScript
//! writes it (`js::number_to_string`).

use regex::Captures;

use crate::js::{number, number_to_string, trim};

use super::range::{group, replace_with, split_blanks};
use super::re::{BUILD, CARET, GTE0, STAR, TILDE, XRANGE};

/// `+text + 1`, as JavaScript writes it.
fn plus_one(text: &str) -> String {
    number_to_string(number(text) + 1.0)
}

/// `isX(id)`: no part, `x`, `X` or `*`.
fn is_x(id: Option<&str>) -> bool {
    id.is_none_or(|id| id.is_empty() || id.eq_ignore_ascii_case("x") || id == "*")
}

/// `invalidXRangeOrder(M, m, p)`: an x before a number (`x.1`, `1.x.2`).
fn invalid_x_range_order(major: Option<&str>, minor: Option<&str>, patch: Option<&str>) -> bool {
    (is_x(major) && !is_x(minor))
        || (is_x(minor) && patch.is_some_and(|p| !p.is_empty()) && !is_x(patch))
}

/// `parseComparator(comp)`: the build cut, then carets, tildes, x-ranges and stars replaced.
pub(super) fn parse_comparator(comp: &str) -> String {
    let comp = replace_with(BUILD.as_ref(), comp, false, |_| String::new());
    let comp = replace_carets(&comp);
    let comp = replace_tildes(&comp);
    let comp = replace_x_ranges(&comp);
    replace_stars(&comp)
}

/// Each blank-separated part of `comp` (trimmed) replaced by `one`, joined by a space.
fn each_part(comp: &str, one: fn(&str) -> String) -> String {
    let parts: Vec<String> = split_blanks(comp).into_iter().map(one).collect();
    parts.join(" ")
}

/// The groups `M`, `m`, `p` and `pr` of a tilde, caret or x-range match, from `first`.
fn parts<'t>(found: &Captures<'t>, first: usize) -> [Option<&'t str>; 4] {
    [first, first + 1, first + 2, first + 3].map(|index| group(found, index))
}

/// `~`, `~>` (any); `~2`, `~2.x` (`>=2.0.0 <3.0.0-0`); `~1.2` (`>=1.2.0 <1.3.0-0`); `~1.2.3`
/// (`>=1.2.3 <1.3.0-0`); `~0.0.1` (`>=0.0.1 <0.1.0-0`).
fn replace_tildes(comp: &str) -> String {
    each_part(trim(comp), replace_tilde)
}

fn replace_tilde(comp: &str) -> String {
    replace_with(TILDE.as_ref(), comp, false, |found| {
        let [major, minor, patch, pre] = parts(found, 1);
        let (major_text, minor_text) = (major.unwrap_or_default(), minor.unwrap_or_default());
        let patch_text = patch.unwrap_or_default();
        if is_x(major) {
            String::new()
        } else if is_x(minor) {
            format!(">={major_text}.0.0 <{}.0.0-0", plus_one(major_text))
        } else if is_x(patch) {
            format!(
                ">={major_text}.{minor_text}.0 <{major_text}.{}.0-0",
                plus_one(minor_text)
            )
        } else if let Some(pre) = pre.filter(|pre| !pre.is_empty()) {
            format!(
                ">={major_text}.{minor_text}.{patch_text}-{pre} <{major_text}.{}.0-0",
                plus_one(minor_text)
            )
        } else {
            format!(
                ">={major_text}.{minor_text}.{patch_text} <{major_text}.{}.0-0",
                plus_one(minor_text)
            )
        }
    })
}

/// `^` (any); `^2`, `^2.x`, `^2.0` (`>=2.0.0 <3.0.0-0`); `^1.2.3` (`>=1.2.3 <2.0.0-0`); `^0.0.1`
/// (`>=0.0.1 <0.0.2-0`); `^0.1.0` (`>=0.1.0 <0.2.0-0`).
fn replace_carets(comp: &str) -> String {
    each_part(trim(comp), replace_caret)
}

fn replace_caret(comp: &str) -> String {
    replace_with(CARET.as_ref(), comp, false, |found| {
        let [major, minor, patch, pre] = parts(found, 1);
        let (major_text, minor_text) = (major.unwrap_or_default(), minor.unwrap_or_default());
        let patch_text = patch.unwrap_or_default();
        if is_x(major) {
            return String::new();
        }
        if is_x(minor) {
            return format!(">={major_text}.0.0 <{}.0.0-0", plus_one(major_text));
        }
        if is_x(patch) {
            let upper = if major_text == "0" {
                format!("{major_text}.{}.0-0", plus_one(minor_text))
            } else {
                format!("{}.0.0-0", plus_one(major_text))
            };
            return format!(">={major_text}.{minor_text}.0 <{upper}");
        }
        let lower = match pre.filter(|pre| !pre.is_empty()) {
            Some(pre) => format!("{major_text}.{minor_text}.{patch_text}-{pre}"),
            None => format!("{major_text}.{minor_text}.{patch_text}"),
        };
        let upper = match (major_text, minor_text) {
            ("0", "0") => format!("{major_text}.{minor_text}.{}-0", plus_one(patch_text)),
            ("0", _) => format!("{major_text}.{}.0-0", plus_one(minor_text)),
            _ => format!("{}.0.0-0", plus_one(major_text)),
        };
        format!(">={lower} <{upper}")
    })
}

fn replace_x_ranges(comp: &str) -> String {
    each_part(comp, replace_x_range)
}

fn replace_x_range(comp: &str) -> String {
    let comp = trim(comp);
    replace_with(XRANGE.as_ref(), comp, false, |found| {
        let whole = group(found, 0).unwrap_or_default();
        let gtlt = group(found, 1).unwrap_or_default();
        let [major, minor, patch, _] = parts(found, 2);
        if invalid_x_range_order(major, minor, patch) {
            return comp.to_owned();
        }
        x_range(whole, gtlt, [major, minor, patch])
    })
}

/// One x-range's comparators (`replaceXRange`'s callback, past its order check).
fn x_range(whole: &str, gtlt: &str, [major, minor, patch]: [Option<&str>; 3]) -> String {
    let x_major = is_x(major);
    let x_minor = x_major || is_x(minor);
    let any_x = x_minor || is_x(patch);
    let gtlt = if gtlt == "=" && any_x { "" } else { gtlt };
    let major_text = major.unwrap_or_default().to_owned();
    let minor_text = minor.unwrap_or_default().to_owned();
    if x_major {
        // Nothing is allowed, or nothing is forbidden.
        return if gtlt == ">" || gtlt == "<" {
            "<0.0.0-0"
        } else {
            "*"
        }
        .to_owned();
    }
    if !gtlt.is_empty() && any_x {
        // The patch is an x: an x is 0, and `>1` is `>=2.0.0`, `<=1.2` is `<1.3.0-0`.
        let (mut major, mut minor) = (
            major_text,
            if x_minor { "0".to_owned() } else { minor_text },
        );
        let gtlt = match gtlt {
            ">" => {
                if x_minor {
                    major = plus_one(&major);
                    "0".clone_into(&mut minor);
                } else {
                    minor = plus_one(&minor);
                }
                ">="
            }
            "<=" => {
                if x_minor {
                    major = plus_one(&major);
                } else {
                    minor = plus_one(&minor);
                }
                "<"
            }
            other => other,
        };
        let pre = if gtlt == "<" { "-0" } else { "" };
        return format!("{gtlt}{major}.{minor}.0{pre}");
    }
    if x_minor {
        return format!(">={major_text}.0.0 <{}.0.0-0", plus_one(&major_text));
    }
    if any_x {
        return format!(
            ">={major_text}.{minor_text}.0 <{major_text}.{}.0-0",
            plus_one(&minor_text)
        );
    }
    whole.to_owned()
}

/// `*` is ANDed with everything else, and `''` means any version: the first `*` is removed.
fn replace_stars(comp: &str) -> String {
    replace_with(STAR.as_ref(), trim(comp), false, |_| String::new())
}

/// `>=0.0.0` is `*`.
pub(super) fn replace_gte0(comp: &str) -> String {
    replace_with(GTE0.as_ref(), trim(comp), false, |_| String::new())
}

/// `hyphenReplace`: `1.2 - 3.4.5` is `>=1.2.0 <=3.4.5`, `1.2.3 - 3.4` is `>=1.2.3 <3.5.0-0`.
pub(super) fn hyphen_replace(found: &Captures<'_>) -> String {
    let from = group(found, 1).unwrap_or_default();
    let [from_major, from_minor, from_patch, _] = parts(found, 2);
    let to = group(found, 7).unwrap_or_default();
    let [to_major, to_minor, to_patch, to_pre] = parts(found, 8);
    let (fm, fn_) = (
        from_major.unwrap_or_default(),
        from_minor.unwrap_or_default(),
    );
    let (tm, tn, tp) = (
        to_major.unwrap_or_default(),
        to_minor.unwrap_or_default(),
        to_patch.unwrap_or_default(),
    );
    let lower = if is_x(from_major) {
        String::new()
    } else if is_x(from_minor) {
        format!(">={fm}.0.0")
    } else if is_x(from_patch) {
        format!(">={fm}.{fn_}.0")
    } else {
        format!(">={from}")
    };
    let upper = if is_x(to_major) {
        String::new()
    } else if is_x(to_minor) {
        format!("<{}.0.0-0", plus_one(tm))
    } else if is_x(to_patch) {
        format!("<{tm}.{}.0-0", plus_one(tn))
    } else if let Some(pre) = to_pre.filter(|pre| !pre.is_empty()) {
        format!("<={tm}.{tn}.{tp}-{pre}")
    } else {
        format!("<={to}")
    };
    trim(&format!("{lower} {upper}")).to_owned()
}
