//! Port of `src/marketplace/archive-source.ts`, switched.
//!
//! Fetching an `archive` source (plugins/marketplace-reference.md, "archive plugin source";
//! plugins/host-marketplace.md, "Stay within the download limits for hosted files"): the download
//! goes through `HttpClient`, the digest through `Sha256`, the unzip is the host's (`ArchiveReader`
//! reports what the zip holds) and the kit judges every documented limit. A refusal is a
//! `PluginIssue`; nothing throws. The totals, the plugin root and the paths and links that would
//! leave the plugin directory are all judged here (`archive_scan.rs`), before a host unzips
//! anything.

use serde::{Deserialize, Serialize};

use super::archive_scan::{archive_stats, unsafe_entry_issue};
use super::http_download::{Download, download_following};
use super::install_failures::archive_integrity_message;
use super::plugin_content::PLUGIN_CONTENT_DIRECTORIES;
use super::source_safety::archive_url_issues;
use super::types::{ArchiveFetchPlan, ArchiveLimits};
use crate::format::issues::{PluginIssue, error};
use crate::format::path_text::without_trailing_slashes;
use crate::js::number_to_string;
use crate::ports::Signal;
use crate::ports::http_client::{BodyAs, HttpClient};
use crate::ports::marketplace::{ArchiveReader, ArchiveStats, Sha256};
use crate::registry::{Bytes, Port};

const MIB: f64 = 1024.0 * 1024.0;

/// The verified zip and where the plugin root is in it: `""` at the top, or the one directory down.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FetchedArchive {
    /// The zip.
    pub bytes: Bytes,
    /// Its totals.
    pub stats: ArchiveStats,
    /// The plugin root in it.
    pub root: String,
}

/// "Every redirect target must use `https://` and can't point at a loopback, link-local, or
/// cloud-metadata host."
fn refuse_archive_hop(target: &str, _from: &str) -> Option<String> {
    let refused = archive_url_issues(target, "source.url")
        .into_iter()
        .next()?;
    Some(format!("The redirect is refused: {}", refused.message))
}

/// A limit as a count the download takes: the documented limits are whole and not negative.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a documented limit is a whole number of bytes, milliseconds or hops"
)]
const fn count(limit: f64) -> u64 {
    limit as u64
}

/// Downloads the zip (`download_following`); a body cut at the limit is refused.
async fn download(
    plan: &ArchiveFetchPlan,
    http: &dyn HttpClient,
    signal: &Signal,
) -> Result<Vec<u8>, PluginIssue> {
    let limits = &plan.limits;
    let download = Download {
        url: plan.url.clone(),
        headers: plan.headers.clone(),
        body_as: BodyAs::Bytes,
        timeout_ms: count(limits.timeout_ms),
        max_bytes: count(limits.max_download_bytes),
        max_redirects: u32::try_from(count(limits.max_redirects)).unwrap_or(u32::MAX),
        refuse_hop: refuse_archive_hop,
        path: "source.url".to_owned(),
    };
    let response = download_following(&download, http, signal).await?;
    if response.truncated {
        return Err(error(
            "source.url",
            &format!(
                "The archive is larger than {} MiB",
                number_to_string(limits.max_download_bytes / MIB)
            ),
        ));
    }
    response
        .bytes
        .ok_or_else(|| error("source.url", "The server sent no body"))
}

/// "When you set it, Claude Code refuses a download that doesn't match." Compared without regard
/// to case.
async fn digest_issue(
    bytes: &[u8],
    plan: &ArchiveFetchPlan,
    sha256: &dyn Sha256,
) -> Option<PluginIssue> {
    let expected = plan.sha256.as_ref()?.to_lowercase();
    let actual = sha256.hex(bytes).await.to_lowercase();
    (actual != expected).then(|| {
        error(
            "source.sha256",
            &archive_integrity_message(&plan.url, &expected, &actual),
        )
    })
}

