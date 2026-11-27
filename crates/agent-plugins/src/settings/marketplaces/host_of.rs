//! Port of `src/settings/marketplaces/host-of.ts`, switched.
//!
//! The host a `hostPattern` entry is matched against (settings-reference.md,
//! `strictKnownMarketplaces` "Allowed source types", `hostPattern`): the hostname of a `url`
//! source, and of a `git` source depending on the form of the git URL.

use super::git_url::is_scheme_url;
use crate::js::is_whitespace;
use crate::js::url::parse;

/// A character neither a user nor a host part holds: `@`, `/` or a blank.
fn breaks_user(character: char) -> bool {
    matches!(character, '@' | '/') || is_whitespace(character)
}

/// A character a host part does not hold: `@`, `:`, `/` or a blank.
fn breaks_host(character: char) -> bool {
    character == ':' || breaks_user(character)
}

/// `[^@:/\s]+` from the start of `text`: the part and what follows it; `None` when empty.
fn host_part(text: &str) -> Option<(&str, &str)> {
    let end = text.find(breaks_host).unwrap_or(text.len());
    let (part, rest) = text.split_at(end);
    (!part.is_empty()).then_some((part, rest))
}

/// `user@host:path`, the SSH address without a scheme: "the host between `@` and `:`":
/// `^[^@:/\s]+@([^@:/\s]+):`.
fn user_at_host(address: &str) -> Option<&str> {
    let (_, rest) = host_part(address)?;
    let (host, rest) = host_part(rest.strip_prefix('@')?)?;
    rest.starts_with(':').then_some(host)
}

/// `([^@:/\s]+)[:/]` from the start of `text`.
fn host_then_separator(text: &str) -> Option<&str> {
    let (host, rest) = host_part(text)?;
    (rest.starts_with(':') || rest.starts_with('/')).then_some(host)
}

/// Wider forms the blocklist reads (judgment: the docs say only that "a host from a wider set of
/// forms" is taken): `host:path`, `host/path` and `user@host/path`:
/// `^(?:[^@/\s]+@)?([^@:/\s]+)[:/]`.
fn schemeless(address: &str) -> Option<&str> {
    let end = address.find(breaks_user).unwrap_or(address.len());
    let with_user = (end > 0)
        .then(|| address.get(end..))
        .flatten()
        .and_then(|rest| rest.strip_prefix('@'))
        .and_then(host_then_separator);
    with_user.or_else(|| host_then_separator(address))
}

/// A path on this machine, which has no host: `^[/.~]`.
fn is_local_path(address: &str) -> bool {
    address.starts_with(['/', '.', '~'])
}

/// One letter before `:` is a Windows drive, which git also reads as a path: `^[A-Za-z]:`.
fn is_drive(address: &str) -> bool {
    let mut characters = address.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.next() == Some(':')
}

/// The hostname of a URL with a scheme; `None` when it has none (`file:///x`) or does not parse.
fn scheme_host(text: &str) -> Option<String> {
    let hostname = parse(text)?.hostname;
    (!hostname.is_empty()).then(|| hostname.to_lowercase())
}

/// The hostname of a `url` source (judgment: lower case, as the URL parser spells it).
#[must_use]
pub fn host_of_url(address: &str) -> Option<String> {
    if is_scheme_url(address) {
        scheme_host(address)
    } else {
        None
    }
}

/// The host of a `git` source's URL. `wide` is the blocklist: it also reads an address with no
/// scheme and no `user@`. Anything else has no host (`None`).
#[must_use]
pub fn host_of_git(address: &str, wide: bool) -> Option<String> {
    if is_scheme_url(address) {
        return scheme_host(address);
    }
    if let Some(scp) = user_at_host(address) {
        return Some(scp.to_lowercase());
    }
    if !wide || is_local_path(address) || is_drive(address) {
        return None;
    }
    schemeless(address).map(str::to_lowercase)
}

crate::registry::exports! {
    "settings/marketplaces/host-of" {
        "hostOfUrl" => ok |address: String| host_of_url(&address);
        "hostOfGit" => ok |address: String, wide: bool| host_of_git(&address, wide);
    }
}

#[cfg(test)]
mod tests {
    use super::host_of_git;

    #[test]
    fn a_git_address_has_its_host_by_form() {
        assert_eq!(
            host_of_git("git@GitHub.com:o/r", false).as_deref(),
            Some("github.com")
        );
        assert_eq!(host_of_git("github.com/o/r", false), None);
        assert_eq!(
            host_of_git("github.com/o/r", true).as_deref(),
            Some("github.com")
        );
        assert_eq!(host_of_git("ab:c@host/x", true).as_deref(), Some("host"));
        assert_eq!(host_of_git("C:/x", true), None);
    }
}
