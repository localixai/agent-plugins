//! JavaScript's `new URL(text)` and `new URL(text, base)`: the members the package reads of a `URL`, which
//! are the WHATWG URL Standard's basic URL parser's answer, here the `url` crate's. A text that
//! makes `new URL` throw is `None`. `tests/differential/js.rs` holds it to the answers
//! `scripts/rust/js-oracle.ts` recorded in Bun (JavaScriptCore).
//!
//! Where the crate and JavaScriptCore differ, JavaScriptCore is right for this package (it is the
//! runtime Claude Code runs on), and each difference is one rule below:
//!
//! - **`file:` keeps its leading empty segments.** `new URL('file:////a').pathname` is `//a`;
//!   the crate follows an older standard that removed them and answers `/a`
//!   (`file_empty_segments`).
//! - **A non-special URL's empty path becomes `/` after user information when a query or a
//!   fragment follows.** `new URL('git+ssh://git@h?x').href` is `git+ssh://git@h/?x`, and
//!   without the `@` it is `git+ssh://h?x`; the crate keeps the path empty in both
//!   (`authority_holds_user_information`).
//! - **An opaque path ends in `%20`, not a space, before a query or a fragment.**
//!   `new URL('mailto:a ?x').pathname` is `a%20`; the crate leaves the space
//!   (the standard's cannot-be-a-base path state).
//! - **`pathname` of a URL with no authority drops a leading `/.`.** `new URL('foo:/.x').pathname` is
//!   `x`, and its `href` still `foo:/.x`: JavaScriptCore reads the `/.` it writes before a path
//!   that starts with `//` (`foo:/.//x`) into every path that starts with `/.`.

use serde::Serialize;
use url::Position;

/// The message of the `TypeError` JavaScriptCore throws for a text `new URL` refuses.
pub const INVALID_URL_MESSAGE: &str = "Invalid URL";

/// What `new URL(text)` gives for the members the package reads, each as JavaScript spells it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Url {
    /// `href`: the serialized URL.
    pub href: String,
    /// `origin`: scheme, host and port of a special URL, `null` (the text) for any other.
    pub origin: String,
    /// `protocol`: the scheme in lowercase and a colon (`https:`).
    pub protocol: String,
    /// `username`, percent-encoded as the standard does.
    pub username: String,
    /// `password`, percent-encoded as the standard does.
    pub password: String,
    /// `host`: `hostname`, a colon and the port when the port is not the scheme's default.
    pub host: String,
    /// `hostname`: a domain in lowercase and Punycode, an IPv4 address in dotted decimal, an IPv6
    /// address in brackets and compressed, or an opaque host of a non-special scheme.
    pub hostname: String,
    /// `port`: the digits, empty when absent or the scheme's default.
    pub port: String,
    /// `pathname`: the path, or an opaque path's text.
    pub pathname: String,
    /// `search`: the query with its `?`, empty when the query is absent or empty.
    pub search: String,
    /// `hash`: the fragment with its `#`, empty when the fragment is absent or empty.
    pub hash: String,
}

/// `prefix` and `text` when `text` is present and not empty, else nothing: how `search` and
/// `hash` spell a part.
fn with_prefix(prefix: char, text: Option<&str>) -> String {
    match text {
        Some(text) if !text.is_empty() => format!("{prefix}{text}"),
        _ => String::new(),
    }
}

/// The text the parser reads: no leading or trailing C0 control or space, no tab or newline.
fn cleaned(text: &str) -> String {
    text.trim_matches(|character: char| character <= ' ')
        .chars()
        .filter(|character| !matches!(character, '\t' | '\n' | '\r'))
        .collect()
}

/// Whether the authority of a non-special URL (`scheme://authority/…`) holds an `@`, even with
/// nothing before it, which the parsed URL no longer shows.
fn authority_holds_user_information(cleaned: &str) -> bool {
    let after_scheme = cleaned.split_once(':').map_or("", |(_, rest)| rest);
    after_scheme.strip_prefix("//").is_some_and(|authority| {
        authority
            .split(['/', '?', '#'])
            .next()
            .is_some_and(|authority| authority.contains('@'))
    })
}

/// The path of a `file:` URL as JavaScriptCore keeps it: the crate's path with the empty segments
/// it removed from the front put back. `cleaned` is the text, `path` the crate's answer.
fn file_empty_segments(cleaned: &str, path: &str) -> String {
    let rest = cleaned.get("file:".len()..).unwrap_or_default();
    let rest = rest
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .replace('\\', "/");
    // After `//` comes the host, up to the slash that starts the path.
    let path_text = match rest.strip_prefix("//") {
        Some(after) => after
            .find('/')
            .and_then(|at| after.get(at..))
            .unwrap_or_default(),
        None => rest.as_str(),
    };
    let segments = path_segments(path_text.strip_prefix('/').unwrap_or(path_text));
    let leading = segments
        .iter()
        .take_while(|segment| segment.is_empty())
        .count();
    if leading == segments.len() {
        "/".repeat(leading.max(1))
    } else {
        format!("{}{path}", "/".repeat(leading))
    }
}

