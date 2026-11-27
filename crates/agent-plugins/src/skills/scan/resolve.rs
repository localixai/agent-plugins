//! Port of `src/skills/scan/resolve.ts`, switched.
//!
//! Which skill a name runs when several share it (skills.md, "Resolve skills that share a name",
//! "How a skill gets its command name", "Run a same-named skill from an added directory", "When a
//! synced skill name matches another command"). The scans of every scope go in, the skills and
//! commands a session lists come out, each with the names that invoke it.
//!
//! Every scope has a naming strategy in one registry typed over the scopes: the names it prefers,
//! the names it falls back to when none of those is free, and whether names are compared as the
//! synced skills' are (ignoring case, spacing and invisible characters).

use std::collections::{HashMap, HashSet};

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use super::locations::{ScanScope, scope_rule};
use super::names::{fold_name, relative_below, segments_of};
use super::scan_types::ScannedEntry;
use crate::format::skill_types::BodyReader;
use crate::js::{JsObject, JsValue};
use crate::registry::Set;

/// Who holds a name: a skill or command file of the folders, or (absent) another source of
/// commands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameHolder {
    /// The scope it was read from.
    pub scope: ScanScope,
    /// Its file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// A name an entry would have liked and did not get, and who has it (skills.md, "When a synced
/// skill name matches another command").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YieldedName {
    /// The name.
    pub name: String,
    /// The folder skill or command file that uses it; absent for a built-in command, a plugin
    /// skill or an MCP prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_by: Option<NameHolder>,
}

/// One listed skill or command of the folders.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedFolderSkill {
    /// The skill or command, without its body, with its listed command (`/apps/web:deploy`,
    /// `/cli:verify`).
    pub invocable: JsObject,
    /// Reads its body when it runs (`ScannedEntry.readBody`).
    pub read_body: BodyReader,
    /// The scope it was read from.
    pub scope: ScanScope,
    /// Every name that invokes it, the listed command first.
    pub names: Vec<String>,
    /// The names it prefers that another command already used: a synced skill that lost its short
    /// name lists it here.
    pub yielded: Vec<YieldedName>,
}

/// An entry that lost its name to another, for a host that wants to say so.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadowedEntry {
    /// The scope it was read from.
    pub scope: ScanScope,
    /// The command it would have had.
    pub command: String,
    /// Its file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// What the resolution lists and what it leaves out.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resolution {
    /// The listed skills and commands.
    pub resolved: Vec<ResolvedFolderSkill>,
    /// The entries that lost every name.
    pub shadowed: Vec<ShadowedEntry>,
}

/// The commands other sources already use, as the host gives them (`Iterable<string>`).
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Taken {
    /// An array.
    List(Vec<String>),
    /// A `Set`.
    Set(Set<String>),
}

/// What `resolveFolderSkills` resolves against.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveFolderOptions {
    /// The working directory the nested skills' qualified names are relative to.
    pub cwd: String,
    /// The added directories, in order: their folder names prefix a clashing skill.
    #[serde(default)]
    pub added_directories: Option<Vec<String>>,
    /// Commands other sources already use: built-in commands, bundled skills, plugin skills, MCP
    /// prompts. Only a synced skill's short name yields to them.
    #[serde(default)]
    pub taken: Option<Taken>,
}

struct Naming {
    preferred: Vec<String>,
    fallback: Vec<String>,
    /// Compare as synced skills do: case, spacing, invisible characters and compatibility forms
    /// ignored.
    folded: bool,
}

struct NamingContext<'a> {
    options: &'a ResolveFolderOptions,
    prefixes: HashMap<String, String>,
}

fn is_skill(invocable: &JsObject) -> bool {
    invocable.contains_key("dir")
}

fn text_of<'a>(invocable: &'a JsObject, key: &str) -> Option<&'a str> {
    invocable.get(key).and_then(JsValue::as_str)
}

fn unique(names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .collect::<IndexSet<_>>()
        .into_iter()
        .collect()
}

/// `deploy` by its frontmatter `name` or by its folder: both invoke a skill in a folder scope.
fn own_names(entry: &ScannedEntry) -> Naming {
    let command = text_of(&entry.invocable, "command")
        .unwrap_or_default()
        .to_owned();
    Naming {
        preferred: unique(vec![command, entry.dir_name.clone()]),
        fallback: Vec::new(),
        folded: false,
    }
}

