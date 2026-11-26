//! Port of `src/marketplace/http-download.ts` at sha256:29d9e6aa147d.
//!
//! One download through `HttpClient` with its redirects followed by hand, so each hop is judged
//! (plugins/host-marketplace.md, "Stay within the download limits for hosted files"): at most
//! `maxRedirects` hops, each target passed to `refuseHop`, and a request sent to another origin
//! carries none of the configured headers, nor does any request after it. The one loop of the
//! `archive` zip and the `url` marketplace file; what a body must be is the caller's.
//!
//! Not switched: the `archive` zip's body (up to 256 MiB, `fetch-plan.ts`) would cross into the
//! core and back again while `archive-source.ts`, which reads it, is TypeScript (docs/RUST.md,
//! "What stays TypeScript"); it switches with that module, and the bytes then cross once.

use serde::Serialize;

use super::types::HeaderMap;
use crate::format::issues::{PluginIssue, error};
use crate::js::url::{parse, parse_with_base};
use crate::ports::http_client::{
    BodyAs, HttpClient, HttpMethod, HttpRequest, HttpResponse, Redirect,
};
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Bytes, Callback, Port};

/// The reason a redirect from `from` to `target` is refused, or `None` to follow it: a function of
/// the caller's, so its answer may wait. A Rust caller passes a closure, answered at once.
pub trait RefuseHop: Send + Sync {
    /// Its answer for one hop.
    fn refuse<'a>(&'a self, target: &'a str, from: &'a str) -> BoxFuture<'a, Option<String>>;
}

impl<F: Fn(&str, &str) -> Option<String> + Send + Sync> RefuseHop for F {
    fn refuse<'a>(&'a self, target: &'a str, from: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(std::future::ready(self(target, from)))
    }
}

impl RefuseHop for Callback {
    fn refuse<'a>(&'a self, target: &'a str, from: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(self.call::<Option<String>>(vec![target.into(), from.into()]))
    }
}

/// One download.
#[derive(Clone, Debug)]
pub struct Download<R> {
    /// Where it starts.
    pub url: String,
    /// The configured headers, sent only while the requests stay on the first origin.
    pub headers: HeaderMap,
    /// How the body comes back.
    pub body_as: BodyAs,
    /// After how long a request gives up.
    pub timeout_ms: u64,
    /// At most this many bytes of body.
    pub max_bytes: u64,
    /// At most this many redirects.
    pub max_redirects: u32,
    /// The reason a redirect from `from` to `target` is refused, or `None` to follow it.
    pub refuse_hop: R,
    /// The issue path of every refusal.
    pub path: String,
}

/// The statuses that redirect.
const REDIRECTS: [u16; 5] = [301, 302, 303, 307, 308];

/// What each failure of the client is reported as.
const FAILURES: [(&str, &str); 3] = [
    ("connection", "The download failed to connect"),
    ("timeout", "The server did not respond in time"),
    ("aborted", "The download was aborted"),
];

/// `new URL(url).origin`, or `None` where `new URL` throws.
#[must_use]
pub fn origin_of(url: &str) -> Option<String> {
    parse(url).map(|parsed| parsed.origin)
}

/// The next URL of a redirect, or the issue that refuses it.
async fn follow_redirect<R: RefuseHop>(
    response: &HttpResponse,
    from: &str,
    followed: u32,
    download: &Download<R>,
) -> Result<String, PluginIssue> {
    let refuse = |message: &str| error(&download.path, message);
    if followed >= download.max_redirects {
        return Err(refuse(&format!(
            "The download was redirected more than {} times",
            download.max_redirects
        )));
    }
    let Some(location) = response.headers.get("location") else {
        return Err(refuse("The server redirected without a location"));
    };
    let Some(target) = parse_with_base(location, from).map(|url| url.href) else {
        return Err(refuse(&format!(
            "The redirect target \"{location}\" is not a URL"
        )));
    };
    match download.refuse_hop.refuse(&target, from).await {
        None => Ok(target),
        Some(refused) => Err(refuse(&refused)),
    }
}

/// The final response, a 2xx that is no redirect, or the issue that ends the download.
pub async fn download_following<R: RefuseHop>(
    download: &Download<R>,
    http: &dyn HttpClient,
    signal: &Signal,
) -> Result<HttpResponse, PluginIssue> {
    let start = origin_of(&download.url);
    let mut url = download.url.clone();
    let mut send_headers = true;
    let mut followed = 0;
    loop {
        send_headers = send_headers && origin_of(&url) == start;
        let request = HttpRequest {
            method: HttpMethod::Get,
            url: url.clone(),
            headers: if send_headers {
                download.headers.clone()
            } else {
                HeaderMap::new()
            },
            body: None,
            body_as: download.body_as,
            timeout_ms: Some(download.timeout_ms),
            max_bytes: Some(download.max_bytes),
            redirect: Redirect::Manual,
            signal: signal.clone(),
        };
        let response = match http.request(request).await {
            Ok(response) => response,
            Err(failure) => {
                let message = FAILURES
                    .iter()
                    .find(|(kind, _)| *kind == failure.failure)
                    .map_or("", |(_, message)| message);
                return Err(error(&download.path, message));
            }
        };
        if !REDIRECTS.contains(&response.status) {
            return if (200..=299).contains(&response.status) {
                Ok(response)
            } else {
                Err(error(
                    &download.path,
                    &format!("The server answered {}", response.status),
                ))
            };
        }
        url = follow_redirect(&response, &url, followed, download).await?;
        followed += 1;
    }
}

/// An `HttpResponse` as it crosses back: its bytes `{"$bytes": base64}`, absent when none came.
#[derive(Serialize)]
#[serde(untagged)]
enum Downloaded {
    Response {
        status: u16,
        headers: HeaderMap,
        body: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        bytes: Option<Bytes>,
        truncated: bool,
    },
    Issue(PluginIssue),
}

impl From<Result<HttpResponse, PluginIssue>> for Downloaded {
    fn from(result: Result<HttpResponse, PluginIssue>) -> Self {
        match result {
            Ok(response) => Self::Response {
                status: response.status,
                headers: response.headers,
                body: response.body,
                bytes: response.bytes.map(Bytes),
                truncated: response.truncated,
            },
            Err(issue) => Self::Issue(issue),
        }
    }
}

/// `Download` as it crosses: a record holding a function, so a port whose members are read
/// (docs/RUST.md, "Effects").
async fn download_of(record: Port) -> Download<Callback> {
    let url = record.member("url");
    let headers = record.member("headers");
    let body_as = record.member("as");
    let timeout_ms = record.member("timeoutMs");
    let max_bytes = record.member("maxBytes");
    let max_redirects = record.member("maxRedirects");
    let refuse_hop = record.member("refuseHop");
    let path = record.member("path");
    Download {
        url: url.await,
        headers: headers.await,
        body_as: body_as.await,
        timeout_ms: timeout_ms.await,
        max_bytes: max_bytes.await,
        max_redirects: max_redirects.await,
        refuse_hop: refuse_hop.await,
        path: path.await,
    }
}

crate::registry::exports! {
    "marketplace/http-download" {
        "downloadFollowing" => async ok |download: Port, http: Port, signal: Signal|
            Downloaded::from(download_following(&download_of(download).await, &http, &signal).await);
    }
}

#[cfg(test)]
mod tests;