/// The segments of a path as the standard's path state builds them, where only whether a segment
/// is empty matters: `.` is dropped, `..` removes the segment before it, and either one at the end
/// leaves an empty segment.
fn path_segments(path: &str) -> Vec<&str> {
    let mut segments: Vec<&str> = Vec::new();
    let mut parts = path.split('/').peekable();
    while let Some(part) = parts.next() {
        let last = parts.peek().is_none();
        match part.to_ascii_lowercase().as_str() {
            "." | "%2e" => {}
            ".." | ".%2e" | "%2e." | "%2e%2e" => {
                segments.pop();
            }
            _ => {
                segments.push(part);
                continue;
            }
        }
        if last {
            segments.push("");
        }
    }
    segments
}

/// Parses `text` as `new URL(text)` does, or `None` where it throws.
#[must_use]
pub fn parse(text: &str) -> Option<Url> {
    let url = ::url::Url::parse(text).ok()?;
    let cleaned = cleaned(text);
    let hostname = url.host_str().unwrap_or_default().to_owned();
    let port = url.port().map(|port| port.to_string()).unwrap_or_default();
    let host = if port.is_empty() {
        hostname.clone()
    } else {
        format!("{hostname}:{port}")
    };
    let mut path = url.path().to_owned();
    if url.scheme() == "file" {
        path = file_empty_segments(&cleaned, &path);
    } else if path.is_empty()
        && !hostname.is_empty()
        && (url.query().is_some() || url.fragment().is_some())
        && authority_holds_user_information(&cleaned)
    {
        "/".clone_into(&mut path);
    }
    if url.cannot_be_a_base()
        && path.ends_with(' ')
        && (url.query().is_some() || url.fragment().is_some())
    {
        path.pop();
        path.push_str("%20");
    }
    let href = format!(
        "{}{path}{}",
        &url[..Position::BeforePath],
        &url[Position::AfterPath..]
    );
    // `pathname` of a URL with no authority: `foo:/.x` reads as `x`.
    let pathname = if !url.has_authority() && !url.cannot_be_a_base() {
        path.strip_prefix("/.").unwrap_or(&path).to_owned()
    } else {
        path
    };
    Some(Url {
        href,
        origin: url.origin().ascii_serialization(),
        protocol: format!("{}:", url.scheme()),
        username: url.username().to_owned(),
        password: url.password().unwrap_or_default().to_owned(),
        host,
        hostname,
        port,
        pathname,
        search: with_prefix('?', url.query()),
        hash: with_prefix('#', url.fragment()),
    })
}

/// Whether `text` opens with a scheme (`[A-Za-z][A-Za-z0-9+.-]*:`), so it is not read against
/// the base's path.
fn opens_with_scheme(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters
            .find(|character| !(character.is_ascii_alphanumeric() || "+.-".contains(*character)))
            == Some(':')
}

/// Whether `text` opens with two of the separators a special URL reads, `/` and `\`.
fn opens_with_two_separators(text: &str) -> bool {
    let mut characters = text.chars();
    matches!(
        (characters.next(), characters.next()),
        (Some('/' | '\\'), Some('/' | '\\'))
    )
}

/// Whether `text` is a rooted path whose first segment is a Windows drive letter (`/C:` or `\C|`
/// and then the end, a separator, a query or a fragment).
fn rooted_drive_letter(text: &str) -> bool {
    let mut characters = text.chars();
    matches!(
        (characters.next(), characters.next(), characters.next(), characters.next()),
        (Some('/' | '\\'), Some(letter), Some(':' | '|'), None | Some('/' | '\\' | '?' | '#'))
            if letter.is_ascii_alphabetic()
    )
}

/// Parses `text` against `base` as `new URL(text, base)` does, or `None` where it throws: the base
/// read as `new URL(base)` reads it, `text` resolved against it by the crate, and the result read
/// again so the rules above apply to what it resolved to. Where the crate's `join` follows an older
/// standard, the text is first written as the one the standard resolves it to:
///
/// - **Against a special base, two separators start the host**, and any more are skipped
///   (the special authority ignore slashes state): `///h/x` is `<scheme>://h/x`, where the crate
///   refuses it; such a text is read as `<scheme>:<text>`.
/// - **Against a `file:` base with a host, a rooted drive letter keeps the host**: `/C:/x` against
///   `file://h/a` is `file://h/C:/x` (the file slash state); the crate drops the host.
/// - **Against a non-special base, `\` is no separator**: `\a` against `foo://h/p/q` is
///   `foo://h/p/\a` and `/\a` is `foo://h/\a`, where the crate reads a leading `\` as `/`; the
///   first is read as `./<text>`, the second after the base's scheme and authority.
#[must_use]
pub fn parse_with_base(text: &str, base: &str) -> Option<Url> {
    let base = ::url::Url::parse(&parse(base)?.href).ok()?;
    let text = cleaned(text);
    if opens_with_scheme(&text) {
        return parse(base.join(&text).ok()?.as_str());
    }
    let special = base.is_special();
    if special && opens_with_two_separators(&text) {
        return parse(&format!("{}:{text}", base.scheme()));
    }
    if let Some(host) = base
        .host_str()
        .filter(|host| base.scheme() == "file" && !host.is_empty() && rooted_drive_letter(&text))
    {
        return parse(&format!("file://{host}{text}"));
    }
    // An opaque path (`mailto:x`) takes only a fragment, which the crate answers.
    let non_special_path = !special && !base.cannot_be_a_base();
    if non_special_path && text.starts_with('\\') {
        return parse(base.join(&format!("./{text}")).ok()?.as_str());
    }
    if non_special_path && text.starts_with("/\\") {
        return parse(&format!("{}{text}", &base[..Position::BeforePath]));
    }
    parse(base.join(&text).ok()?.as_str())
}