/// "The install fails when the zip exceeds any of these extraction limits." The ratio is against
/// the size of the zip the kit downloaded (`zip_bytes`), never a size the host reports.
#[expect(
    clippy::cast_precision_loss,
    reason = "sizes compare as the JavaScript numbers the limits are"
)]
fn extraction_issue(
    stats: &ArchiveStats,
    zip_bytes: usize,
    limits: &ArchiveLimits,
) -> Option<PluginIssue> {
    let breaks = [
        (
            stats.entries as f64 > limits.max_entries,
            format!(
                "The archive has more than {} files and directories",
                number_to_string(limits.max_entries)
            ),
        ),
        (
            stats.largest_file_bytes as f64 > limits.max_file_bytes,
            format!(
                "A file in the archive is larger than {} MiB uncompressed",
                number_to_string(limits.max_file_bytes / MIB)
            ),
        ),
        (
            stats.uncompressed_bytes as f64 > limits.max_uncompressed_bytes,
            format!(
                "The archive is larger than {} MiB uncompressed",
                number_to_string(limits.max_uncompressed_bytes / MIB)
            ),
        ),
        (
            stats.uncompressed_bytes as f64 > limits.max_compression_ratio * zip_bytes as f64,
            format!(
                "The archive's content is more than {} times the size of the zip",
                number_to_string(limits.max_compression_ratio)
            ),
        ),
    ];
    let (_, message) = breaks.into_iter().find(|(exceeded, _)| *exceeded)?;
    Some(error("source", &message))
}

/// The folder of resource forks macOS Finder adds beside what it zips: never the plugin (judgment).
const FINDER_FORKS: &str = "__MACOSX";

/// "The plugin root may be at the top of the zip or one directory down": at the top when plugin
/// content is there, else the one wrapping directory.
fn root_of(top_level: &[String]) -> Result<String, PluginIssue> {
    let names: Vec<&str> = top_level
        .iter()
        .map(|name| without_trailing_slashes(name))
        .filter(|name| *name != FINDER_FORKS)
        .collect();
    if names
        .iter()
        .any(|name| PLUGIN_CONTENT_DIRECTORIES.contains(name))
    {
        return Ok(String::new());
    }
    match names.as_slice() {
        [only] => Ok((*only).to_owned()),
        _ => Err(error(
            "source",
            "The archive has no plugin root at its top or one directory down",
        )),
    }
}

/// What fetching an archive needs of the host.
pub struct ArchivePorts<'a> {
    /// The download.
    pub http: &'a dyn HttpClient,
    /// The digest.
    pub sha256: &'a dyn Sha256,
    /// The zip's entries.
    pub archive: &'a dyn ArchiveReader,
}

/// Downloads the archive and judges it; the verified zip and its plugin root, or the issue that
/// refuses it.
pub async fn fetch_archive(
    plan: &ArchiveFetchPlan,
    ports: &ArchivePorts<'_>,
    signal: &Signal,
) -> Result<FetchedArchive, PluginIssue> {
    if let Some(blocked) = archive_url_issues(&plan.url, "source.url")
        .into_iter()
        .next()
    {
        return Err(blocked);
    }
    let bytes = download(plan, ports.http, signal).await?;
    if let Some(mismatch) = digest_issue(&bytes, plan, ports.sha256).await {
        return Err(mismatch);
    }
    let scan = ports
        .archive
        .scan(&bytes, signal)
        .await
        .map_err(|failure| {
            error(
                "source",
                &format!("The archive cannot be read: {}", failure.message),
            )
        })?;
    let stats = archive_stats(&scan, bytes.len() as u64);
    if let Some(oversized) = extraction_issue(&stats, bytes.len(), &plan.limits) {
        return Err(oversized);
    }
    let root = root_of(&stats.top_level)?;
    if let Some(unsafe_entry) = unsafe_entry_issue(&scan, &root) {
        return Err(unsafe_entry);
    }
    Ok(FetchedArchive {
        bytes: Bytes(bytes),
        stats,
        root,
    })
}

/// The ports of `fetchArchive` as they cross: a record of the host's objects.
#[derive(Deserialize)]
struct ArchivePortsGiven {
    http: Port,
    sha256: Port,
    archive: Port,
}

/// `{ signal }`.
#[derive(Deserialize)]
struct WithSignal {
    signal: Signal,
}

/// A fetched archive or the issue, as the TypeScript returns either.
#[derive(Serialize)]
#[serde(untagged)]
enum Fetched {
    Archive(FetchedArchive),
    Issue(PluginIssue),
}

async fn fetch_given(
    plan: ArchiveFetchPlan,
    given: ArchivePortsGiven,
    options: WithSignal,
) -> Fetched {
    let ports = ArchivePorts {
        http: &given.http,
        sha256: &given.sha256,
        archive: &given.archive,
    };
    match fetch_archive(&plan, &ports, &options.signal).await {
        Ok(archive) => Fetched::Archive(archive),
        Err(issue) => Fetched::Issue(issue),
    }
}

crate::registry::exports! {
    "marketplace/archive-source" {
        "fetchArchive" => async ok |plan: ArchiveFetchPlan, ports: ArchivePortsGiven, options: WithSignal|
            fetch_given(plan, ports, options).await;
    }
}
