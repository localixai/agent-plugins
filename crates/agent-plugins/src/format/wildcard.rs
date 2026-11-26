//! Port of `src/format/wildcard.ts`, switched.
//! The crate's one `*` wildcard: `*` is any run of characters (none included), every other
//! character is literal and the whole text must match. The settings URL patterns
//! (`allowedHttpHookUrls`, the MCP `serverUrl` entries) match with `matches_wildcard`; a pattern
//! that needs flags or a narrower `*` (a rule's tool name, a path segment, a domain label)
//! compiles `wildcard_source`.

/// What a `*` matches unless a caller narrows it: any text, line breaks included.
pub const ANY_TEXT: &str = "[\\s\\S]*";

/// `text` as regular-expression source that matches only itself.
#[must_use]
pub fn escape_reg_exp(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(
            character,
            '.' | '*' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\'
        ) {
            out.push('\\');
        }
        out.push(character);
    }
    out
}

/// A `*` pattern as regular-expression source, `star` what a `*` matches (any text by default).
#[must_use]
pub fn wildcard_source(pattern: &str, star: Option<&str>) -> String {
    let star = star.unwrap_or(ANY_TEXT);
    pattern
        .split('*')
        .map(escape_reg_exp)
        .collect::<Vec<_>>()
        .join(star)
}

/// Whether `text` matches `pattern` whole, each `*` any run of characters. Linear, with no
/// regular expression; a caller that matches without case folds both sides first.
#[must_use]
pub fn matches_wildcard(pattern: &str, text: &str) -> bool {
    let mut parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return text == pattern;
    }
    let first = parts.remove(0);
    let last = parts.pop().unwrap_or_default();
    let Some(end) = text.len().checked_sub(last.len()) else {
        return false;
    };
    if end < first.len() || !text.starts_with(first) || !text.ends_with(last) {
        return false;
    }
    let mut at = first.len();
    for middle in parts {
        let Some(found) = text.get(at..).and_then(|rest| rest.find(middle)) else {
            return false;
        };
        if at + found + middle.len() > end {
            return false;
        }
        at += found + middle.len();
    }
    true
}

crate::registry::exports! {
    "format/wildcard" {
        "escapeRegExp" => ok |text: String| escape_reg_exp(&text);
        "wildcardSource" => ok |pattern: String, star: Option<String>| wildcard_source(&pattern, star.as_deref());
        "matchesWildcard" => ok |pattern: String, text: String| matches_wildcard(&pattern, &text);
    }
}
