//! Port of `src/format/marketplace/marketplace-source.ts`, switched.
//!
//! Reading a marketplace source object (plugins/marketplace-reference.md, "Marketplace sources",
//! "Fields by type", "Source values valid only in policy lists", "Source objects in settings"): the
//! `source` of an `extraKnownMarketplaces` value, or an entry of `strictKnownMarketplaces` and
//! `blockedMarketplaces`. `policy` says which of the three keys the object is in. Issue paths are
//! relative to the source object (`repo`, `plugins[0].name`); the caller prefixes its own.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::Deserialize;

use super::marketplace_names::{NameOrigin, marketplace_name_issues};
use super::marketplace_schema::{read_owner, unknown_field};
use super::schema_issues::zod_issues;
use super::settings_source_plugins::read_settings_plugins;
use super::source_failures::policy_only_notice;
use super::types::{MarketplaceLoadStatus, MarketplaceSource, SourceParseResult};
use crate::format::issues::{PluginIssue, Severity, blocks_loading, error, warning};
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;
use crate::zod::{self, Schema, Shape, fields};

/// `MarketplaceSourceOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct MarketplaceSourceOptions {
    /// The object is an entry of `strictKnownMarketplaces` or `blockedMarketplaces`.
    pub policy: bool,
    /// The `extraKnownMarketplaces` key; a `settings` source's `name` must equal it.
    #[serde(default)]
    pub key: Option<String>,
}

/// What the `path` of a `github` or `git` source defaults to ("Fields by type").
pub const DEFAULT_MARKETPLACE_PATH: &str = ".claude-plugin/marketplace.json";

#[derive(Default)]
struct Built {
    source: Option<MarketplaceSource>,
    issues: Vec<PluginIssue>,
    status: Option<MarketplaceLoadStatus>,
}

/// A reader's answer: `Built & { fieldIssues }`.
#[derive(Default)]
struct Read {
    built: Built,
    field_issues: Vec<PluginIssue>,
}

static TEXT: LazyLock<Schema> = LazyLock::new(zod::string);
static FLAG: LazyLock<Schema> = LazyLock::new(zod::boolean);
static TEXT_LIST: LazyLock<Schema> = LazyLock::new(|| zod::array(zod::string()));
static HEADER_MAP: LazyLock<Schema> = LazyLock::new(|| zod::record(zod::string(), zod::string()));

/// A required key that is absent is `Invalid input`; a present one is read by its own schema.
fn read_fields(shape: &Shape, raw: &JsObject) -> (JsObject, Vec<PluginIssue>) {
    let mut kept = JsObject::new();
    let mut issues = Vec::new();
    for (key, schema) in shape {
        let value = raw.get(key).unwrap_or(&JsValue::Undefined);
        if value.is_undefined() {
            if schema.safe_parse(&JsValue::Undefined).is_err() {
                issues.push(error(key, "Invalid input"));
            }
            continue;
        }
        match schema.safe_parse(value) {
            Ok(data) => {
                kept.insert(key.clone(), data);
            }
            Err(failure) => issues.extend(zod_issues(&failure, key)),
        }
    }
    (kept, issues)
}

/// A key the type does not have: "Claude Code ignores it at load time".
fn unknown_keys(shape: &Shape, raw: &JsObject) -> Vec<PluginIssue> {
    let known: Vec<&str> = shape.keys().map(String::as_str).collect();
    property_order(raw)
        .into_iter()
        .filter(|(key, _)| key.as_str() != "source" && !shape.contains_key(key.as_str()))
        .map(|(key, _)| unknown_field(key, key, &known))
        .collect()
}

/// The parsed fields of a source object, as each reader's `build` reads them.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fields {
    url: Option<String>,
    headers: Option<IndexMap<String, String>>,
    headers_helper: Option<String>,
    repo: Option<String>,
    #[serde(rename = "ref")]
    reference: Option<String>,
    path: Option<String>,
    sparse_paths: Option<Vec<String>>,
    skip_lfs: Option<bool>,
    package: Option<String>,
    version: Option<String>,
    registry: Option<String>,
    host_pattern: Option<String>,
    path_pattern: Option<String>,
}

