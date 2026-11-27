//! Port of `src/marketplace/release-channels.ts`, switched.
//!
//! Release channels (plugins/host-marketplace.md, "Run release channels"): "Claude Code has no
//! release-channel concept". A channel is a marketplace whose entries point at another ref of the same
//! plugin; the two `marketplace.json` files need different `name` values, because "a user can't have
//! two marketplaces with the same name registered at once", and the two refs need different versions,
//! or none, because "updates are detected by comparing versions, so a ref that moves without a version
//! change leaves users on the cached copy". This checks the pair as a hosting author would before
//! publishing; the issues are findings for the author, not a refusal Claude Code makes.

use serde::Deserialize;

use super::version_resolution::usable_version;
use crate::format::issues::{PluginIssue, error, warning};
use crate::format::marketplace::types::{Marketplace, MarketplaceEntry, PluginSource};
use crate::registry::Map;

/// The `version` of each plugin's `plugin.json` at the ref a channel points at, by plugin name. It wins over the entry's version.
pub type ChannelManifestVersions = Map<String, String>;

/// The manifests' versions of each side.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ChannelManifests {
    /// The first channel's.
    #[serde(default)]
    pub first: Option<ChannelManifestVersions>,
    /// The second channel's.
    #[serde(default)]
    pub second: Option<ChannelManifestVersions>,
}

fn pin(place: &str, reference: Option<&String>, sha: Option<&String>) -> String {
    format!(
        "{place}#{}@{}",
        reference.map_or("", String::as_str),
        sha.map_or("", String::as_str)
    )
}

/// Where a source points, as one comparable string: the repository (and subdirectory) with its `ref`
/// and `sha`. A git source with neither points at the default branch, which is a place too. Every
/// source that has no ref (a path, an npm package, an archive, a command) is `None`: it names
/// no channel.
fn points_at(source: &PluginSource) -> Option<String> {
    match source {
        PluginSource::Relative { .. }
        | PluginSource::Npm { .. }
        | PluginSource::Archive { .. }
        | PluginSource::Command { .. } => None,
        PluginSource::Github {
            repo,
            reference,
            sha,
        } => Some(pin(repo, reference.as_ref(), sha.as_ref())),
        PluginSource::Url {
            url,
            reference,
            sha,
        } => Some(pin(url, reference.as_ref(), sha.as_ref())),
        PluginSource::GitSubdir {
            url,
            path,
            reference,
            sha,
        } => Some(pin(
            &format!("{url}:{path}"),
            reference.as_ref(),
            sha.as_ref(),
        )),
    }
}

fn ref_of(source: &PluginSource) -> Option<String> {
    points_at(source)
}

/// The version a channel declares for an entry: `plugin.json`'s, then the entry's; a blank one, or one that names no folder, is not set (`usableVersion`, the rule the install uses).
fn channel_version(
    entry: &MarketplaceEntry,
    manifests: Option<&ChannelManifestVersions>,
) -> Option<String> {
    let declared = manifests.and_then(|versions| {
        versions
            .entries
            .iter()
            .find(|(name, _)| *name == entry.name)
            .map(|(_, version)| version.as_str())
    });
    usable_version(declared).or_else(|| usable_version(entry.version.as_deref()))
}

fn names(name: &str) -> PluginIssue {
    error(
        "name",
        &format!(
            "Both marketplaces are named \"{name}\": give the two marketplace.json files different name values, because Claude Code identifies a marketplace by its name"
        ),
    )
}

fn same_version(name: &str, version: &str) -> PluginIssue {
    warning(
        &format!("plugins.{name}.version"),
        &format!(
            "Both channels declare version \"{version}\" for \"{name}\": give the two refs different versions, or omit version so the commit SHA distinguishes them, because a ref that moves without a version change leaves users on the cached copy"
        ),
    )
}

fn same_ref(name: &str) -> PluginIssue {
    warning(
        &format!("plugins.{name}.source"),
        &format!(
            "Both channels point \"{name}\" at the same ref and sha: a channel needs its own ref of the plugin"
        ),
    )
}

fn plugin_findings(
    one: &MarketplaceEntry,
    other: &MarketplaceEntry,
    manifests: &ChannelManifests,
) -> Vec<PluginIssue> {
    let mut found = Vec::new();
    let first = channel_version(one, manifests.first.as_ref());
    let second = channel_version(other, manifests.second.as_ref());
    if let Some(first) = first.filter(|first| Some(first) == second.as_ref()) {
        found.push(same_version(&one.name, &first));
    }
    let reference = ref_of(&one.source);
    if reference.is_some() && reference == ref_of(&other.source) {
        found.push(same_ref(&one.name));
    }
    found
}

/// What is wrong with `first` and `second` as the two catalogs of a channel pair. Judgments
/// (spec/judgments.md): the same name is an error and the rest are warnings; only plugins both list
/// are compared; an unset version on either side is fine (the SHA tells them apart); two entries that
/// point at the same ref and sha are reported, since they are not two channels.
#[must_use]
pub fn check_release_channels(
    first: &Marketplace,
    second: &Marketplace,
    manifests: &ChannelManifests,
) -> Vec<PluginIssue> {
    let mut found = if first.name == second.name {
        vec![names(&first.name)]
    } else {
        vec![]
    };
    // `new Map(...)`: a later entry of one name replaces an earlier one.
    for entry in &first.plugins {
        if let Some(other) = second
            .plugins
            .iter()
            .rev()
            .find(|other| other.name == entry.name)
        {
            found.extend(plugin_findings(entry, other, manifests));
        }
    }
    found
}

crate::registry::exports! {
    "marketplace/release-channels" {
        "checkReleaseChannels" => ok |first: Marketplace, second: Marketplace, manifests: Option<ChannelManifests>|
            check_release_channels(&first, &second, &manifests.unwrap_or_default());
    }
}
