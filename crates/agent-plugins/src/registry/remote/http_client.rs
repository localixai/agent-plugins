//! [`Port`] as the host's HTTP client (`src/ports/http-client.ts`): `HttpResponse | HttpFailure`,
//! read as a `Result`, the body's bytes as the wire tags them.

use indexmap::IndexMap;
use serde::Deserialize;

use super::{Port, arg};
use crate::ports::http_client::{HttpClient, HttpRequest, HttpResponse};
use crate::ports::{BoxFuture, PortFailure};
use crate::registry::wire::Bytes;

/// An `HttpResponse` as it crosses: its bytes `{"$bytes": base64}`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    status: u16,
    headers: IndexMap<String, String>,
    body: String,
    #[serde(default)]
    bytes: Option<Bytes>,
    truncated: bool,
}

impl From<Response> for HttpResponse {
    fn from(response: Response) -> Self {
        Self {
            status: response.status,
            headers: response.headers,
            body: response.body,
            bytes: response.bytes.map(|Bytes(bytes)| bytes),
            truncated: response.truncated,
        }
    }
}

impl HttpClient for Port {
    fn request(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, PortFailure>> {
        let asked = self.ask_failable::<Response>("request", vec![arg(Some(&request))]);
        Box::pin(async move { asked.await.map(HttpResponse::from) })
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::super::testing::{exchange, json, need, port};
    use crate::ports::Signal;
    use crate::ports::http_client::{BodyAs, HttpClient as _, HttpMethod, HttpRequest, Redirect};

    fn request() -> HttpRequest {
        HttpRequest {
            method: HttpMethod::Get,
            url: "https://a.test/".to_owned(),
            headers: IndexMap::new(),
            body: None,
            body_as: BodyAs::Bytes,
            timeout_ms: None,
            max_bytes: Some(10),
            redirect: Redirect::Manual,
            signal: Signal::new(),
        }
    }

    #[test]
    fn a_request_crosses_and_its_response_is_read_back() {
        let response = json(
            r#"{"status": 200, "headers": {}, "body": "", "bytes": {"$bytes": "AQI="},
            "truncated": false}"#,
        );
        let (asked, read) = exchange(async { port().request(request()).await }, response);
        let sent = json(
            r#"{"method": "GET", "url": "https://a.test/", "headers": {}, "as": "bytes",
            "maxBytes": 10, "redirect": "manual", "signal": {"$signal": null, "aborted": false}}"#,
        );
        assert_eq!(asked, need("request", vec![sent]));
        assert_eq!(
            read,
            "Ok(HttpResponse { status: 200, headers: {}, body: \"\", bytes: Some([1, 2]), truncated: false })"
        );
    }

    #[test]
    fn a_failure_is_an_err() {
        let failure = json(r#"{"failure": "timeout", "message": "after 5s"}"#);
        let (_, read) = exchange(async { port().request(request()).await }, failure);
        assert_eq!(
            read,
            r#"Err(PortFailure { failure: "timeout", message: "after 5s" })"#
        );
    }
}
