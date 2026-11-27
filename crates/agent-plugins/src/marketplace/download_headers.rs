//! Port of `src/marketplace/download-headers.ts`, switched.
//!
//! The headers of a download (plugins/host-marketplace.md, "Authenticate archive downloads"; "Stay
//! within the download limits"; marketplace-reference.md, "Fields by type", "Plugin entries"):
//! `headers` and `headersHelper` can be set on the marketplace's `url` source and on an entry; the
//! entry's value replaces the source's of the same name, and within one place the command's output
//! replaces a header of the same name in `headers`. Redirects to another origin carry none of them:
//! `fetchArchive` drops them on that hop. The prompt that asks a user to accept an entry's command,
//! and the command's own execution, are the host's (`CommandRunner`); here the policy and the merge.

use indexmap::IndexMap;
use serde::Deserialize;

use super::command_source::WithSignal;
use super::header_helper::{
    HeadersHelperPorts, HelperCache, HelperOutcome, HelperSetting, Produce,
};
use super::header_map::{merge_headers, normalize_headers, without_routing_headers};
use super::header_policy::{
    AcceptedCommand, HeaderLayer, HeaderOrigin, HeaderPlace, HeadersSetting, HelperEffect,
    HelperVerdict, PluginOperation, filters_routing, helper_allowed,
};
use super::types::{HeaderMap, ResolvedHeaders};
use crate::format::issues::error;
use crate::js::url;
use crate::ports::Signal;
use crate::registry::Port;

/// The host's ports (`DownloadHeaderPorts`): a run's, and the cache.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct DownloadHeaderPorts {
    /// Runs the command.
    pub runner: Port,
    /// The environment the command inherits.
    pub environment: Port,
    /// Finds the configuration directory.
    pub locations: Port,
    /// One per host session, so a `url` source's command output is reused for 60 seconds.
    pub cache: Port,
}

impl DownloadHeaderPorts {
    /// The ports a run needs.
    const fn helper(&self) -> HeadersHelperPorts {
        HeadersHelperPorts {
            runner: self.runner,
            environment: self.environment,
            locations: self.locations,
        }
    }
}

/// The `url` source of a marketplace, declared in a settings file (`UrlSourceHeaders`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlSourceHeaders {
    /// The marketplace URL.
    pub url: String,
    /// Its headers.
    #[serde(default)]
    pub headers: Option<IndexMap<String, String>>,
    /// The helper that makes them.
    #[serde(default)]
    pub headers_helper: Option<String>,
    /// Unset on the first fetch after a user adds a marketplace by URL: that fetch supplies the name.
    #[serde(default)]
    pub name: Option<String>,
}

/// The layer of a `url` source.
#[must_use]
pub fn url_source_layer(source: &UrlSourceHeaders, origin: HeaderOrigin) -> HeaderLayer {
    let mut variables =
        IndexMap::from([("CLAUDE_CODE_MARKETPLACE_URL".to_owned(), source.url.clone())]);
    if let Some(name) = &source.name {
        variables.insert("CLAUDE_CODE_MARKETPLACE_NAME".to_owned(), name.clone());
    }
    HeaderLayer {
        place: HeaderPlace::UrlSource,
        origin,
        headers: source.headers.clone(),
        headers_helper: source.headers_helper.clone(),
        variables,
        marketplace_url: Some(source.url.clone()),
        archive_url: None,
        operation: None,
        accepted: None,
    }
}

/// A plugin entry's fields (`MarketplaceEntry`, or an inline entry in settings).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryHeaders {
    /// The entry name.
    pub name: String,
    /// Its headers.
    #[serde(default)]
    pub headers: Option<IndexMap<String, String>>,
    /// The helper that makes them.
    #[serde(default)]
    pub headers_helper: Option<String>,
}

/// Where an entry is declared and what the user is doing.
#[derive(Clone, Debug, Deserialize)]
pub struct EntryDeclaration {
    /// Where the declaration lives.
    pub origin: HeaderOrigin,
    /// What the user is doing.
    pub operation: PluginOperation,
    /// The command and archive URL the user accepted; unset when they accepted nothing.
    #[serde(default)]
    pub accepted: Option<AcceptedCommand>,
}

/// The layer of an entry.
#[must_use]
pub fn entry_layer(
    entry: &EntryHeaders,
    archive_url: &str,
    declared: &EntryDeclaration,
) -> HeaderLayer {
    HeaderLayer {
        place: HeaderPlace::Entry,
        origin: declared.origin,
        headers: entry.headers.clone(),
        headers_helper: entry.headers_helper.clone(),
        variables: IndexMap::from([
            ("CLAUDE_CODE_PLUGIN_NAME".to_owned(), entry.name.clone()),
            (
                "CLAUDE_CODE_PLUGIN_ARCHIVE_URL".to_owned(),
                archive_url.to_owned(),
            ),
        ]),
        marketplace_url: None,
        archive_url: Some(archive_url.to_owned()),
        operation: Some(declared.operation),
        accepted: declared.accepted.clone(),
    }
}