type Build = fn(Fields, &MarketplaceSourceOptions) -> Built;

/// One reader per type: the schema of its fields, and what the parsed fields build.
fn reader(
    schema: &Schema,
    build: Build,
    raw: &JsObject,
    options: &MarketplaceSourceOptions,
) -> Read {
    let empty = Shape::new();
    let shape = schema.shape().unwrap_or(&empty);
    let (kept, issues) = read_fields(shape, raw);
    let unknown = unknown_keys(shape, raw);
    let whole = schema.safe_parse(&JsValue::Object(kept));
    let parsed = whole.ok().and_then(|data| Fields::deserialize(data).ok());
    let Some(fields) = parsed.filter(|_| issues.is_empty()) else {
        return Read {
            built: Built::default(),
            field_issues: [issues, unknown].concat(),
        };
    };
    let built = build(fields, options);
    let field_issues = [unknown, built.issues.clone()].concat();
    Read {
        built,
        field_issues,
    }
}

const WILDCARD_WARNING: &str = "The owner wildcard \"<owner>/*\" is valid only in strictKnownMarketplaces and blockedMarketplaces; here Claude Code takes it literally and the clone fails";

/// `owner/*` outside a policy list names no single repository ("Fields by type", `repo`).
fn repo_built(repo: &str, policy: bool) -> Built {
    if policy || !repo.ends_with("/*") {
        return Built::default();
    }
    Built {
        source: None,
        issues: vec![warning("repo", WILDCARD_WARNING)],
        status: Some(MarketplaceLoadStatus::FailsOwnerWildcard),
    }
}

fn git_fields() -> Shape {
    fields! {
        "ref" => TEXT.optional(),
        "path" => TEXT.optional(),
        "sparsePaths" => TEXT_LIST.optional(),
        "skipLfs" => FLAG.optional(),
    }
}

/// The settings source: its name, its inline catalog and its owner.
fn settings_built(fields: &JsValue, raw: &JsObject, options: &MarketplaceSourceOptions) -> Built {
    let name = fields
        .get("name")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    let name_issues = match &options.key {
        Some(key) if key != name => vec![error(
            "name",
            &format!(
                "Marketplace name \"{name}\" must equal the extraKnownMarketplaces key \"{key}\""
            ),
        )],
        _ => Vec::new(),
    };
    let reserved: Vec<PluginIssue> = marketplace_name_issues(name, &NameOrigin::default())
        .into_iter()
        .filter(|found| found.severity == Severity::Error)
        .collect();
    let owner = fields
        .get("owner")
        .filter(|owner| !owner.is_undefined())
        .map(|_| read_owner(raw.get("owner").unwrap_or(&JsValue::Undefined)));
    let plugins = fields
        .get("plugins")
        .and_then(JsValue::as_array)
        .map_or(&[][..], Vec::as_slice);
    let catalog = read_settings_plugins(plugins);
    let owner_issues = owner
        .as_ref()
        .map(|owner| owner.issues.clone())
        .unwrap_or_default();
    let blocking = [name_issues, reserved, owner_issues].concat();
    let issues = [blocking.clone(), catalog.issues].concat();
    if blocks_loading(&blocking) {
        return Built {
            issues,
            ..Built::default()
        };
    }
    Built {
        issues,
        source: Some(MarketplaceSource::Settings {
            name: name.to_owned(),
            plugins: catalog.plugins,
            owner: owner.and_then(|owner| owner.value),
        }),
        status: None,
    }
}

static SETTINGS_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "name" => TEXT.clone(),
        "plugins" => zod::array(zod::unknown()),
        "owner" => zod::unknown().optional(),
    })
});

const fn built(source: MarketplaceSource) -> Built {
    Built {
        source: Some(source),
        issues: Vec::new(),
        status: None,
    }
}

fn path_or_default(path: Option<String>) -> String {
    path.unwrap_or_else(|| DEFAULT_MARKETPLACE_PATH.to_owned())
}