/// `NAMING[entry.scope](entry, context)`.
fn naming_of(entry: &ScannedEntry, context: &NamingContext<'_>) -> Naming {
    match entry.scope {
        ScanScope::Managed | ScanScope::User | ScanScope::Project => own_names(entry),
        ScanScope::Nested => Naming {
            fallback: vec![format!(
                "{}:{}",
                relative_below(&context.options.cwd, &entry.base),
                entry.dir_name
            )],
            ..own_names(entry)
        },
        ScanScope::Additional => Naming {
            // Only a skill gets the prefix: a command of an added directory "runs your project's
            // command" instead (permissions.md, "Additional directories grant file access, not
            // configuration").
            fallback: if is_skill(&entry.invocable) {
                let prefix = context.prefixes.get(&entry.base).unwrap_or(&entry.base);
                vec![format!("{prefix}:{}", entry.dir_name)]
            } else {
                Vec::new()
            },
            ..own_names(entry)
        },
        ScanScope::Synced => {
            let command = text_of(&entry.invocable, "command").unwrap_or_default();
            let short = if is_skill(&entry.invocable) {
                text_of(&entry.invocable, "name").unwrap_or_default()
            } else {
                command
            };
            Naming {
                preferred: vec![short.to_owned(), command.to_owned()],
                fallback: Vec::new(),
                folded: true,
            }
        }
    }
}

/// The names in use, exactly and folded. The commands other sources use (`taken`) count for a
/// synced skill's short name only: a skill of the folders replaces a built-in command of its name
/// (skills.md, "Resolve skills that share a name").
struct Claims {
    exact: HashMap<String, NameHolder>,
    folded: HashMap<String, Option<NameHolder>>,
}

impl Claims {
    fn new(others: &[String]) -> Self {
        Self {
            exact: HashMap::new(),
            folded: others.iter().map(|name| (fold_name(name), None)).collect(),
        }
    }

    fn add(&mut self, name: &str, holder: &NameHolder) {
        self.exact.insert(name.to_owned(), holder.clone());
        self.folded.insert(fold_name(name), Some(holder.clone()));
    }

    fn free(&self, name: &str, folded: bool) -> bool {
        if folded {
            !self.folded.contains_key(&fold_name(name))
        } else {
            !self.exact.contains_key(name)
        }
    }

    /// Who uses the name; `None` for a name nobody has and for one only another source of
    /// commands has.
    fn holder(&self, name: &str, folded: bool) -> Option<NameHolder> {
        if folded {
            self.folded.get(&fold_name(name)).cloned().flatten()
        } else {
            self.exact.get(name).cloned()
        }
    }
}

/// The last `depth` parts joined by `/` (`parts.slice(-depth).join('/')`).
fn last_parts(parts: &[String], depth: usize) -> String {
    parts
        .get(parts.len().saturating_sub(depth)..)
        .unwrap_or_default()
        .join("/")
}

/// The prefix of each added directory: its folder's name; when two share one, the parent's name
/// before it (`b/cli`), with more parents only while they still match (judgment: the docs name the
/// parent only).
fn prefixes_of(directories: &[String]) -> HashMap<String, String> {
    let parts: Vec<Vec<String>> = directories
        .iter()
        .map(|directory| segments_of(directory))
        .collect();
    let joined: Vec<String> = parts.iter().map(|each| each.join("/")).collect();
    let mut found = HashMap::new();
    for (index, (directory, own)) in directories.iter().zip(&parts).enumerate() {
        let mut depth = 1;
        let clashes = |depth: usize| {
            parts
                .iter()
                .zip(&joined)
                .enumerate()
                .any(|(at, (other, other_joined))| {
                    at != index
                        && joined.get(index) != Some(other_joined)
                        && last_parts(other, depth) == last_parts(own, depth)
                })
        };
        while clashes(depth) && depth < own.len() {
            depth += 1;
        }
        found.insert(directory.clone(), last_parts(own, depth));
    }
    found
}

fn with_command(mut invocable: JsObject, command: &str) -> JsObject {
    invocable.insert("command".to_owned(), JsValue::from(command));
    invocable
}

/// Nested skills that took a qualified name say which directory they apply to (skills.md, "Load
/// skills in monorepos").
fn qualified(mut invocable: JsObject, directory: &str) -> JsObject {
    let note = format!("applies to {directory}");
    let description = match invocable.get("description") {
        None | Some(JsValue::Undefined) => note,
        Some(description) => format!("{} ({note})", crate::js::string_of(description)),
    };
    invocable.insert("description".to_owned(), JsValue::from(description));
    invocable
}

enum Placed {
    Resolved(ResolvedFolderSkill),
    Shadowed(ShadowedEntry),
}

