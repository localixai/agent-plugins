//! Port of `src/skills/lookup.ts`, switched.
//!
//! Which skill a typed or called name invokes. A skill answers to its listed command and to its other
//! names: a plugin skill's bare name (skills.md, "How a skill gets its command name"), a folder skill's
//! directory-qualified and alias names. One lookup serves the `/` prompt, the model's Skill call and a
//! subagent's `skills` list, so a name that runs a skill one way runs it every way.
//!
//! The TypeScript's `find` hands back the very row it was given, which identity cannot carry across
//! the boundary: the core answers each name with the row's index, and the wrapper maps an index back
//! to the host's row (the public `SkillLookup` hands that object back).

use indexmap::IndexMap;
use serde::Deserialize;

use super::expand_policy::SkillOriginKind;

/// What the lookup reads of a catalog row: its listed command, its origin's kind and every name
/// that invokes it, the listed command first (`NamedInvocable`).
pub trait NamedInvocable {
    /// The listed command (`invocable.command`).
    fn command(&self) -> &str;
    /// The origin's kind (`invocable.origin.kind`).
    fn origin_kind(&self) -> SkillOriginKind;
    /// Every name that invokes the row, the listed command first.
    fn names(&self) -> &[String];
}

/// A row as the wrapper passes it: the fields the lookup reads, nothing of the host's row else.
#[derive(Clone, Debug, Deserialize)]
pub struct LookupRow {
    /// The listed command.
    pub command: String,
    /// The origin's kind.
    pub kind: SkillOriginKind,
    /// Every name that invokes it.
    pub names: Vec<String>,
}

impl NamedInvocable for LookupRow {
    fn command(&self) -> &str {
        &self.command
    }

    fn origin_kind(&self) -> SkillOriginKind {
        self.kind
    }

    fn names(&self) -> &[String] {
        &self.names
    }
}

/// The rows by name: each name with the index of the row it invokes, in the order claimed.
#[derive(Clone, Debug, Default)]
pub struct SkillLookup {
    by_name: IndexMap<String, usize>,
}

impl SkillLookup {
    /// The index of the row a name invokes, or `None`.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }

    /// Every name that invokes a row, in the order claimed.
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.by_name.keys()
    }

    /// Each name with its row's index, in the order claimed: what the wrapper maps back.
    #[must_use]
    pub fn claims(&self) -> Vec<(String, usize)> {
        self.by_name
            .iter()
            .map(|(name, index)| (name.clone(), *index))
            .collect()
    }
}

/// The rows whose listed command is claimed before any other's. A synced skill's command is
/// `anthropic-skills:<name>`, which only a plugin named `anthropic-skills` can also produce, and
/// skills.md ("Names reserved for synced skills") says that `/anthropic-skills:<name>` then "runs the
/// synced skill".
const fn claims_first(kind: SkillOriginKind) -> bool {
    match kind {
        SkillOriginKind::Synced => true,
        SkillOriginKind::Plugin
        | SkillOriginKind::Managed
        | SkillOriginKind::User
        | SkillOriginKind::Project
        | SkillOriginKind::Bundled => false,
    }
}

/// The rows by name. A synced skill's command is claimed first (`CLAIMS_FIRST`); otherwise the
/// earlier row wins a command. Judgment (spec/judgments.md): a listed command wins over another
/// row's alias of the same text, and among aliases the earlier row wins.
pub fn skill_lookup<E: NamedInvocable>(rows: &[E]) -> SkillLookup {
    let mut by_name = IndexMap::new();
    let first = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| claims_first(row.origin_kind()));
    for (index, row) in first.chain(rows.iter().enumerate()) {
        by_name.entry(row.command().to_owned()).or_insert(index);
    }
    for (index, row) in rows.iter().enumerate() {
        for name in row.names() {
            by_name.entry(name.clone()).or_insert(index);
        }
    }
    SkillLookup { by_name }
}

crate::registry::exports! {
    "skills/lookup" {
        "skillLookup" => ok |rows: Vec<LookupRow>| skill_lookup(&rows).claims();
    }
}

#[cfg(test)]
mod tests {
    use super::{LookupRow, SkillOriginKind, skill_lookup};

    fn row(command: &str, kind: SkillOriginKind, names: &[&str]) -> LookupRow {
        LookupRow {
            command: command.to_owned(),
            kind,
            names: names.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    #[test]
    fn a_synced_skill_claims_its_command_first_and_a_command_wins_over_an_alias() {
        let rows = [
            row(
                "anthropic-skills:deploy",
                SkillOriginKind::Plugin,
                &["deploy"],
            ),
            row("anthropic-skills:deploy", SkillOriginKind::Synced, &[]),
            row("deploy", SkillOriginKind::Project, &[]),
        ];
        let lookup = skill_lookup(&rows);
        assert_eq!(lookup.find("anthropic-skills:deploy"), Some(1));
        assert_eq!(lookup.find("deploy"), Some(2));
        assert_eq!(lookup.find("other"), None);
    }
}
