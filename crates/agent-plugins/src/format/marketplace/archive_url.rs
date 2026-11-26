//! Port of `src/format/marketplace/archive-url.ts`, switched.
//!
//! An archive source's `url` "must use `https://` and can't point at a loopback, link-local, or
//! cloud-metadata host" (plugins/marketplace-reference.md, "archive plugin source"). The validator
//! reports a break on `source.url` with the text of spec §15.6.5 (oracle 2.1.325); a text that does
//! not parse as a URL draws `Invalid URL` first and the same report (J15.8). The scheme is read in
//! any letter case.

use super::blocked_hosts::is_blocked_host;
use super::plugin_source_fields::{SourceSite, field_error};
use crate::format::issues::PluginIssue;

const ARCHIVE_URL: &str = "Archive URLs must use https:// and must not point at a loopback, link-local, or cloud-metadata host";

fn host_of(link: &str) -> Option<String> {
    crate::js::url::parse(link).map(|url| url.hostname)
}

/// The issues of an archive source's `url`.
#[must_use]
pub fn archive_url_issues(url: &str, site: &SourceSite) -> Vec<PluginIssue> {
    let host = host_of(url);
    let allowed = host.as_deref().is_some_and(|host| {
        url.as_bytes()
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case(b"https://"))
            && !is_blocked_host(host)
    });
    if allowed {
        return Vec::new();
    }
    let mut issues = Vec::new();
    if host.is_none() {
        issues.push(field_error(site, "url", "Invalid URL"));
    }
    issues.push(field_error(site, "url", ARCHIVE_URL));
    issues
}

crate::registry::exports! {
    "format/marketplace/archive-url" {
        "archiveUrlIssues" => ok |url: String, site: SourceSite| archive_url_issues(&url, &site);
    }
}
