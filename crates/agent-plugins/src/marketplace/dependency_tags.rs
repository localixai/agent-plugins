//! Port of `src/marketplace/dependency-tags.ts`, switched.
//!
//! How a dependency's version constraint meets git tags (plugins/dependencies.md, "Release a plugin
//! that others depend on", "How a constraint resolves against tags", "Combine constraints from
//! several plugins"). A dependency with a range installs at the highest tag `<plugin-name>--v<version>`
//! that satisfies it; tags come from the `GitTags` port, so everything here is pure.

use serde::{Deserialize, Serialize};

use super::dependencies::dependency_id;
use super::semver::{
    highest_satisfying, highest_version, is_valid_range, ranges_overlap, satisfies_range,
    strict_version,
};
use super::types::NormalizedDependency;
use super::version_hash::short_hash_12;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::types::{PluginSource, PluginSourceKind};

/// A release tag and the version it names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReleaseTag {
    /// The tag.
    pub tag: String,
    /// The version it names.
    pub version: String,
}

/// `<plugin-name>--v<version>` (plugins/dependencies.md, "Create a release tag").
#[must_use]
pub fn release_tag(name: &str, version: &str) -> String {
    format!("{name}--v{version}")
}

/// The version a tag names for `name`, or `None` for another plugin's tag or one whose version
/// is not strict semver. Judgment (spec/judgments.md): a `v`, `=` or blank before the version
/// (`name--vv2.1.0`) is not a release tag; the docs name only `<name>--v<version>`.
#[must_use]
pub fn parse_release_tag(tag: &str, name: &str) -> Option<String> {
    let prefix = release_tag(name, "");
    tag.strip_prefix(prefix.as_str()).and_then(strict_version)
}

fn release_tags(tags: &[String], name: &str) -> Vec<ReleaseTag> {
    tags.iter()
        .filter_map(|tag| {
            parse_release_tag(tag, name).map(|version| ReleaseTag {
                tag: tag.clone(),
                version,
            })
        })
        .collect()
}

/// The release tag at `version`, among those already parsed.
fn tag_at(found: &[ReleaseTag], version: Option<&str>) -> Option<ReleaseTag> {
    let version = version?;
    found.iter().find(|one| one.version == version).cloned()
}

/// The highest tag of `name` that satisfies `range`, with node-semver's rules: a range never matches
/// a pre-release unless it opts in (`^2.0.0-0`). `None` when none does.
#[must_use]
pub fn select_tag(tags: &[String], name: &str, range: &str) -> Option<ReleaseTag> {
    let found = release_tags(tags, name);
    let versions: Vec<String> = found.iter().map(|one| one.version.clone()).collect();
    tag_at(&found, highest_satisfying(&versions, range).as_deref())
}

/// The id, not the bare name, names the dependency: two marketplaces can host the same plugin name (troubleshooting.md uses `"<dep>"`).
fn problem(dep: &NormalizedDependency, what: &str) -> PluginIssue {
    let id = dependency_id(dep);
    error(
        &format!("dependencies.{id}"),
        &format!("Dependency \"{id}\" {what}"),
    )
}

/// What `combineRanges` found: a tag, or why the ranges cannot all hold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Combined {
    /// The highest tag that satisfies every range.
    Tag(ReleaseTag),
    /// The ranges cannot all hold.
    Issue(PluginIssue),
}

/// The distinct members of `values`, in order (`[...new Set(values)]`).
fn distinct_of(values: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        if !out.contains(value) {
            out.push(value.clone());
        }
    }
    out
}