fn place(entry: &ScannedEntry, claims: &mut Claims, context: &NamingContext<'_>) -> Placed {
    let naming = naming_of(entry, context);
    let folded = naming.folded;
    let (mut names, taken): (Vec<String>, Vec<String>) = naming
        .preferred
        .into_iter()
        .partition(|name| claims.free(name, folded));
    let yielded = taken
        .into_iter()
        .map(|name| YieldedName {
            held_by: claims.holder(&name, folded),
            name,
        })
        .collect();
    let used_fallback = names.is_empty();
    if used_fallback {
        names = naming
            .fallback
            .into_iter()
            .filter(|name| claims.free(name, folded))
            .take(1)
            .collect();
    }
    let path = text_of(&entry.invocable, "path").map(str::to_owned);
    let Some(command) = names.first().cloned() else {
        return Placed::Shadowed(ShadowedEntry {
            scope: entry.scope,
            command: text_of(&entry.invocable, "command")
                .unwrap_or_default()
                .to_owned(),
            path,
        });
    };
    let holder = NameHolder {
        scope: entry.scope,
        path,
    };
    for name in &names {
        claims.add(name, &holder);
    }
    let base = with_command(entry.invocable.clone(), &command);
    let invocable = if used_fallback {
        qualified_note(base, entry, context)
    } else {
        base
    };
    Placed::Resolved(ResolvedFolderSkill {
        invocable,
        read_body: entry.read_body.clone(),
        scope: entry.scope,
        names,
        yielded,
    })
}

fn qualified_note(
    invocable: JsObject,
    entry: &ScannedEntry,
    context: &NamingContext<'_>,
) -> JsObject {
    if is_skill(&invocable) && entry.scope == ScanScope::Nested {
        qualified(
            invocable,
            &relative_below(&context.options.cwd, &entry.base),
        )
    } else {
        invocable
    }
}

/// Skills of the folder scopes first, then command files (a skill wins over a command), then
/// synced skills.
fn in_phases(entries: Vec<&ScannedEntry>) -> Vec<&ScannedEntry> {
    let mut ranked = entries;
    ranked.sort_by_key(|entry| scope_rule(entry.scope).rank);
    let synced = |entry: &&ScannedEntry| entry.scope == ScanScope::Synced;
    let local: Vec<&ScannedEntry> = ranked
        .iter()
        .copied()
        .filter(|entry| !synced(entry))
        .collect();
    let skills = local
        .iter()
        .copied()
        .filter(|entry| is_skill(&entry.invocable));
    let commands = local
        .iter()
        .copied()
        .filter(|entry| !is_skill(&entry.invocable));
    skills
        .chain(commands)
        .chain(ranked.iter().copied().filter(synced))
        .collect()
}

/// One entry per real target: the same folder reached through several links loads once.
fn once_per_target(entries: &[ScannedEntry]) -> Vec<&ScannedEntry> {
    let mut seen = HashSet::new();
    entries
        .iter()
        .filter(|entry| {
            let Some(canonical) = &entry.canonical else {
                return true;
            };
            let kind = if is_skill(&entry.invocable) {
                "skill"
            } else {
                "command"
            };
            seen.insert(format!("{kind}:{canonical}"))
        })
        .collect()
}

/// The listed skills and commands of the folders, each with the names that invoke it.
#[must_use]
pub fn resolve_folder_skills(
    scanned: &[ScannedEntry],
    options: &ResolveFolderOptions,
) -> Resolution {
    let taken = match &options.taken {
        None => &[][..],
        Some(Taken::List(names)) => names.as_slice(),
        Some(Taken::Set(names)) => names.members.as_slice(),
    };
    let mut claims = Claims::new(taken);
    let context = NamingContext {
        options,
        prefixes: prefixes_of(options.added_directories.as_deref().unwrap_or_default()),
    };
    let mut resolution = Resolution {
        resolved: Vec::new(),
        shadowed: Vec::new(),
    };
    for entry in in_phases(once_per_target(scanned)) {
        match place(entry, &mut claims, &context) {
            Placed::Resolved(resolved) => resolution.resolved.push(resolved),
            Placed::Shadowed(shadowed) => resolution.shadowed.push(shadowed),
        }
    }
    resolution
}

crate::registry::exports! {
    "skills/scan/resolve" {
        "resolveFolderSkills" => ok |scanned: Vec<ScannedEntry>, options: ResolveFolderOptions|
            resolve_folder_skills(&scanned, &options);
    }
}
