//! Port of `src/marketplace/marketplace-download.ts`, switched.
//!
//! Running the `http-file` plan of a `url` marketplace source (plugins/host-marketplace.md, "Stay
//! within the download limits for hosted files"): the plan's 5 MiB and 10 seconds, and a redirect to
//! a different origin "must use `https://` and can't point at a loopback, link-local, or
//! cloud-metadata host", its request carrying none of the configured headers. The docs give no
//! redirect count for this file: the archive's five holds, and a same-origin redirect is followed
//! unjudged (judgment, spec/judgments.md). `readMarketplaceText` reads what comes back.

use serde::{Deserialize, Serialize};

use super::http_download::{Download, download_following, origin_of};
use super::types::HeaderMap;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::blocked_hosts::is_blocked_host;
use crate::js::number_to_string;
use crate::js::url::parse;
use crate::ports::Signal;
use crate::ports::http_client::BodyAs;
use crate::registry::Port;

/// The `http-file` plan (`Extract<MarketplaceFetchPlan, { kind: 'http-file' }>`), as far as the download reads it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpFilePlan {
    /// The marketplace file's address.
    pub url: String,
    /// The most bytes of body.
    pub max_bytes: u64,
    /// How long a request may take.
    pub timeout_ms: u64,
    /// The configured headers.
    pub headers: HeaderMap,
}

const MAX_REDIRECTS: u32 = 5;

/// Why a redirect from `from` to `target` is refused: only one that leaves the origin is judged.
fn refuse_cross_origin(target: &str, from: &str) -> Option<String> {
    if origin_of(target) == origin_of(from) {
        return None;
    }
    let refused = |why: &str| format!("The redirect to \"{target}\" is refused: {why}");
    let https = target
        .as_bytes()
        .get(..8)
        .is_some_and(|start| start.eq_ignore_ascii_case(b"https://"));
    if !https {
        return Some(refused("another origin must use https://"));
    }
    parse(target)
        .is_some_and(|url| is_blocked_host(&url.hostname))
        .then(|| refused("it points at a loopback, link-local or cloud-metadata host"))
}

/// The ports the download uses.
#[derive(Debug, Deserialize)]
pub struct Ports {
    /// The host's HTTP client.
    pub http: Port,
}

/// What the host resolved besides the plan.
#[derive(Debug, Deserialize)]
pub struct FetchOptions {
    /// Cancels the download.
    pub signal: Signal,
    /// The headers the host resolved for the source (`resolveHeaders` over `urlSourceLayer`, the
    /// command's output included); absent, the plan's own `headers`.
    #[serde(default)]
    pub headers: Option<HeaderMap>,
}

/// The marketplace file's text, or the issue that refuses it.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Fetched {
    /// The text.
    Text {
        /// The file.
        text: String,
    },
    /// Why not.
    Issue(PluginIssue),
}

/// Downloads the marketplace file: its text, or the issue that refuses it. `headers` are the ones the
/// host resolved for the source (`resolveHeaders` over `urlSourceLayer`, the command's output
/// included); absent, the plan's own `headers`.
pub async fn fetch_marketplace_file(
    plan: &HttpFilePlan,
    ports: &Ports,
    options: FetchOptions,
) -> Fetched {
    let download = Download {
        url: plan.url.clone(),
        headers: options.headers.unwrap_or_else(|| plan.headers.clone()),
        body_as: BodyAs::Text,
        timeout_ms: plan.timeout_ms,
        max_bytes: plan.max_bytes,
        max_redirects: MAX_REDIRECTS,
        refuse_hop: refuse_cross_origin,
        path: "url".to_owned(),
    };
    let response = match download_following(&download, &ports.http, &options.signal).await {
        Ok(response) => response,
        Err(issue) => return Fetched::Issue(issue),
    };
    if response.truncated {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the plan's limit is 5 MiB, far below 2^53, as JavaScript's number holds it"
        )]
        let mebibytes = plan.max_bytes as f64 / (1024.0 * 1024.0);
        return Fetched::Issue(error(
            "url",
            &format!(
                "The marketplace file is larger than {} MiB",
                number_to_string(mebibytes)
            ),
        ));
    }
    Fetched::Text {
        text: response.body,
    }
}

crate::registry::exports! {
    "marketplace/marketplace-download" {
        "fetchMarketplaceFile" => async ok |plan: HttpFilePlan, ports: Ports, options: FetchOptions|
            fetch_marketplace_file(&plan, &ports, options).await;
    }
}
