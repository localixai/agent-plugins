//! Port of `src/marketplace/node-deps-lockfile.ts`, switched.
//!
//! The facts of a lockfile the dependency install checks (plugins/loading.md, "Limits on the
//! dependency install": a registry package "pinned in the lockfile to an exact version", an `https`
//! download link, the same dependencies in `package.json` and the lockfile; "When the dependency
//! install runs": `lockfileVersion`). The docs name only `lockfileVersion`; the rest of each layout
//! is npm's and Bun's own and is read as little as the checks need (spec/judgments.md).

use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::node_deps_spec::{
    DEPENDENCY_SECTIONS, is_registry_id, is_registry_origin, read_dependency_map,
};
use crate::js::json_parse::json_parse;
use crate::js::{JsObject, JsValue, Pattern, is_whitespace, property_order};
use crate::protocol::is_record::is_record;

/// The lockfiles the install reads, in the order Claude Code checks them: "the first match".
pub const LOCKFILE_NAMES: [&str; 3] = ["bun.lock", "npm-shrinkwrap.json", "package-lock.json"];

/// One of `LOCKFILE_NAMES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockfileName {
    /// `bun.lock`.
    #[serde(rename = "bun.lock")]
    BunLock,
    /// `npm-shrinkwrap.json`.
    #[serde(rename = "npm-shrinkwrap.json")]
    NpmShrinkwrap,
    /// `package-lock.json`.
    #[serde(rename = "package-lock.json")]
    PackageLock,
}

/// A package the lockfile pins to something other than a registry version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedOrigin {
    /// The package.
    pub name: String,
    /// What it is pinned to, in the specifier forms of `package.json` (`link:../a`, `git+ssh://...`).
    pub origin: String,
}

/// What the lockfile records the root package as depending on, by section.
pub type RootDependencies = IndexMap<String, IndexMap<String, String>>;

/// What the checks read of a lockfile.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LockfileFacts {
    /// `lockfileVersion`; `undefined` when the file is not readable or has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<f64>,
    /// What the lockfile records the root package as depending on.
    pub root: RootDependencies,
    /// Packages pinned to a non-registry origin.
    pub locked: Vec<LockedOrigin>,
    /// Every download link the lockfile holds, whatever its scheme.
    pub downloads: Vec<String>,
    /// Registry packages not pinned to an exact version, each as `name@version` (the name alone when none is given).
    pub unpinned: Vec<String>,
}

fn unreadable() -> LockfileFacts {
    LockfileFacts::default()
}

fn root_of(entry: Option<&JsValue>) -> RootDependencies {
    let Some(entry) = entry.filter(|entry| is_record(entry)) else {
        return IndexMap::new();
    };
    DEPENDENCY_SECTIONS
        .iter()
        .filter_map(|section| {
            let map = read_dependency_map(entry.get(section).unwrap_or(&JsValue::Undefined));
            (!map.is_empty()).then(|| ((*section).to_owned(), map))
        })
        .collect()
}

fn version_of(parsed: &JsObject) -> Option<f64> {
    parsed.get("lockfileVersion").and_then(JsValue::as_f64)
}

#[derive(Default)]
struct Found {
    locked: Vec<LockedOrigin>,
    downloads: Vec<String>,
    unpinned: Vec<String>,
}

/// An exact version: `1.3.0`, `2.0.0-beta.1`, `1.0.0+build`; not a range, a tag or a link.
static EXACT: Pattern = LazyLock::new(|| {
    Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$").ok()
});

fn is_exact(version: &str) -> bool {
    EXACT.as_ref().is_some_and(|regex| regex.is_match(version))
}

fn pinned_label(name: &str, version: Option<&JsValue>) -> String {
    match version.and_then(JsValue::as_str) {
        Some(version) => format!("{name}@{version}"),
        None => name.to_owned(),
    }
}

// ---- npm ----

const MODULES: &str = "node_modules/";

/// `node_modules/a/node_modules/@s/b` is the package `@s/b`.
fn npm_name_of(key: &str) -> String {
    // `lastIndexOf` is -1 when absent, so the name starts at `MODULES.length - 1`.
    let start = key
        .rfind(MODULES)
        .map_or(MODULES.len() - 1, |at| at + MODULES.len());
    key.get(start..).unwrap_or_default().to_owned()
}

fn text_member<'a>(entry: &'a JsObject, key: &str) -> Option<&'a str> {
    entry.get(key).and_then(JsValue::as_str)
}

/// An npm entry is a locked origin (a link, or a `resolved` that is no registry download), else a
/// download link of any scheme, whose `https` is checked later; a registry entry must carry an
/// exact `version`.
fn place_npm(key: &str, entry: &JsObject, into: &mut Found) {
    let name = npm_name_of(key);
    if entry.get("link").and_then(JsValue::as_bool) == Some(true) {
        into.locked.push(LockedOrigin {
            name,
            origin: format!(
                "link:{}",
                text_member(entry, "resolved").unwrap_or_default()
            ),
        });
        return;
    }
    if let Some(resolved) = text_member(entry, "resolved") {
        if is_registry_origin(resolved) {
            into.downloads.push(resolved.to_owned());
        } else {
            into.locked.push(LockedOrigin {
                name: name.clone(),
                origin: resolved.to_owned(),
            });
        }
    }
    if !text_member(entry, "version").is_some_and(is_exact) {
        into.unpinned
            .push(pinned_label(&name, entry.get("version")));
    }
}