/// The tag a dependency resolves to when several installed plugins constrain it: the highest that
/// satisfies every range. A `PluginIssue` when the ranges cannot all hold: `has an invalid version
/// requirement`, `has version requirements too complex to intersect` or `has conflicting version
/// requirements` (with the ranges listed); `None` when they overlap but no tag falls in the
/// overlap, which is when the marketplace's current copy is fetched (`autoUpdateDecision` decides
/// what that means). The dependency's `name` reads the tags; its id names it in a message. Judgment:
/// the text after the quoted words, and 64 combinations of `||` alternatives as the limit of "too complex".
///
/// # Errors
///
/// What node-semver throws for a version or range it cannot read.
pub fn combine_ranges(
    ranges: &[String],
    tags: &[String],
    dep: &NormalizedDependency,
) -> Result<Option<Combined>, String> {
    let distinct = distinct_of(ranges);
    if distinct.is_empty() {
        return Ok(None);
    }
    if let Some(invalid) = distinct.iter().find(|range| !is_valid_range(range)) {
        let what = format!("has an invalid version requirement: \"{invalid}\"");
        return Ok(Some(Combined::Issue(problem(dep, &what))));
    }
    let Some(overlap) = ranges_overlap(&distinct)? else {
        let what = "has version requirements too complex to intersect";
        return Ok(Some(Combined::Issue(problem(dep, what))));
    };
    if !overlap {
        let what = format!(
            "has conflicting version requirements: {}",
            distinct.join(", ")
        );
        return Ok(Some(Combined::Issue(problem(dep, &what))));
    }
    let fits: Vec<ReleaseTag> = release_tags(tags, &dep.name)
        .into_iter()
        .filter(|one| {
            distinct
                .iter()
                .all(|range| satisfies_range(&one.version, range))
        })
        .collect();
    let versions: Vec<String> = fits.iter().map(|one| one.version.clone()).collect();
    Ok(tag_at(&fits, highest_version(&versions)?.as_deref()).map(Combined::Tag))
}

/// How an update to a tag or to the current copy was decided.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "via", rename_all = "kebab-case")]
pub enum UpdateVia {
    /// To a tag.
    Tag {
        /// The tag.
        tag: String,
        /// Its version.
        version: String,
    },
    /// To the marketplace's current copy.
    CurrentCopy {
        /// Its `plugin.json` version.
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
}

/// What auto-update does with a dependency that installed plugins constrain (plugins/dependencies.md, "Combine constraints from several plugins").
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
pub enum AutoUpdateDecision {
    /// It moves.
    Update(UpdateVia),
    /// It stays.
    Skip(SkipReason),
}

/// Why an update is skipped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum SkipReason {
    /// The ranges do not overlap.
    RangesConflict {
        /// The conflict.
        issue: PluginIssue,
    },
    /// The current copy's version falls outside a range.
    CurrentCopyOutOfRange {
        /// Its `plugin.json` version.
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
}

/// The facts of an auto-update.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateInput {
    /// The dependency.
    pub dep: NormalizedDependency,
    /// The range of every installed plugin that constrains the dependency.
    pub ranges: Vec<String>,
    /// The tags.
    pub tags: Vec<String>,
    /// The `version` in the plugin.json of the marketplace's current copy.
    #[serde(default)]
    pub current_copy_version: Option<String>,
}

fn current_copy_decision(input: &AutoUpdateInput) -> AutoUpdateDecision {
    let version = input.current_copy_version.clone();
    let fits = version.as_ref().is_some_and(|version| {
        input
            .ranges
            .iter()
            .all(|range| satisfies_range(version, range))
    });
    if fits || input.ranges.is_empty() {
        AutoUpdateDecision::Update(UpdateVia::CurrentCopy { version })
    } else {
        AutoUpdateDecision::Skip(SkipReason::CurrentCopyOutOfRange { version })
    }
}

