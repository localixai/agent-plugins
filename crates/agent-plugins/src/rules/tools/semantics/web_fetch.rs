//! Port of `src/rules/tools/semantics/web-fetch.ts`, switched.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::js::{number_to_string, slice_index, url};

/// errors.md, "WebFetch cannot fetch localhost": the message of a refused URL, verbatim.
pub const LOCAL_HOST_MESSAGE: &str = "WebFetch cannot fetch localhost or other hostnames without a dot. To reach a local server, use Bash with curl instead.";

/// "WebFetch reads up to 100,000 characters of a page's content per call."
pub const FETCH_WINDOW_CHARACTERS: f64 = 100_000.0;

/// What WebFetch makes of a URL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckedUrl {
    /// The URL to fetch: `https` for an `http` one; `undefined` when it is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// An `http` URL was changed to `https`.
    pub upgraded: bool,
    /// Why it was refused.
    pub issues: Vec<PluginIssue>,
}

const URL_FIELD: &str = "tool_input.url";

fn refused(message: &str) -> CheckedUrl {
    CheckedUrl {
        url: None,
        upgraded: false,
        issues: vec![error(URL_FIELD, message)],
    }
}

/// A root-label dot does not make a name dotted: `localhost.` is `localhost` (spec/judgments.md).
fn has_dot(hostname: &str) -> bool {
    hostname.strip_suffix('.').unwrap_or(hostname).contains('.')
}

/// `url.protocol = 'https:'` on an `http:` URL, then `url.href`: the setter keeps every other part
/// and drops a port that is the new scheme's default (`:443`), which reading the text again with
/// the new scheme does too.
fn upgraded_href(href: &str) -> String {
    let swapped = format!("https:{}", href.strip_prefix("http:").unwrap_or(href));
    url::parse(&swapped).map_or(swapped, |parsed| parsed.href)
}

/// What WebFetch does with a URL before it makes a request (tools-reference.md, "WebFetch tool
/// behavior"): it refuses `localhost` and any other hostname without a dot, with the message of
/// errors.md, and upgrades an `http` URL to `https`. Text that does not parse, or a scheme other
/// than http and https, is an "Invalid URL" issue (spec/judgments.md: errors.md names that message
/// for the refusals before v2.1.268, the docs say no more).
#[must_use]
pub fn check_web_fetch_url(text: &str) -> CheckedUrl {
    let Some(parsed) =
        url::parse(text).filter(|parsed| parsed.protocol == "http:" || parsed.protocol == "https:")
    else {
        return refused("Invalid URL");
    };
    if !has_dot(&parsed.hostname) {
        return refused(LOCAL_HOST_MESSAGE);
    }
    let upgraded = parsed.protocol == "http:";
    CheckedUrl {
        url: Some(if upgraded {
            upgraded_href(&parsed.href)
        } else {
            parsed.href
        }),
        upgraded,
        issues: Vec::new(),
    }
}

/// The part of a page one call reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchedPart {
    /// Up to 100,000 characters of the page, from the offset.
    pub text: String,
    /// Characters of the page after this part.
    pub unread: f64,
    /// The `offset` that reads the next part; `undefined` when nothing is unread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<f64>,
    /// The note that tells Claude how much went unread; `undefined` when nothing did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

/// The part of a page WebFetch reads (tools-reference.md, "WebFetch tool behavior"; hooks.md,
/// `offset`: "number of characters to skip from the start of the page"): up to 100,000 characters
/// from `offset`, and how much went unread. Characters are code points, so a part never splits one;
/// an offset past the end reads nothing; an offset that is not a whole number counts as absent; the
/// wording of the notice is a judgment (spec/judgments.md).
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "a page's length in code points is far below 2^53, as a JavaScript array's is"
)]
pub fn slice_fetched(page: &str, offset: Option<f64>) -> FetchedPart {
    let characters: Vec<char> = page.chars().collect();
    let length = characters.len() as f64;
    let start = offset
        .filter(|offset| offset.is_finite() && offset.fract() == 0.0 && *offset >= 0.0)
        .unwrap_or(0.0);
    let end = (start + FETCH_WINDOW_CHARACTERS).min(length);
    let (from, to) = (
        slice_index(start, characters.len()),
        slice_index(end, characters.len()),
    );
    let text: String = characters
        .get(from..to.max(from))
        .unwrap_or_default()
        .iter()
        .collect();
    let unread = (length - end).max(0.0);
    if unread == 0.0 {
        return FetchedPart {
            text,
            unread,
            next_offset: None,
            notice: None,
        };
    }
    FetchedPart {
        text,
        unread,
        next_offset: Some(end),
        notice: Some(format!(
            "{} characters of the page were not read; fetch again with offset {} to continue.",
            number_to_string(unread),
            number_to_string(end)
        )),
    }
}

crate::registry::exports! {
    "rules/tools/semantics/web-fetch" {
        "LOCAL_HOST_MESSAGE" => ok || LOCAL_HOST_MESSAGE;
        "FETCH_WINDOW_CHARACTERS" => ok || FETCH_WINDOW_CHARACTERS;
        "checkWebFetchUrl" => ok |text: String| check_web_fetch_url(&text);
        "sliceFetched" => ok |page: String, offset: Option<f64>| slice_fetched(&page, offset);
    }
}
