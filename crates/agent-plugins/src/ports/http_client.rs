//! The core's counterpart of `src/ports/http-client.ts` at sha256:47d24c4ff515, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! HTTP requests: hook handlers of type `http`, marketplace downloads, remote MCP checks.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{BoxFuture, PortFailure, Signal};

/// The request method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    /// `GET`.
    #[serde(rename = "GET")]
    Get,
    /// `POST`.
    #[serde(rename = "POST")]
    Post,
}

/// How the body comes back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyAs {
    /// Decoded text in `body`.
    #[default]
    Text,
    /// Raw bytes in `bytes`; an archive always is.
    Bytes,
}

/// Whether redirects are followed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Redirect {
    /// Followed by the host.
    #[default]
    Follow,
    /// Answered as they are.
    Manual,
}

/// One request.
#[derive(Clone, Debug)]
pub struct HttpRequest {
    /// Its method.
    pub method: HttpMethod,
    /// Its URL.
    pub url: String,
    /// Its headers.
    pub headers: IndexMap<String, String>,
    /// Its body.
    pub body: Option<String>,
    /// How the response body comes back.
    pub body_as: BodyAs,
    /// After how long the host gives up and answers a `timeout` failure.
    pub timeout_ms: Option<u64>,
    /// At most this many bytes of body are read, counted in bytes either way; the rest is cut
    /// and the response says `truncated`.
    pub max_bytes: Option<u64>,
    /// Whether redirects are followed.
    pub redirect: Redirect,
    /// Its cancellation.
    pub signal: Signal,
}

/// A response.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HttpResponse {
    /// The status code.
    pub status: u16,
    /// The headers, names in lower case.
    pub headers: IndexMap<String, String>,
    /// The body as text (empty when it came back as bytes).
    pub body: String,
    /// The body as bytes, when asked for.
    pub bytes: Option<Vec<u8>>,
    /// Whether the body was cut at `max_bytes`.
    pub truncated: bool,
}

/// The host's HTTP client. A failure's kind is `connection`, `timeout` or `aborted`.
pub trait HttpClient: Send + Sync {
    /// Sends one request.
    fn request(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, PortFailure>>;
}
