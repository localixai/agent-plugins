//! Port of `src/host/session-host-skills.ts`, switched.
//!
//! A skill the host supplies itself (`SessionOptions.hostSkills`): one built into the host, or one its
//! extension mechanism registers at run time. Claude Code's counterpart is a bundled skill (skills.md,
//! "Bundled skills"), and the kit applies the docs' rules for one: a folder skill of its name replaces
//! it, a synced skill's short name yields to it, `disableBundledSkills` and
//! `CLAUDE_CODE_DISABLE_BUNDLED_SKILLS` remove it, `skillOverrides` applies to it, and neither
//! `disableSkillShellExecution`, `allowManagedPermissionRulesOnly` nor the plugin-only lock touches it.
//!
//! The memo by the host's list (the very array its getter returns) and the settings that remove the
//! host skills stay at the edge: the list is the host's object, and the settings are read there.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::session_skill_entries::{SkillPlace, place_of};
use crate::format::bundled_skill::{BundledSkillHead, read_bundled_skill};
use crate::format::issues::{PluginIssue, error};
use crate::format::skill_types::{BodyReader, KnownBody};
use crate::js::{JsObject, JsValue};

/// The body of a host skill: its Markdown, or a reader the session calls at each expansion.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum HostBody {
    /// The Markdown after the frontmatter.
    Text(String),
    /// The host's reader.
    Reader(BodyReader),
}

/// A skill the host supplies itself.
#[derive(Clone, Debug, Deserialize)]
pub struct HostSkill {
    /// The head: its name, frontmatter and file.
    #[serde(flatten)]
    pub head: BundledSkillHead,
    /// The body or its reader.
    pub body: HostBody,
}

/// One host skill's entry (`Entry` with `source: { kind: 'host' }`).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    /// The skill without its body.
    pub invocable: JsObject,
    /// Its one name, the listed command.
    pub names: Vec<String>,
    /// Reads its body.
    pub read_body: BodyReader,
    /// `{ kind: 'host' }`.
    pub source: JsValue,
    /// Where it lives.
    #[serde(flatten)]
    pub place: SkillPlace,
}

/// The host skills' entries and what keeps one from loading.
#[derive(Clone, Debug, Default, Serialize)]
pub struct HostSkillEntries {
    /// The entries, in the list's order.
    pub entries: Vec<HostEntry>,
    /// A refused name, a second skill of one name.
    pub issues: Vec<PluginIssue>,
}

fn reader_of(body: HostBody) -> BodyReader {
    match body {
        HostBody::Text(body) => BodyReader::core(KnownBody(body)),
        HostBody::Reader(reader) => reader,
    }
}

fn host_source() -> JsValue {
    let mut source = JsObject::new();
    source.insert("kind".to_owned(), "host".into());
    JsValue::Object(source)
}

/// The entries of the host's list. Judgment (spec/judgments.md): a second skill of one name is an
/// issue and the first stays.
#[must_use]
pub fn host_entries_of(list: Vec<HostSkill>) -> HostSkillEntries {
    let mut read_all = HostSkillEntries::default();
    let mut seen = HashSet::new();
    for skill in list {
        let text = match &skill.body {
            HostBody::Text(body) => body.as_str(),
            HostBody::Reader(_) => "",
        };
        let read = read_bundled_skill(&skill.head, text);
        read_all.issues.extend(read.issues);
        let Some(invocable) = read.invocable else {
            continue;
        };
        let command = invocable
            .get("command")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
            .to_owned();
        if !seen.insert(command.clone()) {
            read_all.issues.push(error(
                &format!("hostSkills.{}", skill.head.name),
                "a host skill of this name came first; this one is not used",
            ));
            continue;
        }
        read_all.entries.push(HostEntry {
            place: place_of(&invocable),
            names: vec![command],
            read_body: reader_of(skill.body),
            source: host_source(),
            invocable,
        });
    }
    read_all
}

crate::registry::exports! {
    "host/session-host-skills" {
        "hostEntriesOf" => ok |list: Vec<HostSkill>| host_entries_of(list);
    }
}
