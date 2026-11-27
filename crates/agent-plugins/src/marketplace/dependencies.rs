//! Port of `src/marketplace/dependencies.ts`, switched.
//!
//! A plugin's `dependencies` (plugins/dependencies.md, "Declare a dependency with a version
//! constraint"): an entry is `"name"`, `"name@marketplace"` or an object whose `name` (required),
//! `version` and `marketplace` are strings. The marketplace is the declaring plugin's own unless set;
//! the `version` is a node-semver range. This is the single owner of that reading; `loading` and the
//! entry validators import it. Checks of what is installed are in `checkInstalledDependency` and
//! `checkDependency`; tags and versions are in `dependency-tags.ts`.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::ids::{IdPart, plugin_id_issues};
use crate::js::{JsObject, JsValue, trim};
use crate::marketplace::semver::{is_valid_range, satisfies_range};
use crate::marketplace::types::NormalizedDependency;

const PATH: &str = "dependencies";

/// `name@marketplace`, or `name` when the dependency has no marketplace (a plugin loaded from
/// claude.ai names none).
#[must_use]
pub fn dependency_id(dep: &NormalizedDependency) -> String {
    match &dep.marketplace {
        None => dep.name.clone(),
        Some(marketplace) => format!("{}@{marketplace}", dep.name),
    }
}

/// What a dependency item says, before its defaults apply.
struct Parts {
    name: String,
    marketplace: Option<String>,
    version: Option<String>,
}

/// An optional string field: absent is `None`; a value that is not a string, or an empty one
/// where `empty` is false, is the issue.
fn string_field(raw: &JsObject, key: &str, path: &str) -> Result<Option<String>, PluginIssue> {
    let value = match raw.get(key) {
        None | Some(JsValue::Undefined) => return Ok(None),
        Some(value) => value,
    };
    let empty = key == "version";
    if let JsValue::String(text) = value
        && (empty || !text.is_empty())
    {
        return Ok(Some(text.clone()));
    }
    let wanted = if empty {
        "a string"
    } else {
        "a non-empty string"
    };
    Err(error(
        &format!("{path}.{key}"),
        &format!("\"{key}\" must be {wanted}"),
    ))
}

/// The first way a `name` or `marketplace` breaks the id grammar (`PLUGIN_ID_PART`), reported on
/// `path`.
fn id_issue(part: IdPart, text: Option<&str>, path: &str) -> Option<PluginIssue> {
    text.and_then(|text| plugin_id_issues(part, text, path).into_iter().next())
}

/// Each part of a dependency is a part of a plugin id: the name as in its marketplace entry, and
/// the marketplace.
fn check_parts(
    parts: Parts,
    name_path: &str,
    marketplace_path: &str,
) -> Result<Parts, PluginIssue> {
    match id_issue(IdPart::Plugin, Some(&parts.name), name_path).or_else(|| {
        id_issue(
            IdPart::Marketplace,
            parts.marketplace.as_deref(),
            marketplace_path,
        )
    }) {
        Some(issue) => Err(issue),
        None => Ok(parts),
    }
}

/// What the manifest reports for a string with two `@` or an object without `name` (spec J16.17,
/// oracle 2.1.325).
const INVALID_INPUT: &str = "Invalid input";

/// `name@marketplace` splits at the `@`; a string with a second `@` is invalid input (J16.17).
fn from_string(text: &str, path: &str) -> Result<Parts, PluginIssue> {
    let at = text.find('@');
    if at != text.rfind('@') {
        return Err(error(path, INVALID_INPUT));
    }
    let Some(at) = at else {
        if text.is_empty() {
            return Err(error(path, "A dependency name must not be empty"));
        }
        let parts = Parts {
            name: text.to_owned(),
            marketplace: None,
            version: None,
        };
        return check_parts(parts, path, path);
    };
    let name = text.get(..at).unwrap_or_default();
    let marketplace = text.get(at + 1..).unwrap_or_default();
    if name.is_empty() || marketplace.is_empty() {
        return Err(error(
            path,
            &format!("Dependency \"{text}\" must be \"name\" or \"name@marketplace\""),
        ));
    }
    let parts = Parts {
        name: name.to_owned(),
        marketplace: Some(marketplace.to_owned()),
        version: None,
    };
    check_parts(parts, path, path)
}

fn from_object(raw: &JsObject, path: &str) -> Result<Parts, PluginIssue> {
    let name = string_field(raw, "name", path);
    let marketplace = string_field(raw, "marketplace", path);
    let version = string_field(raw, "version", path);
    let (name, marketplace, version) = (name?, marketplace?, version?);
    let Some(name) = name else {
        return Err(error(path, INVALID_INPUT));
    };
    check_parts(
        Parts {
            name,
            marketplace,
            version,
        },
        &format!("{path}.name"),
        &format!("{path}.marketplace"),
    )
}

fn read_parts(value: &JsValue, path: &str) -> Result<Parts, PluginIssue> {
    if let JsValue::String(text) = value {
        return from_string(text, path);
    }
    let message = "A dependency must be a plugin name, \"name@marketplace\" or an object";
    value
        .as_object()
        .map_or_else(|| Err(error(path, message)), |raw| from_object(raw, path))
}

