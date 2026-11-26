use std::sync::Mutex;

use indexmap::IndexMap;

use super::*;
use crate::ports::{PortFailure, now};

/// A client that answers each request from its list in order and keeps what it was asked.
struct Scripted {
    answers: Mutex<Vec<Result<HttpResponse, PortFailure>>>,
    asked: Mutex<Vec<(String, HeaderMap)>>,
}

impl Scripted {
    fn new(mut answers: Vec<Result<HttpResponse, PortFailure>>) -> Self {
        answers.reverse();
        Self {
            answers: Mutex::new(answers),
            asked: Mutex::new(Vec::new()),
        }
    }

    fn asked(&self) -> Vec<(String, HeaderMap)> {
        self.asked
            .lock()
            .map(|asked| asked.clone())
            .unwrap_or_default()
    }
}

impl HttpClient for Scripted {
    fn request(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, PortFailure>> {
        if let Ok(mut asked) = self.asked.lock() {
            asked.push((request.url, request.headers));
        }
        let answer = self
            .answers
            .lock()
            .ok()
            .and_then(|mut answers| answers.pop())
            .unwrap_or_else(|| Err(failure("connection")));
        Box::pin(std::future::ready(answer))
    }
}

fn failure(kind: &str) -> PortFailure {
    PortFailure {
        failure: kind.to_owned(),
        message: String::new(),
    }
}

fn answer(status: u16, location: Option<&str>) -> HttpResponse {
    let mut headers = IndexMap::new();
    if let Some(location) = location {
        headers.insert("location".to_owned(), location.to_owned());
    }
    HttpResponse {
        status,
        headers,
        body: "ok".to_owned(),
        bytes: None,
        truncated: false,
    }
}

fn download(
    refuse_hop: fn(&str, &str) -> Option<String>,
) -> Download<fn(&str, &str) -> Option<String>> {
    let mut headers = HeaderMap::new();
    headers.insert("authorization".to_owned(), "Bearer t".to_owned());
    Download {
        url: "https://a.test/m.json".to_owned(),
        headers,
        body_as: BodyAs::Text,
        timeout_ms: 10_000,
        max_bytes: 100,
        max_redirects: 2,
        refuse_hop,
        path: "source.url".to_owned(),
    }
}

const fn follow(_: &str, _: &str) -> Option<String> {
    None
}

fn run(
    download: &Download<fn(&str, &str) -> Option<String>>,
    http: &Scripted,
) -> Result<HttpResponse, PluginIssue> {
    now(download_following(download, http, &Signal::new()))
        .unwrap_or_else(|| Err(error("pending", "pending")))
}

fn message(result: Result<HttpResponse, PluginIssue>) -> String {
    result.err().map(|issue| issue.message).unwrap_or_default()
}

#[test]
fn a_relative_redirect_is_followed_and_another_origin_gets_no_headers() {
    let http = Scripted::new(vec![
        Ok(answer(302, Some("/v2/m.json"))),
        Ok(answer(301, Some("//cdn.test/m.json"))),
        Ok(answer(200, None)),
    ]);
    let got = run(&download(follow), &http);
    assert_eq!(got.map(|response| response.status).ok(), Some(200));
    let asked = http.asked();
    let urls: Vec<&str> = asked.iter().map(|(url, _)| url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://a.test/m.json",
            "https://a.test/v2/m.json",
            "https://cdn.test/m.json"
        ]
    );
    let sent: Vec<usize> = asked.iter().map(|(_, headers)| headers.len()).collect();
    assert_eq!(sent, [1, 1, 0]);
}

#[test]
fn redirects_past_the_limit_are_refused() {
    let http = Scripted::new(vec![
        Ok(answer(302, Some("/1"))),
        Ok(answer(302, Some("/2"))),
        Ok(answer(302, Some("/3"))),
    ]);
    assert_eq!(
        message(run(&download(follow), &http)),
        "The download was redirected more than 2 times"
    );
}

#[test]
fn a_refused_hop_and_a_bad_target_are_issues() {
    let refuse: fn(&str, &str) -> Option<String> = |target, _| Some(format!("no {target}"));
    let http = Scripted::new(vec![Ok(answer(307, Some("https://b.test/")))]);
    assert_eq!(message(run(&download(refuse), &http)), "no https://b.test/");
    let http = Scripted::new(vec![Ok(answer(307, Some("https://[::1/")))]);
    assert_eq!(
        message(run(&download(follow), &http)),
        "The redirect target \"https://[::1/\" is not a URL"
    );
    let http = Scripted::new(vec![Ok(answer(303, None))]);
    assert_eq!(
        message(run(&download(follow), &http)),
        "The server redirected without a location"
    );
}

#[test]
fn a_failure_and_an_error_status_are_issues() {
    let http = Scripted::new(vec![Err(failure("timeout"))]);
    assert_eq!(
        message(run(&download(follow), &http)),
        "The server did not respond in time"
    );
    let http = Scripted::new(vec![Ok(answer(404, None))]);
    assert_eq!(
        message(run(&download(follow), &http)),
        "The server answered 404"
    );
}

#[test]
fn an_origin_is_read_as_new_url_reads_it() {
    assert_eq!(
        origin_of("https://A.test:443/x").as_deref(),
        Some("https://a.test")
    );
    assert_eq!(origin_of("not a url"), None);
}
