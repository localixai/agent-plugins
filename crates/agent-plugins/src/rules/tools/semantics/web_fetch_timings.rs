//! Port of `src/rules/tools/semantics/web-fetch-timings.ts`, switched.
//!
//! WebFetch's cache time, its download deadline and its notice of a redirect to another host
//! (tools-reference.md, "WebFetch tool behavior").

use serde::Serialize;

use super::env_number::{positive_integer, whole_number};
use crate::js::url;
use crate::ports::environment::Environment;
use crate::registry::Port;

/// tools-reference.md, "WebFetch tool behavior": "caches each response for 15 minutes by default".
const DEFAULT_CACHE_TTL_MS: f64 = 900_000.0;
/// "A page that hasn't finished downloading within five minutes ... fails with a deadline error."
const DEFAULT_DEADLINE_MS: f64 = 300_000.0;

/// WebFetch's timings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchTimings {
    /// How long a fetched URL's response stays cached.
    pub cache_ttl_ms: f64,
    /// How long a download may take, redirects included; `None` when `0` removed the limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<f64>,
}

/// The cache time and the download deadline of WebFetch (tools-reference.md, "WebFetch tool
/// behavior"; env-vars.md). `CLAUDE_CODE_WEBFETCH_CACHE_TTL_MS` "takes plain digits only; `0`, a
/// decimal, or any other spelling keeps the default". `CLAUDE_CODE_WEBFETCH_DEADLINE_MS` takes
/// plain digits, `0` removes the limit, and a decimal or any other spelling keeps the default.
pub async fn web_fetch_timings(env: &dyn Environment) -> FetchTimings {
    let deadline = whole_number(env, "CLAUDE_CODE_WEBFETCH_DEADLINE_MS")
        .await
        .unwrap_or(DEFAULT_DEADLINE_MS);
    FetchTimings {
        cache_ttl_ms: positive_integer(env, "CLAUDE_CODE_WEBFETCH_CACHE_TTL_MS")
            .await
            .unwrap_or(DEFAULT_CACHE_TTL_MS),
        deadline_ms: (deadline != 0.0).then_some(deadline),
    }
}

fn host_of(text: &str) -> Option<String> {
    url::parse(text).map(|parsed| parsed.hostname.to_lowercase())
}

/// "When a URL redirects to a different host, WebFetch returns a text result that names the
/// original URL and the redirect target instead of following it" (tools-reference.md). `None` when
/// the target is on the same host, which is followed. Hosts compare by hostname, ignoring case,
/// port and scheme, and the wording is a judgment (spec/judgments.md).
#[must_use]
pub fn redirect_notice(original: &str, target: &str) -> Option<String> {
    let from = host_of(original)?;
    let to = host_of(target)?;
    (from != to).then(|| {
        format!(
            "The URL {original} redirects to a different host: {target}. Fetch the new URL with a second WebFetch call."
        )
    })
}

crate::registry::exports! {
    "rules/tools/semantics/web-fetch-timings" {
        "webFetchTimings" => async ok |env: Port| web_fetch_timings(&env).await;
        "redirectNotice" => ok |original: String, target: String| redirect_notice(&original, &target);
    }
}