/// A dependency item read, or the issue that keeps it from being one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Normalized {
    /// The dependency, its defaults applied.
    Dependency(NormalizedDependency),
    /// What is wrong with the item.
    Issue(PluginIssue),
}

/// Reads one `dependencies` item. Judgments (spec/judgments.md): an empty or blank `version` is no
/// constraint (node-semver reads "" as any version); a key other than the three is ignored. The
/// `name` and the marketplace must each satisfy the plugin id grammar, as a marketplace entry's name
/// must, or they could never resolve. A string with two `@` and an object without `name` are
/// `Invalid input` on the item, as the manifest reports them (spec J16.17).
/// `declaring_marketplace` is the marketplace of the plugin that declares it, or `None` for a
/// plugin that has none (loaded from claude.ai), whose bare names then have no marketplace.
#[must_use]
pub fn normalize_dependency(
    value: &JsValue,
    declaring_marketplace: Option<&str>,
    path: Option<&str>,
) -> Normalized {
    let default_path = format!("{PATH}[0]");
    let path = path.unwrap_or(&default_path);
    let parts = match read_parts(value, path) {
        Ok(parts) => parts,
        Err(issue) => return Normalized::Issue(issue),
    };
    let marketplace = parts
        .marketplace
        .or_else(|| declaring_marketplace.map(str::to_owned));
    let range = parts
        .version
        .as_deref()
        .map(trim)
        .filter(|range| !range.is_empty())
        .map(str::to_owned);
    let dep = NormalizedDependency {
        name: parts.name,
        marketplace,
        range,
    };
    if let Some(range) = dep.range.as_deref().filter(|range| !is_valid_range(range)) {
        return Normalized::Issue(error(
            &format!("{path}.version"),
            &format!(
                "Dependency \"{}\" has an invalid version requirement: \"{range}\"",
                dependency_id(&dep)
            ),
        ));
    }
    Normalized::Dependency(dep)
}

/// Whether a `dependencies` item names a marketplace itself (`"name@marketplace"`, or an object
/// with `marketplace`), as opposed to resolving to the declaring plugin's. Normalizing hides it,
/// and it decides whether a `--plugin-dir` copy meets the entry on Claude Code before 2.1.242.
#[must_use]
pub fn names_marketplace(value: &JsValue) -> bool {
    if let JsValue::String(text) = value {
        return text.contains('@');
    }
    value.get("marketplace").is_some_and(JsValue::is_string)
}

/// A dependency as the host found it: its version, whether it is enabled, and whether it is a
/// `--plugin-dir` copy.
///
/// Its members are in the order `dependency-graph.ts` writes them in the copies it finds
/// (`enabled`, `local`, then `version`), which the package's interface lists otherwise.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledDependency {
    /// Whether it is enabled.
    pub enabled: bool,
    /// A copy loaded with `--plugin-dir`: it satisfies the entry and no range is checked against
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<bool>,
    /// The tag's version for a tag-resolved copy, else the `version` of its `plugin.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// The `errorDetails.type` `claude plugin list --json` carries for a dependency problem
/// (plugins/troubleshooting.md, "Dependency errors"): `dependency-unsatisfied` for a dependency that
/// is not installed (also with the `@synced` note) or disabled, `dependency-version-unsatisfied` for
/// `Requires "<dep>" <range>, installed <version>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyErrorType {
    /// `dependency-unsatisfied`.
    DependencyUnsatisfied,
    /// `dependency-version-unsatisfied`.
    DependencyVersionUnsatisfied,
}

/// What is wrong with the dependency: no copy meets it, its copy is off, or its version is
/// outside the range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyCause {
    /// `missing`.
    Missing,
    /// `disabled`.
    Disabled,
    /// `out-of-range`.
    OutOfRange,
}

/// A dependency problem, with the type the host reports it under and the facts a host words its
/// own advice from. Judgment (spec/judgments.md): `message` is the documented text; Claude Code
/// 2.1.322 adds `` — run `claude plugin install <dep>`, or check that its marketplace is added ``
/// to the `is not installed` line, advice for its own CLI, which the kit leaves to the host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyIssue {
    /// The issue itself.
    #[serde(flatten)]
    pub issue: PluginIssue,
    /// The type the host reports it under.
    pub error_type: DependencyErrorType,
    /// The declared dependency: `name@marketplace`, or `name` when it names no marketplace.
    pub dependency: String,
    /// What is wrong.
    pub cause: DependencyCause,
}

const fn cause_type(cause: DependencyCause) -> DependencyErrorType {
    match cause {
        DependencyCause::Missing | DependencyCause::Disabled => {
            DependencyErrorType::DependencyUnsatisfied
        }
        DependencyCause::OutOfRange => DependencyErrorType::DependencyVersionUnsatisfied,
    }
}

fn issue_for(dep: &NormalizedDependency, cause: DependencyCause, message: &str) -> DependencyIssue {
    let dependency = dependency_id(dep);
    DependencyIssue {
        issue: error(&format!("{PATH}.{dependency}"), message),
        error_type: cause_type(cause),
        dependency,
        cause,
    }
}

