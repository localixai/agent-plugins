//! Port of `src/host/session-folder-resolution.ts`, switched.
//!
//! The folder skills a session lists: the scanned entries the source policy lets load, resolved to
//! their names (`resolveFolderSkills`). The memo stays at the edge, invalidated by scans,
//! `host.update` and a change of `taken` (a new array: the host's skills changed), as it compares
//! the host's state object and the array it was given; each resolution is one crossing.

use serde::Serialize;

use super::session_skill_entries::{SkillPlace, place_of};
use crate::format::skill_types::BodyReader;
use crate::js::{JsObject, JsValue};
use crate::registry::Set;
use crate::settings::customization::sources::loads;
use crate::skills::scan::locations::{ScanScope, scope_rule};
use crate::skills::scan::resolve::{
    Resolution, ResolveFolderOptions, ShadowedEntry, YieldedName, resolve_folder_skills,
};
use crate::skills::scan::scan_types::ScannedEntry;

/// A folder skill's facts for the host's list (`FolderSkillSource`).
#[derive(Clone, Debug, Serialize)]
pub struct FolderSkillSource {
    /// The listed command.
    pub command: String,
    /// Every name that invokes it.
    pub names: Vec<String>,
    /// The scope it loaded from.
    pub scope: ScanScope,
    /// `invocable.origin`.
    pub origin: JsValue,
    /// The names it preferred that another command already used.
    pub yielded: Vec<YieldedName>,
}

/// The folder skills' facts and the entries that lost their names (`FolderSkillSources`).
#[derive(Clone, Debug, Default, Serialize)]
pub struct FolderSkillSources {
    /// One per listed entry.
    pub entries: Vec<FolderSkillSource>,
    /// The entries that lost their name to another.
    pub shadowed: Vec<ShadowedEntry>,
}

/// One folder entry (`Entry` with `source: { kind: 'folder', scope }`).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntry {
    /// The skill or command without its body.
    pub invocable: JsObject,
    /// Every name that invokes it, the listed command first.
    pub names: Vec<String>,
    /// The scan's reader.
    pub read_body: BodyReader,
    /// `{ kind: 'folder', scope }`.
    pub source: JsValue,
    /// Where it lives.
    #[serde(flatten)]
    pub place: SkillPlace,
}

/// What one resolution answers: the entries listed and their sources.
#[derive(Clone, Debug, Serialize)]
pub struct FolderResolution {
    /// The entries, in the resolution's order.
    pub listed: Vec<FolderEntry>,
    /// Their facts for the host's list.
    pub sources: FolderSkillSources,
}

fn text_of(value: Option<&JsValue>) -> String {
    value
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn sources_of(resolution: &Resolution) -> FolderSkillSources {
    FolderSkillSources {
        entries: resolution
            .resolved
            .iter()
            .map(|each| FolderSkillSource {
                command: text_of(each.invocable.get("command")),
                names: each.names.clone(),
                scope: each.scope,
                origin: each.invocable.get("origin").cloned().unwrap_or_default(),
                yielded: each.yielded.clone(),
            })
            .collect(),
        shadowed: resolution.shadowed.clone(),
    }
}

fn folder_source(scope: ScanScope) -> JsValue {
    let mut source = JsObject::new();
    source.insert("kind".to_owned(), "folder".into());
    source.insert(
        "scope".to_owned(),
        crate::js::value::to_js_value(&scope).unwrap_or_default(),
    );
    JsValue::Object(source)
}

/// Whether the source policy lets an entry load: `SCOPE_RULES[scope][skills | commands]`, asked of
/// `policy.loads('skills', source.origin)` (the policy's `locked` surfaces).
fn allowed(entry: &ScannedEntry, locked: &[String]) -> bool {
    let rule = scope_rule(entry.scope);
    let kind = entry.invocable.get("kind").and_then(JsValue::as_str);
    let source = if kind == Some("command") {
        rule.commands
    } else {
        rule.skills
    };
    source.is_some_and(|source| loads(locked, "skills", &JsValue::from(source.origin)))
}

/// The entries the policy lets load, resolved to their names.
#[must_use]
pub fn resolve_folder_entries(
    scanned: &[ScannedEntry],
    locked: &Set<String>,
    options: &ResolveFolderOptions,
) -> FolderResolution {
    let allowed: Vec<ScannedEntry> = scanned
        .iter()
        .filter(|entry| allowed(entry, &locked.members))
        .cloned()
        .collect();
    let resolution = resolve_folder_skills(&allowed, options);
    let sources = sources_of(&resolution);
    let listed = resolution
        .resolved
        .into_iter()
        .map(|each| FolderEntry {
            place: place_of(&each.invocable),
            names: each.names,
            read_body: each.read_body,
            source: folder_source(each.scope),
            invocable: each.invocable,
        })
        .collect();
    FolderResolution { listed, sources }
}

crate::registry::exports! {
    "host/session-folder-resolution" {
        "resolveFolderEntries" => ok |scanned: Vec<ScannedEntry>, locked: Set<String>, options: ResolveFolderOptions|
            resolve_folder_entries(&scanned, &locked, &options);
    }
}