/// Whether auto-update moves a constrained dependency, and to what (plugins/dependencies.md, "Combine
/// constraints from several plugins"): to the highest tag that satisfies every installed plugin's
/// range; when the ranges do not overlap, nowhere (the dependency stays at its current version and
/// the Errors tab names the constraining plugin: `issue` is that conflict); when they overlap and no
/// tag falls in the overlap, to the marketplace's current copy, unless its `version` falls outside any
/// range, which skips the update. A copy that states no version satisfies no range (judgment).
///
/// # Errors
///
/// What node-semver throws for a version or range it cannot read.
pub fn auto_update_decision(input: &AutoUpdateInput) -> Result<AutoUpdateDecision, String> {
    Ok(
        match combine_ranges(&input.ranges, &input.tags, &input.dep)? {
            None => current_copy_decision(input),
            Some(Combined::Issue(issue)) => {
                AutoUpdateDecision::Skip(SkipReason::RangesConflict { issue })
            }
            Some(Combined::Tag(found)) => AutoUpdateDecision::Update(UpdateVia::Tag {
                tag: found.tag,
                version: found.version,
            }),
        },
    )
}

/// What the constraint was checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Via {
    /// A tag.
    #[serde(rename = "tag")]
    Tag,
    /// The marketplace's current copy.
    #[serde(rename = "current-copy")]
    CurrentCopy,
}

/// What the constraint was checked against: a tag, or the marketplace's current copy (`version` is its `plugin.json` version, absent when it has none).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedVersion {
    /// Which.
    pub via: Via,
    /// The tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// The version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// A dependency's constraint and where its plugin comes from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResolution {
    /// The dependency.
    pub dep: NormalizedDependency,
    /// Every range that constrains the dependency when several installed plugins do (`dep.range` is
    /// the one of the plugin being installed); absent, `dep.range` alone applies.
    #[serde(default)]
    pub ranges: Option<Vec<String>>,
    /// Where the plugin comes from.
    pub source: PluginSource,
    /// The tags of the repository that hosts the plugin (empty for a source with none, or a folder that is not a git repository).
    pub tags: Vec<String>,
    /// The `version` in the plugin.json of the marketplace's current copy.
    #[serde(default)]
    pub current_copy_version: Option<String>,
}

/// A resolved version, or the issue that stopped it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Resolution {
    /// Resolved.
    Version(ResolvedVersion),
    /// Not resolved.
    Issue(PluginIssue),
}

fn current_copy(input: &DependencyResolution) -> ResolvedVersion {
    ResolvedVersion {
        via: Via::CurrentCopy,
        tag: None,
        version: input.current_copy_version.clone(),
    }
}

fn by_tag(found: ReleaseTag) -> ResolvedVersion {
    ResolvedVersion {
        via: Via::Tag,
        tag: Some(found.tag),
        version: Some(found.version),
    }
}

/// The distinct ranges that apply: the ones given, else the dependency's own.
fn ranges_of(input: &DependencyResolution) -> Vec<String> {
    match (&input.ranges, &input.dep.range) {
        (Some(given), _) => distinct_of(given),
        (None, Some(own)) => vec![own.clone()],
        (None, None) => Vec::new(),
    }
}

/// The highest satisfying tag, a conflict between the ranges, or `None` when there is no range
/// or no tag fits. One range selects directly; several must overlap (`combineRanges`).
fn tag_for(input: &DependencyResolution) -> Result<Option<Combined>, String> {
    let ranges = ranges_of(input);
    match ranges.as_slice() {
        [] => Ok(None),
        [only] => Ok(select_tag(&input.tags, &input.dep.name, only).map(Combined::Tag)),
        _ => combine_ranges(&ranges, &input.tags, &input.dep),
    }
}

fn resolution_of(found: Combined) -> Resolution {
    match found {
        Combined::Issue(issue) => Resolution::Issue(issue),
        Combined::Tag(found) => Resolution::Version(by_tag(found)),
    }
}

