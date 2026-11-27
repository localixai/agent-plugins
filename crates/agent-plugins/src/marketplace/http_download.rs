//! Ported from `src/marketplace/http-download.ts`; only the core uses it now, so it has no wrapper.
//!
//! One download through `HttpClient` with its redirects followed by hand, so each hop is judged
//! (plugins/host-marketplace.md, "Stay within the download limits for hosted files"): at most
//! `maxRedirects` hops, each target passed to `refuseHop`, and a request sent to another origin
//! carries none of the configured headers, nor does any request after it. The one loop of the
//! `archive` zip and the `url` marketplace file; what a body must be is the caller's.

use super::types::HeaderMap;
use crate::format::issues::{PluginIssue, error};
use crate::js::url::{parse, parse_with_base};
use crate::ports::http_client::{
    BodyAs, HttpClient, HttpMethod, HttpRequest, HttpResponse, Redirect,
};
use crate::ports::{BoxFuture, Signal};

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

#[cfg(test)]
mod tests;
