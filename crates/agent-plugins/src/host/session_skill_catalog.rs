//! Port of `src/host/session-skill-catalog.ts`, switched.
//!
//! The session's skills and commands as rows: what the kit itself lists (`skills()`), expands
//! (`expandSkill`, `expandMessage`) and preloads, so a host's own list, roster and prompt listing are
//! the kit's. Pure over the entries the session already holds: no port is called. An entry crosses
//! as its data alone (`CatalogEntry`): its `readBody` and its plugin stay the host's.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::registry::Set;
use crate::skills::expand_policy::SkillOriginKind;
use crate::skills::invocation::{InvocationPolicy, invocation_policy};
use crate::skills::listing::{ListedSkill, ListingOptions, is_dormant, listed_as};
use crate::skills::lookup::{LookupRow, skill_lookup};
use crate::skills::visibility::{
    ModelListing, Overridable, OverrideState, SkillOverrides, override_state,
};

/// One entry as the catalog reads it (`Entry` without its `readBody` and its plugin).
#[derive(Clone, Debug, Deserialize)]
pub struct CatalogEntry {
    /// The skill or command without its body.
    pub invocable: JsValue,
    /// Every name that invokes it, the listed command first.
    pub names: Vec<String>,
    /// Where it came from.
    pub source: JsValue,
    /// The `SKILL.md` or the command file.
    #[serde(default)]
    pub file: Option<String>,
    /// The skill's folder.
    #[serde(default)]
    pub directory: Option<String>,
}

/// One skill or command the session can invoke.
#[derive(Clone, Debug, Serialize)]
pub struct SessionSkill {
    /// The skill or command without its body.
    pub invocable: JsValue,
    /// Every name that invokes this row, the listed command first; a name another row or a
    /// workflow holds is left out.
    pub names: Vec<String>,
    /// Where it came from.
    pub source: JsValue,
    /// The `SKILL.md` or the command file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The skill's folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// The `skillOverrides` state in force.
    #[serde(rename = "override")]
    pub override_state: OverrideState,
    /// Who may invoke it and whether the `/` menu shows it, with the override applied.
    pub policy: InvocationPolicy,
    /// What the model's listing says of it before the budget (`listedAs`).
    pub listed: ModelListing,
    /// It sets `paths` and no file touched in this session activated it.
    pub dormant: bool,
}

/// What the rows are made from.
#[derive(Clone, Debug, Deserialize)]
pub struct CatalogInput {
    /// The session's entries: plugins first, then the folders, then the host's skills.
    pub entries: Vec<CatalogEntry>,
    /// The names the plugins' workflows hold: no skill row answers to them.
    pub workflows: Set<String>,
    /// `skillOverrides`.
    #[serde(default)]
    pub overrides: Option<SkillOverrides>,
    /// The commands a `paths` match activated.
    pub activated: Set<String>,
}

/// An entry with its head read as the listing reads it.
struct Read {
    entry: CatalogEntry,
    listed: ListedSkill,
}

impl Read {
    fn command(&self) -> &str {
        &self.listed.invocable.command
    }
}

fn lookup_row(read: &Read) -> Result<LookupRow, String> {
    let kind = JsValue::from(read.listed.invocable.origin.kind.as_str());
    Ok(LookupRow {
        command: read.command().to_owned(),
        kind: SkillOriginKind::deserialize(kind).map_err(|error| error.to_string())?,
        names: read.entry.names.clone(),
    })
}

/// What every row is made with: the overrides and the activated commands.
struct Facts {
    overrides: Option<SkillOverrides>,
    activated: Set<String>,
}

/// One row of an entry the lookup gives its listed command.
fn row_of(facts: &Facts, read: Read, names: Vec<String>) -> SessionSkill {
    let Read { entry, listed } = read;
    let overrides = facts.overrides.clone().unwrap_or_default();
    let invocable = &listed.invocable;
    let overridable = Overridable {
        name: invocable.name.clone(),
        origin: invocable.origin.clone(),
    };
    let options = ListingOptions {
        overrides: facts.overrides.clone(),
        activated: Some(facts.activated.clone()),
        ..ListingOptions::default()
    };
    SessionSkill {
        names,
        source: entry.source,
        file: entry.file,
        directory: entry.directory,
        override_state: override_state(&overridable, &overrides),
        policy: invocation_policy(invocable, &overrides),
        listed: listed_as(&listed, &options),
        dormant: is_dormant(&listed, Some(&facts.activated)),
        invocable: entry.invocable,
    }
}

/// The rows, in listing order: plugins first, then the folders, then the host's skills.
///
/// # Errors
///
/// An entry's `invocable` is no skill head.
pub fn catalog_rows(input: CatalogInput) -> Result<Vec<SessionSkill>, String> {
    let CatalogInput {
        entries,
        workflows,
        overrides,
        activated,
    } = input;
    let held = |name: &String| workflows.members.contains(name);
    let mut reads = Vec::new();
    for entry in entries {
        let listed =
            ListedSkill::deserialize(entry.invocable.clone()).map_err(|error| error.to_string())?;
        let read = Read { entry, listed };
        if !held(&read.command().to_owned()) {
            reads.push(read);
        }
    }
    let rows = reads
        .iter()
        .map(lookup_row)
        .collect::<Result<Vec<_>, _>>()?;
    let lookup = skill_lookup(&rows);
    let facts = Facts {
        overrides,
        activated,
    };
    let mut catalog = Vec::new();
    for (index, read) in reads.into_iter().enumerate() {
        if lookup.find(read.command()) != Some(index) {
            continue;
        }
        let names = read
            .entry
            .names
            .iter()
            .filter(|name| !held(name) && lookup.find(name) == Some(index))
            .cloned()
            .collect();
        catalog.push(row_of(&facts, read, names));
    }
    Ok(catalog)
}

crate::registry::exports! {
    "host/session-skill-catalog" {
        "catalogRows" => fallible |input: CatalogInput| catalog_rows(input);
    }
}