/// A plugin with its own repository: no tag in range fails the install.
fn own_repository(input: &DependencyResolution) -> Result<Resolution, String> {
    if let Some(found) = tag_for(input)? {
        return Ok(resolution_of(found));
    }
    let ranges = ranges_of(input);
    if ranges.is_empty() {
        return Ok(Resolution::Version(current_copy(input)));
    }
    let id = dependency_id(&input.dep);
    Ok(Resolution::Issue(error(
        &format!("dependencies.{id}"),
        &format!(
            "Dependency \"{id}\" has no git tag satisfying {}",
            ranges.join(" ")
        ),
    )))
}

/// A plugin in the marketplace repository: no tag in range installs the current copy, checked at load.
fn marketplace_repository(input: &DependencyResolution) -> Result<Resolution, String> {
    Ok(match tag_for(input)? {
        None => Resolution::Version(current_copy(input)),
        Some(found) => resolution_of(found),
    })
}

/// `npm`, `archive` and `command`: the range does not choose the version, the plugin.json version is checked at load.
#[expect(
    clippy::unnecessary_wraps,
    reason = "one of the table's resolvers, which share a signature"
)]
fn plugin_json(input: &DependencyResolution) -> Result<Resolution, String> {
    Ok(Resolution::Version(current_copy(input)))
}

/// The version `claude plugin list` shows for a resolved dependency (plugins/dependencies.md,
/// "Confirm the resolved version"): a tag-resolved copy carries "a 12-character commit suffix, such as
/// `2.1.0-8713c5b11005`"; the commit is the one the tag names, which the host knows. A copy with no
/// commit known shows the bare tag version (judgment), a current copy its `plugin.json` version.
#[must_use]
pub fn resolved_version_label(resolved: &ResolvedVersion, commit: Option<&str>) -> Option<String> {
    match (&resolved.version, commit) {
        (Some(version), Some(commit)) if resolved.via == Via::Tag => {
            Some(format!("{version}-{}", short_hash_12(commit)))
        }
        _ => resolved.version.clone(),
    }
}

type Resolver = fn(&DependencyResolution) -> Result<Resolution, String>;

/// The resolver of each kind of plugin source.
const fn resolver_of(kind: PluginSourceKind) -> Resolver {
    match kind {
        PluginSourceKind::Github | PluginSourceKind::Url | PluginSourceKind::GitSubdir => {
            own_repository
        }
        PluginSourceKind::Relative => marketplace_repository,
        PluginSourceKind::Npm | PluginSourceKind::Archive | PluginSourceKind::Command => {
            plugin_json
        }
    }
}

/// What a dependency's constraint resolves against, by the kind of its plugin source: a tag of its
/// own repository (`github`, `url`, `git-subdir`: none in range is `has no git tag satisfying`), a
/// tag of the marketplace repository or else its current copy (`relative`), or the `plugin.json`
/// version of the fetched plugin (`npm`, `archive`, `command`). The `version` returned is the one a
/// constraint is checked against: the tag's, which beats `plugin.json`, when resolved by tag.
///
/// # Errors
///
/// What node-semver throws for a version or range it cannot read.
pub fn resolve_dependency_version(input: &DependencyResolution) -> Result<Resolution, String> {
    resolver_of(input.source.kind())(input)
}

crate::registry::exports! {
    "marketplace/dependency-tags" {
        "releaseTag" => ok |name: String, version: String| release_tag(&name, &version);
        "parseReleaseTag" => ok |tag: String, name: String| parse_release_tag(&tag, &name);
        "selectTag" => ok |tags: Vec<String>, name: String, range: String| select_tag(&tags, &name, &range);
        "combineRanges" => fallible |ranges: Vec<String>, tags: Vec<String>, dep: NormalizedDependency| combine_ranges(&ranges, &tags, &dep);
        "autoUpdateDecision" => fallible |input: AutoUpdateInput| auto_update_decision(&input);
        "resolvedVersionLabel" => ok |resolved: ResolvedVersion, commit: Option<String>| resolved_version_label(&resolved, commit.as_deref());
        "resolveDependencyVersion" => fallible |input: DependencyResolution| resolve_dependency_version(&input);
    }
}