static URL: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "url" => TEXT.clone(),
        "headers" => HEADER_MAP.optional(),
        "headersHelper" => TEXT.optional(),
    })
});
static GITHUB: LazyLock<Schema> =
    LazyLock::new(|| zod::object(fields! { "repo" => TEXT.clone(), ..git_fields() }));
static GIT: LazyLock<Schema> =
    LazyLock::new(|| zod::object(fields! { "url" => TEXT.clone(), ..git_fields() }));
// `version` and `registry` are the SDK's shape of an npm source, not the docs' table (spec/judgments.md).
static NPM: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "package" => TEXT.clone(),
        "version" => TEXT.optional(),
        "registry" => TEXT.optional(),
    })
});
static PATH: LazyLock<Schema> = LazyLock::new(|| zod::object(fields! { "path" => TEXT.clone() }));
static NONE: LazyLock<Schema> = LazyLock::new(|| zod::object(Shape::new()));
static HOST_PATTERN: LazyLock<Schema> =
    LazyLock::new(|| zod::object(fields! { "hostPattern" => TEXT.clone() }));
static PATH_PATTERN: LazyLock<Schema> =
    LazyLock::new(|| zod::object(fields! { "pathPattern" => TEXT.clone() }));

fn url_built(fields: Fields, _: &MarketplaceSourceOptions) -> Built {
    built(MarketplaceSource::Url {
        url: fields.url.unwrap_or_default(),
        headers: fields.headers,
        headers_helper: fields.headers_helper,
    })
}

fn github_built(fields: Fields, options: &MarketplaceSourceOptions) -> Built {
    let repo = fields.repo.unwrap_or_default();
    let checked = repo_built(&repo, options.policy);
    Built {
        source: Some(MarketplaceSource::Github {
            repo,
            reference: fields.reference,
            path: path_or_default(fields.path),
            sparse_paths: fields.sparse_paths,
            skip_lfs: fields.skip_lfs,
        }),
        ..checked
    }
}

fn git_built(fields: Fields, _: &MarketplaceSourceOptions) -> Built {
    built(MarketplaceSource::Git {
        url: fields.url.unwrap_or_default(),
        reference: fields.reference,
        path: path_or_default(fields.path),
        sparse_paths: fields.sparse_paths,
        skip_lfs: fields.skip_lfs,
    })
}

fn npm_built(fields: Fields, _: &MarketplaceSourceOptions) -> Built {
    built(MarketplaceSource::Npm {
        package: fields.package.unwrap_or_default(),
        version: fields.version,
        registry: fields.registry,
    })
}

fn file_built(fields: Fields, _: &MarketplaceSourceOptions) -> Built {
    built(MarketplaceSource::File {
        path: fields.path.unwrap_or_default(),
    })
}

fn directory_built(fields: Fields, _: &MarketplaceSourceOptions) -> Built {
    built(MarketplaceSource::Directory {
        path: fields.path.unwrap_or_default(),
    })
}

fn skills_dir_built(_: Fields, _: &MarketplaceSourceOptions) -> Built {
    built(MarketplaceSource::SkillsDir {})
}

fn host_pattern_built(fields: Fields, _: &MarketplaceSourceOptions) -> Built {
    built(MarketplaceSource::HostPattern {
        host_pattern: fields.host_pattern.unwrap_or_default(),
    })
}

fn path_pattern_built(fields: Fields, _: &MarketplaceSourceOptions) -> Built {
    built(MarketplaceSource::PathPattern {
        path_pattern: fields.path_pattern.unwrap_or_default(),
    })
}