fn record_member<'a>(parsed: &'a JsObject, key: &str) -> Option<&'a JsObject> {
    parsed.get(key).and_then(JsValue::as_object)
}

fn read_npm(parsed: &JsObject) -> LockfileFacts {
    let empty = JsObject::new();
    let packages = record_member(parsed, "packages").unwrap_or(&empty);
    let mut into = Found::default();
    for (key, entry) in property_order(packages) {
        if let Some(entry) = entry.as_object().filter(|_| !key.is_empty()) {
            place_npm(key, entry, &mut into);
        }
    }
    LockfileFacts {
        version: version_of(parsed),
        root: root_of(packages.get("")),
        locked: into.locked,
        downloads: into.downloads,
        unpinned: into.unpinned,
    }
}

// ---- bun.lock ----

/// `/^\s*[}\]]/` on what follows a comma.
fn closes_after(rest: &str) -> bool {
    rest.trim_start_matches(is_whitespace)
        .starts_with(['}', ']'])
}

/// `bun.lock` is JSON with trailing commas; drop each comma that closes an object or an array.
fn without_trailing_commas(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        let rest = text.get(index + character.len_utf8()..).unwrap_or_default();
        let closes = !in_string && character == ',' && closes_after(rest);
        if !closes {
            out.push(character);
        }
        if in_string {
            in_string = escaped || character != '"';
            escaped = !escaped && character == '\\';
        } else {
            in_string = character == '"';
        }
    }
    out
}

/// `name@spec` where a scoped name starts with `@`.
fn split_id(id: &str) -> Option<(String, String)> {
    let first = id.chars().next().map_or(0, char::len_utf8);
    let at = id.get(first..)?.find('@')? + first;
    Some((id.get(..at)?.to_owned(), id.get(at + 1..)?.to_owned()))
}

/// A `bun.lock` id is `name@specifier`: a registry package when the specifier is a registry version
/// (or an `npm:` alias), which must be exact; anything else, a tarball link included, is a locked
/// origin. Only the entry's registry URL is a download link.
fn place_bun(entry: &[JsValue], into: &mut Found) {
    let id = entry.first().and_then(JsValue::as_str).and_then(split_id);
    if let Some((name, spec)) = &id {
        if !is_registry_id(spec) {
            into.locked.push(LockedOrigin {
                name: name.clone(),
                origin: spec.clone(),
            });
        } else {
            let version = spec
                .rfind('@')
                .map_or(spec.as_str(), |at| spec.get(at + 1..).unwrap_or_default());
            if !is_exact(version) {
                into.unpinned.push(format!("{name}@{spec}"));
            }
        }
    }
    if let Some(link) = entry
        .get(1)
        .and_then(JsValue::as_str)
        .filter(|link| !link.is_empty())
    {
        into.downloads.push(link.to_owned());
    }
}

fn read_bun(parsed: &JsObject) -> LockfileFacts {
    let empty = JsObject::new();
    let workspaces = record_member(parsed, "workspaces").unwrap_or(&empty);
    let packages = record_member(parsed, "packages").unwrap_or(&empty);
    let mut into = Found::default();
    for (_, entry) in property_order(packages) {
        if let Some(entry) = entry.as_array() {
            place_bun(entry, &mut into);
        }
    }
    LockfileFacts {
        version: version_of(parsed),
        root: root_of(workspaces.get("")),
        locked: into.locked,
        downloads: into.downloads,
        unpinned: into.unpinned,
    }
}

struct Reader {
    prepare: fn(&str) -> String,
    read: fn(&JsObject) -> LockfileFacts,
}

fn as_is(text: &str) -> String {
    text.to_owned()
}

const fn readers(name: LockfileName) -> Reader {
    match name {
        LockfileName::BunLock => Reader {
            prepare: without_trailing_commas,
            read: read_bun,
        },
        LockfileName::NpmShrinkwrap | LockfileName::PackageLock => Reader {
            prepare: as_is,
            read: read_npm,
        },
    }
}

/// The facts of a lockfile's text; text that is not JSON has no version, so no install runs from it.
#[must_use]
pub fn read_lockfile(name: LockfileName, text: &str) -> LockfileFacts {
    let reader = readers(name);
    match json_parse(&(reader.prepare)(text)) {
        Ok(JsValue::Object(parsed)) => (reader.read)(&parsed),
        Ok(_) | Err(_) => unreadable(),
    }
}

crate::registry::exports! {
    "marketplace/node-deps-lockfile" {
        "LOCKFILE_NAMES" => ok || LOCKFILE_NAMES;
        "readLockfile" => ok |name: LockfileName, text: String| read_lockfile(name, &text);
    }
}