fn keep_filtered(layer: &HeaderLayer, headers: HeaderMap) -> HeaderMap {
    if filters_routing(layer) {
        without_routing_headers(&headers)
    } else {
        headers
    }
}

/// `REFUSAL_RESULT`: what a refused command does: skip it and send the fixed headers, or refuse the download.
fn refusal_result(effect: HelperEffect, fixed: HeaderMap, message: &str) -> HelperOutcome {
    match effect {
        HelperEffect::SkipHelper => HelperOutcome::Resolved(ResolvedHeaders { headers: fixed }),
        HelperEffect::RefuseDownload => HelperOutcome::Issue(error("headersHelper", message)),
    }
}

/// The headers one place sends: its `headers`, with the command's output over them when the
/// command may run. Used for a `url` source's `marketplace.json` fetch and, through
/// `archiveDownloadHeaders`, for an archive. A command that fails, or a refused one that stops
/// the download, is a `PluginIssue`.
pub async fn resolve_headers(
    layer: &HeaderLayer,
    setting: &HeadersSetting,
    ports: &DownloadHeaderPorts,
    signal: &Signal,
) -> HelperOutcome {
    let fixed = keep_filtered(
        layer,
        normalize_headers(&layer.headers.clone().unwrap_or_default()),
    );
    if layer.headers_helper.is_none() {
        return HelperOutcome::Resolved(ResolvedHeaders { headers: fixed });
    }
    if let HelperVerdict::Refused {
        effect, message, ..
    } = helper_allowed(layer, setting)
    {
        return refusal_result(effect, fixed, &message);
    }
    let produce = Produce::new(
        layer.clone(),
        ports.helper(),
        HelperSetting {
            platform: setting.platform,
        },
        signal.clone(),
    );
    match ports.cache.remember(layer, produce).await {
        HelperOutcome::Issue(issue) => HelperOutcome::Issue(issue),
        HelperOutcome::Resolved(printed) => HelperOutcome::Resolved(ResolvedHeaders {
            headers: merge_headers(&fixed, &keep_filtered(layer, printed.headers)),
        }),
    }
}

/// What `archiveDownloadHeaders` is given.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadHeaderRequest {
    /// The archive's URL.
    pub archive_url: String,
    /// The marketplace's `url` source, when it was added by URL.
    #[serde(default)]
    pub marketplace: Option<HeaderLayer>,
    /// The entry's layer.
    #[serde(default)]
    pub entry: Option<HeaderLayer>,
}

fn origin_of(text: &str) -> Option<String> {
    url::parse(text).map(|parsed| parsed.origin)
}

/// "Archive downloads on the marketplace URL's origin, meaning the same scheme, host, and port."
fn source_applies(marketplace: Option<&HeaderLayer>, archive_url: &str) -> bool {
    let Some(marketplace_url) = marketplace.and_then(|layer| layer.marketplace_url.as_deref())
    else {
        return false;
    };
    origin_of(marketplace_url).is_some_and(|origin| Some(origin) == origin_of(archive_url))
}

/// The headers of an archive download: the `url` source's when the archive is on its origin, then
/// the entry's over them. The result fills `ArchiveFetchPlan.headers` (`planPluginFetch` leaves it
/// empty because it knows no settings).
pub async fn archive_download_headers(
    request: &DownloadHeaderRequest,
    setting: &HeadersSetting,
    ports: &DownloadHeaderPorts,
    signal: &Signal,
) -> HelperOutcome {
    let from_source = match &request.marketplace {
        Some(marketplace) if source_applies(Some(marketplace), &request.archive_url) => {
            match resolve_headers(marketplace, setting, ports, signal).await {
                HelperOutcome::Issue(issue) => return HelperOutcome::Issue(issue),
                HelperOutcome::Resolved(resolved) => resolved,
            }
        }
        _ => ResolvedHeaders::default(),
    };
    let from_entry = match &request.entry {
        None => ResolvedHeaders::default(),
        Some(entry) => match resolve_headers(entry, setting, ports, signal).await {
            HelperOutcome::Issue(issue) => return HelperOutcome::Issue(issue),
            HelperOutcome::Resolved(resolved) => resolved,
        },
    };
    HelperOutcome::Resolved(ResolvedHeaders {
        headers: merge_headers(&from_source.headers, &from_entry.headers),
    })
}

crate::registry::exports! {
    "marketplace/download-headers" {
        "urlSourceLayer" => ok |source: UrlSourceHeaders, origin: HeaderOrigin| url_source_layer(&source, origin);
        "entryLayer" => ok |entry: EntryHeaders, archive_url: String, declared: EntryDeclaration|
            entry_layer(&entry, &archive_url, &declared);
        "resolveHeaders" => async ok
            |layer: HeaderLayer, setting: HeadersSetting, ports: DownloadHeaderPorts, options: WithSignal|
            resolve_headers(&layer, &setting, &ports, &options.signal).await;
        "archiveDownloadHeaders" => async ok
            |request: DownloadHeaderRequest, setting: HeadersSetting, ports: DownloadHeaderPorts, options: WithSignal|
            archive_download_headers(&request, &setting, &ports, &options.signal).await;
    }
}