/// The settings reader: its fields as the others', built with the raw object for its owner.
fn settings_reader(raw: &JsObject, options: &MarketplaceSourceOptions) -> Read {
    let empty = Shape::new();
    let shape = SETTINGS_SCHEMA.shape().unwrap_or(&empty);
    let (kept, issues) = read_fields(shape, raw);
    let unknown = unknown_keys(shape, raw);
    let whole = SETTINGS_SCHEMA.safe_parse(&JsValue::Object(kept));
    let Some(fields) = whole.ok().filter(|_| issues.is_empty()) else {
        return Read {
            built: Built::default(),
            field_issues: [issues, unknown].concat(),
        };
    };
    let built = settings_built(&fields, raw, options);
    let field_issues = [unknown, built.issues.clone()].concat();
    Read {
        built,
        field_issues,
    }
}

/// `READERS[kind](raw, options)`, or `None` for a tag that names no type (`isKind`).
fn read_kind(kind: &str, raw: &JsObject, options: &MarketplaceSourceOptions) -> Option<Read> {
    let (schema, build): (&Schema, Build) = match kind {
        "url" => (&URL, url_built),
        "github" => (&GITHUB, github_built),
        "git" => (&GIT, git_built),
        "npm" => (&NPM, npm_built),
        "file" => (&PATH, file_built),
        "directory" => (&PATH, directory_built),
        "settings" => return Some(settings_reader(raw, options)),
        "skills-dir" => (&NONE, skills_dir_built),
        "hostPattern" => (&HOST_PATTERN, host_pattern_built),
        "pathPattern" => (&PATH_PATTERN, path_pattern_built),
        _ => return None,
    };
    Some(reader(schema, build, raw, options))
}

/// What `extraKnownMarketplaces` does with each type ("Marketplace sources", the table).
fn load_status(kind: &str) -> MarketplaceLoadStatus {
    match kind {
        "npm" => MarketplaceLoadStatus::FailsNpm,
        "skills-dir" | "hostPattern" | "pathPattern" => MarketplaceLoadStatus::FailsPolicyOnly,
        _ => MarketplaceLoadStatus::Loads,
    }
}

/// Types a policy list takes and `extraKnownMarketplaces` cannot load: the notice outside a policy list.
fn outside_policy(kind: &str) -> Option<String> {
    match kind {
        "npm" => Some(format!(
            "An npm marketplace source fails to load in extraKnownMarketplaces: {}",
            super::source_failures::SOURCE_FAILURE_NPM
        )),
        "skills-dir" | "hostPattern" | "pathPattern" => Some(policy_only_notice(kind)),
        _ => None,
    }
}

/// A source that matches no type: `Invalid input`, and nothing Claude Code can load.
fn invalid() -> SourceParseResult {
    SourceParseResult {
        source: None,
        issues: vec![error("source", "Invalid input")],
        load_status: MarketplaceLoadStatus::FailsPolicyOnly,
    }
}

/// The marketplace source an object describes. `loadStatus` is what `extraKnownMarketplaces` does
/// with it, whichever key it is in: a type Claude Code parses can still fail to load, and an object
/// that matches no source type (`source` is undefined) loads nothing. Never throws.
#[must_use]
pub fn parse_marketplace_source(
    raw: &JsValue,
    options: &MarketplaceSourceOptions,
) -> SourceParseResult {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return invalid();
    };
    let Some(kind) = members.get("source").and_then(JsValue::as_str) else {
        return invalid();
    };
    let Some(read) = read_kind(kind, members, options) else {
        return invalid();
    };
    let notice = if options.policy {
        None
    } else {
        outside_policy(kind)
    };
    let mut issues = read.field_issues;
    if let (Some(notice), Some(_)) = (notice, &read.built.source) {
        issues.push(warning("source", &notice));
    }
    let load_status = if read.built.source.is_none() {
        invalid().load_status
    } else {
        read.built.status.unwrap_or_else(|| load_status(kind))
    };
    SourceParseResult {
        source: read.built.source,
        issues,
        load_status,
    }
}

crate::registry::exports! {
    "format/marketplace/marketplace-source" {
        "DEFAULT_MARKETPLACE_PATH" => ok || DEFAULT_MARKETPLACE_PATH;
        "parseMarketplaceSource" => ok |raw: JsValue, options: MarketplaceSourceOptions|
            parse_marketplace_source(&raw, &options);
    }
}
