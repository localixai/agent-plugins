//! Port of `src/format/core/manifest.ts`, switched.
//!
//! A core manifest, root `plugin.json` (core §5): a closed object of eleven fields. An unknown
//! top-level field is reported and ignored (core §5.2); every other violation (a missing or bad
//! `$schema` or `name`, a field of the wrong type, an `author` with another member) rejects the
//! package before any component is discovered (core §5.2, §5.3, §11.3 item 2). Metadata is checked by
//! JSON type alone: a `version` that is not SemVer or a `homepage` that is no URL is accepted (core
//! §5.4).

use serde::{Deserialize, Serialize};

use super::extensions::{CoreExtensions, extensions_problem, implemented_extensions};
use super::name::core_name_problems;
use super::versions::{
    CORE_VERSIONS, CoreDocument, CoreVersion, core_schema_version, supported_core_version,
};
use crate::format::issues::{PluginIssue, blocks_loading, error, warning};
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;

/// The author of a core manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreAuthor {
    /// The author's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The author's email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// The author's url.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A core manifest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreManifest {
    /// The canonical schema identifier.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// The plugin's name.
    pub name: String,
    /// The name to show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// What the plugin does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Who wrote it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<CoreAuthor>,
    /// Its home page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// Its repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Its license.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Its keywords.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    /// Only the namespaces the host implements (core §8.1); absent when none is there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<CoreExtensions>,
}

/// What reading a core manifest gave.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreManifestResult {
    /// `None` when the manifest rejects the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<CoreManifest>,
    /// The Agent Plugins version the manifest targets, when the kit implements it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<CoreVersion>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// The type check of a field that holds a string.
fn string(value: &JsValue) -> Option<String> {
    if value.is_string() {
        None
    } else {
        Some("must be a string".to_owned())
    }
}

const AUTHOR_FIELDS: [&str; 3] = ["name", "email", "url"];

fn author_problem(value: &JsValue) -> Option<String> {
    let JsValue::Object(members) = value else {
        return Some("must be an object".to_owned());
    };
    let entries = property_order(members);
    if let Some((key, _)) = entries
        .iter()
        .find(|(one, _)| !AUTHOR_FIELDS.contains(&one.as_str()))
    {
        return Some(format!(
            "may hold only \"name\", \"email\" and \"url\", not \"{key}\""
        ));
    }
    entries
        .iter()
        .find(|(_, member)| !member.is_string())
        .map(|(key, _)| format!("\"{key}\" must be a string"))
}

fn keywords_problem(value: &JsValue) -> Option<String> {
    if value
        .as_array()
        .is_some_and(|items| items.iter().all(JsValue::is_string))
    {
        None
    } else {
        Some("must be an array of strings".to_owned())
    }
}

/// The type check of each optional field (core §5.4, §5.6): a registry over the closed field set.
type FieldProblem = fn(&JsValue) -> Option<String>;

const FIELD_PROBLEMS: [(&str, FieldProblem); 9] = [
    ("displayName", string),
    ("version", string),
    ("description", string),
    ("author", author_problem),
    ("homepage", string),
    ("repository", string),
    ("license", string),
    ("keywords", keywords_problem),
    ("extensions", extensions_problem),
];

/// `Object.hasOwn(FIELD_PROBLEMS, key)`.
fn field_problem(key: &str) -> Option<fn(&JsValue) -> Option<String>> {
    FIELD_PROBLEMS
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, check)| *check)
}

/// What the `$schema` step gave: the version it targets, or why the package is rejected.
struct SchemaStep {
    version: Option<CoreVersion>,
    issue: Option<PluginIssue>,
}

/// The `$schema` step: the version it targets, or why the package is rejected (core §5.2).
fn schema_step(value: &JsValue) -> SchemaStep {
    let rejected = |issue| SchemaStep {
        version: None,
        issue: Some(issue),
    };
    if value.is_undefined() {
        return rejected(error("$schema", "The core manifest has no \"$schema\""));
    }
    let Some(named) = core_schema_version(value, CoreDocument::Plugin) else {
        return rejected(error(
            "$schema",
            "Invalid \"$schema\": not a canonical core manifest identifier",
        ));
    };
    match supported_core_version(Some(&named)) {
        Some(version) => SchemaStep {
            version: Some(version),
            issue: None,
        },
        None => {
            let implemented: Vec<&str> = CORE_VERSIONS.iter().map(|known| known.as_str()).collect();
            rejected(error(
                "$schema",
                &format!(
                    "Unsupported Agent Plugins version {named}: this client implements {}",
                    implemented.join(", ")
                ),
            ))
        }
    }
}