/// The version a copy states, as the troubleshooting message shows it.
fn shown(version: Option<&str>) -> &str {
    version.unwrap_or("(no version)")
}

/// The range check of one copy: no range, or a local copy, passes; no version satisfies no range.
fn range_issue(dep: &NormalizedDependency, copy: &InstalledDependency) -> Option<DependencyIssue> {
    let range = dep.range.as_deref()?;
    if copy.local == Some(true) {
        return None;
    }
    if copy
        .version
        .as_deref()
        .is_some_and(|version| satisfies_range(version, range))
    {
        return None;
    }
    Some(issue_for(
        dep,
        DependencyCause::OutOfRange,
        &format!(
            "Requires \"{}\" {range}, installed {}",
            dependency_id(dep),
            shown(copy.version.as_deref())
        ),
    ))
}

/// Whether a declared dependency is met by its installed copy (plugins/troubleshooting.md,
/// "Dependency errors"): `Dependency "<dep>" is not installed`, `... is disabled`, and
/// `Requires "<dep>" <range>, installed <version>`. A `--plugin-dir` copy meets the entry whatever
/// its version. The dependency is named as the entry declares it, a disabled `--plugin-dir` copy
/// too: not `<name>@inline` (spec J16.24, oracle 2.1.325).
#[must_use]
pub fn check_installed_dependency(
    dep: &NormalizedDependency,
    installed: Option<&InstalledDependency>,
) -> Option<DependencyIssue> {
    let Some(installed) = installed else {
        let message = format!("Dependency \"{}\" is not installed", dependency_id(dep));
        return Some(issue_for(dep, DependencyCause::Missing, &message));
    };
    if !installed.enabled {
        let message = format!(
            "Dependency \"{}\" is disabled — enable it or remove the dependency",
            dependency_id(dep)
        );
        return Some(issue_for(dep, DependencyCause::Disabled, &message));
    }
    range_issue(dep, installed)
}

/// The two places a dependency that names a marketplace can be found.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyCopies {
    /// Installed from the marketplace the entry names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace: Option<InstalledDependency>,
    /// The claude.ai plugin of the same name, `<name>@synced`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_ai: Option<InstalledDependency>,
}

fn synced_note(name: &str) -> String {
    format!(
        "\"{name}@synced\" is loaded from your claude.ai account but meets a marketplace dependency only for other claude.ai plugins"
    )
}

/// A claude.ai plugin meets the entry when it is enabled; a plugin that states no version passes
/// the range.
fn claude_ai_issue(
    dep: &NormalizedDependency,
    copy: &InstalledDependency,
) -> Option<DependencyIssue> {
    if !copy.enabled {
        return check_installed_dependency(dep, Some(copy));
    }
    copy.version.as_ref()?;
    range_issue(dep, copy)
}

/// Whether a dependency is met given both places it can be (plugins/dependencies.md, "Dependencies
/// when your plugin loads from claude.ai"). A plugin loaded from claude.ai (`loads_from_claude_ai`)
/// has a dependency that names a marketplace met by the copy installed from it and enabled (its
/// range checked against that copy), or by an enabled claude.ai plugin of the same name (its range
/// checked against its version; one that states none passes). For any other plugin a claude.ai copy
/// meets nothing, and the `is not installed` message says so (only while that plugin is enabled: a
/// disabled one is not loaded, troubleshooting.md "Dependency errors"). Each issue carries its
/// `errorType`.
#[must_use]
pub fn check_dependency(
    dep: &NormalizedDependency,
    found: &DependencyCopies,
    loads_from_claude_ai: bool,
) -> Option<DependencyIssue> {
    let from_marketplace = check_installed_dependency(dep, found.marketplace.as_ref())?;
    let Some(claude_ai) = &found.claude_ai else {
        return Some(from_marketplace);
    };
    if loads_from_claude_ai {
        return claude_ai_issue(dep, claude_ai).map(|_| from_marketplace);
    }
    if found.marketplace.is_none() && claude_ai.enabled {
        let message = format!(
            "{}: {}",
            from_marketplace.issue.message,
            synced_note(&dep.name)
        );
        return Some(issue_for(dep, from_marketplace.cause, &message));
    }
    Some(from_marketplace)
}

crate::registry::exports! {
    "marketplace/dependencies" {
        "dependencyId" => ok |dep: NormalizedDependency| dependency_id(&dep);
        "normalizeDependency" => ok |value: JsValue, declaring_marketplace: Option<String>, path: Option<String>| normalize_dependency(&value, declaring_marketplace.as_deref(), path.as_deref());
        "namesMarketplace" => ok |value: JsValue| names_marketplace(&value);
        "checkInstalledDependency" => ok |dep: NormalizedDependency, installed: Option<InstalledDependency>| check_installed_dependency(&dep, installed.as_ref());
        "checkDependency" => ok |dep: NormalizedDependency, found: DependencyCopies, loads_from_claude_ai: bool| check_dependency(&dep, &found, loads_from_claude_ai);
    }
}
