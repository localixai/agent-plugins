//! Port of `src/settings/hooks/url-pattern.ts`, switched.
//!
//! The URL pattern of `allowedHttpHookUrls` (settings-reference.md: "array of URL patterns, with
//! `*` as a wildcard"; "Hostname matching is case-insensitive and treats `hooks.example.com.`,
//! with the trailing dot that marks a fully qualified domain name, the same as
//! `hooks.example.com`").
//!
//! The pattern and the address are compared as text after one normalization, which is the kit's
//! judgment where the docs are silent (spec/judgments.md): the scheme and the host are lower case,
//! a trailing dot on the host is dropped, and an address whose path is empty or a lone `/` is tried
//! in both spellings. The path, the port and the user information are compared as written, with
//! one exception: a pattern whose port is the wildcard (`http://localhost:*`) also matches the
//! same host with no port, because the docs read that example as "any `http://localhost` URL". `*`
//! stands for any run of characters, `/` included, and every other character matches itself.

use crate::format::wildcard::matches_wildcard;

/// `^([a-z][a-z0-9+.-]*:\/\/)` of the `i`-flagged ADDRESS pattern: the scheme with its `://`, and
/// what follows it.
fn split_scheme(text: &str) -> Option<(&str, &str)> {
    let colon = text.find(':')?;
    let scheme = text.get(..colon)?;
    let mut characters = scheme.chars();
    let valid = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
        });
    let rest = text.get(colon..)?.strip_prefix("://")?;
    valid.then_some((scheme, rest))
}

/// `[::1]:80` or `host:80` as a host and the rest, which holds the port
/// (`/^(\[[^\]]*\]|[^:]*)([\s\S]*)$/`).
fn host_and_port(text: &str) -> (&str, &str) {
    let end = match (text.starts_with('['), text.find(']')) {
        (true, Some(close)) => close + 1,
        (true, None) | (false, _) => text.find(':').unwrap_or(text.len()),
    };
    (
        text.get(..end).unwrap_or(text),
        text.get(end..).unwrap_or_default(),
    )
}

fn normalize_authority(authority: &str) -> String {
    let at = authority.rfind('@').map_or(0, |index| index + 1);
    let user_information = authority.get(..at).unwrap_or_default();
    let (host, port) = host_and_port(authority.get(at..).unwrap_or_default());
    let lowered = host.to_lowercase();
    let host = lowered.strip_suffix('.').unwrap_or(&lowered);
    format!("{user_information}{host}{port}")
}

/// The text with its scheme and host normalized, and its path; text that is no address is
/// returned as it is, with no path.
fn normalize(text: &str) -> (String, Option<String>) {
    let Some((scheme, rest)) = split_scheme(text) else {
        return (text.to_owned(), None);
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = rest.get(..end).unwrap_or_default();
    let path = rest.get(end..).unwrap_or_default();
    (
        format!(
            "{}://{}{path}",
            scheme.to_lowercase(),
            normalize_authority(authority)
        ),
        Some(path.to_owned()),
    )
}

/// The spellings of an address to try: the root path is written with or without its slash.
fn spellings(text: &str) -> Vec<String> {
    let (normalized, path) = normalize(text);
    match path.as_deref() {
        Some("") => vec![normalized.clone(), format!("{normalized}/")],
        Some("/") => vec![
            normalized.clone(),
            normalized
                .strip_suffix('/')
                .unwrap_or(&normalized)
                .to_owned(),
        ],
        Some(_) | None => vec![normalized],
    }
}

/// `scheme://[user@]host`, then a port that is a lone `*`, then the rest of the pattern
/// (`/^([a-z][a-z0-9+.-]*:\/\/(?:[^/?#@]*@)?(?:\[[^\]]*\]|[^/?#:]+)):\*([\s\S]*)$/`, case
/// sensitive): the origin and the rest.
fn port_wildcard(pattern: &str) -> Option<(String, String)> {
    let (scheme, after) = split_scheme(pattern)?;
    if scheme
        .chars()
        .any(|character| character.is_ascii_uppercase())
    {
        return None;
    }
    let with_user_information = after
        .find(['/', '?', '#', '@'])
        .filter(|index| {
            after
                .get(*index..)
                .is_some_and(|rest| rest.starts_with('@'))
        })
        .map(|index| index + 1);
    [with_user_information, Some(0)]
        .into_iter()
        .flatten()
        .find_map(|start| host_then_wildcard(after, start))
        .map(|(host_end, rest)| {
            (
                format!("{scheme}://{}", after.get(..host_end).unwrap_or_default()),
                rest.to_owned(),
            )
        })
}

/// The end of a host that starts at `start` and is followed by `:*`, and what follows that.
fn host_then_wildcard(after: &str, start: usize) -> Option<(usize, &str)> {
    let tail = after.get(start..)?;
    let bracketed = tail
        .starts_with('[')
        .then(|| tail.find(']').map(|close| close + 1))
        .flatten();
    let plain = tail.find(['/', '?', '#', ':']).unwrap_or(tail.len());
    [bracketed, (plain > 0).then_some(plain)]
        .into_iter()
        .flatten()
        .find_map(|length| {
            tail.get(length..)?
                .strip_prefix(":*")
                .map(|rest| (start + length, rest))
        })
}

/// The patterns that stand for the host with no port: the `*` of the port then stood for a path.
fn without_port(pattern: &str) -> Vec<String> {
    match port_wildcard(pattern) {
        Some((origin, rest)) => vec![format!("{origin}{rest}"), format!("{origin}/*{rest}")],
        None => Vec::new(),
    }
}

/// Whether the address matches the pattern of `allowedHttpHookUrls`.
#[must_use]
pub fn match_url_pattern(pattern: &str, url: &str) -> bool {
    let (wanted, _) = normalize(pattern);
    let addresses = spellings(url);
    let mut candidates = vec![wanted.clone()];
    candidates.extend(without_port(&wanted));
    candidates.iter().any(|each| {
        addresses
            .iter()
            .any(|address| matches_wildcard(each, address))
    })
}

crate::registry::exports! {
    "settings/hooks/url-pattern" {
        "matchUrlPattern" => ok |pattern: String, url: String| match_url_pattern(&pattern, &url);
    }
}