fn name_issue(value: &JsValue) -> Option<PluginIssue> {
    if value.is_undefined() {
        return Some(error("name", "The core manifest has no \"name\""));
    }
    let Some(name) = value.as_str() else {
        return Some(error("name", "Invalid \"name\": must be a string"));
    };
    let problems = core_name_problems(name);
    if problems.is_empty() {
        None
    } else {
        Some(error(
            "name",
            &format!("Invalid \"name\" \"{name}\": {}", problems.join("; ")),
        ))
    }
}

/// Each other top-level member: a warning for an unknown one, an error for a bad value.
fn member_issues(raw: &JsObject) -> Vec<PluginIssue> {
    property_order(raw)
        .into_iter()
        .filter_map(|(key, value)| {
            if key == "$schema" || key == "name" {
                return None;
            }
            let Some(check) = field_problem(key) else {
                return Some(warning(
                    key,
                    &format!("Unknown field \"{key}\": ignored (core §5.2)"),
                ));
            };
            check(value).map(|problem| error(key, &format!("Invalid \"{key}\": {problem}")))
        })
        .collect()
}

fn text(value: Option<&JsValue>) -> Option<String> {
    value.and_then(JsValue::as_str).map(str::to_owned)
}

fn author_of(value: Option<&JsValue>) -> Option<CoreAuthor> {
    let record = value.filter(|one| is_record(one))?;
    Some(CoreAuthor {
        name: text(record.get("name")),
        email: text(record.get("email")),
        url: text(record.get("url")),
    })
}

fn keywords_of(value: Option<&JsValue>) -> Option<Vec<String>> {
    value.and_then(JsValue::as_array).map(|items| {
        items
            .iter()
            .filter_map(|one| one.as_str().map(str::to_owned))
            .collect()
    })
}

/// The manifest's known fields, once they passed their checks; extensions narrowed to what the host implements.
fn kept(raw: &JsObject, implemented: &[String]) -> CoreManifest {
    let field = |key: &str| raw.get(key);
    CoreManifest {
        schema: text(field("$schema")).unwrap_or_default(),
        name: text(field("name")).unwrap_or_default(),
        display_name: text(field("displayName")),
        version: text(field("version")),
        description: text(field("description")),
        author: author_of(field("author")),
        homepage: text(field("homepage")),
        repository: text(field("repository")),
        license: text(field("license")),
        keywords: keywords_of(field("keywords")),
        extensions: implemented_extensions(
            field("extensions").unwrap_or(&JsValue::Undefined),
            implemented,
        ),
    }
}

/// Reads a parsed root `plugin.json` by core §5. `implemented` names the extension namespaces the
/// host implements; the others are dropped without being looked at (core §8.1).
#[must_use]
pub fn read_core_manifest(raw: &JsValue, implemented: &[String]) -> CoreManifestResult {
    let JsValue::Object(members) = raw else {
        return CoreManifestResult {
            manifest: None,
            version: None,
            issues: vec![error(
                "plugin.json",
                "The core manifest is not a JSON object",
            )],
        };
    };
    let schema = schema_step(members.get("$schema").unwrap_or(&JsValue::Undefined));
    let name = name_issue(members.get("name").unwrap_or(&JsValue::Undefined));
    let issues: Vec<PluginIssue> = [schema.issue, name]
        .into_iter()
        .flatten()
        .chain(member_issues(members))
        .collect();
    let fatal = blocks_loading(&issues);
    CoreManifestResult {
        manifest: if fatal {
            None
        } else {
            Some(kept(members, implemented))
        },
        version: schema.version,
        issues,
    }
}

crate::registry::exports! {
    "format/core/manifest" {
        "readCoreManifest" => ok |raw: JsValue, implemented: Option<Vec<String>>| read_core_manifest(&raw, &implemented.unwrap_or_default());
    }
}
