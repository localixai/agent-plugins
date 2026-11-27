//! Port of `src/marketplace/name-conflicts.ts`, switched.
//!
//! Which plugin loads when enabled plugins from different origins share a manifest name
//! (plugins/loading.md, "Name conflicts", "Hooks when two enabled plugins share a name", "Keep a
//! session-only plugin from loading"). Pure over what the host found. The rows and messages the docs
//! quote are the docs'; the wording of the rest is the kit's (spec/judgments.md).
//!
//! A candidate that "is" the winner is the same element of the list the host passed, as in the
//! TypeScript (`===`): the copies are compared by place, not by value.

use std::cmp::Ordering;
use std::ptr;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Where a plugin came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginOrigin {
    /// `session-only`.
    #[serde(rename = "session-only")]
    SessionOnly,
    /// `installed`.
    #[serde(rename = "installed")]
    Installed,
    /// `skills-dir`.
    #[serde(rename = "skills-dir")]
    SkillsDir,
    /// `synced`.
    #[serde(rename = "synced")]
    Synced,
}

/// Whose skills directory a plugin was scaffolded in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillsDirScope {
    /// `~/.claude/skills/`.
    User,
    /// The project's `.claude/skills/`.
    Project,
}

/// A skills-directory plugin's scope and path, to say which copy shadowed which.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillsDir {
    /// Its scope.
    pub scope: SkillsDirScope,
    /// Its path.
    pub path: String,
}

/// One copy of a manifest name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameCandidate {
    /// `name@marketplace`, `name@inline`, `name@skills-dir`.
    pub id: String,
    /// The manifest name every copy of the conflict shares.
    pub name: String,
    /// Where it came from.
    pub origin: PluginOrigin,
    /// Enabled in the settings that apply; `"<id>": false` disables a session-only plugin too.
    pub enabled: bool,
    /// The id appears in managed settings `enabledPlugins`, as `true` or `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_locked: Option<bool>,
    /// A skills-directory plugin's scope and path, to say which copy shadowed which.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills_dir: Option<SkillsDir>,
}

/// What became of one copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameVerdict {
    /// The copy.
    pub id: String,
    /// It loads.
    pub loads: bool,
    /// The row of the `/plugin` **Errors** tab, or the line printed, for a copy that did not load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Recorded only by the `--debug` log: the installed copy a session-only plugin replaced silently.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_log: Option<String>,
}

/// Highest precedence first: managed, session-only, installed, skills-directory, synced.
const fn rank(origin: PluginOrigin) -> u8 {
    match origin {
        PluginOrigin::SessionOnly => 1,
        PluginOrigin::Installed => 2,
        PluginOrigin::SkillsDir => 3,
        PluginOrigin::Synced => 4,
    }
}
const MANAGED_RANK: u8 = 0;

fn rank_of(candidate: &NameCandidate) -> u8 {
    if candidate.managed_locked == Some(true) {
        MANAGED_RANK
    } else {
        rank(candidate.origin)
    }
}

/// "The copy under `~/.claude/skills/` loads and the project's `.claude/skills/` copy is dropped."
const fn scope_order(scope: SkillsDirScope) -> u8 {
    match scope {
        SkillsDirScope::User => 0,
        SkillsDirScope::Project => 1,
    }
}

fn scope_rank(candidate: &NameCandidate) -> u8 {
    candidate
        .skills_dir
        .as_ref()
        .map_or(0, |skills_dir| scope_order(skills_dir.scope))
}

const SESSION_FLAGS: &str = "--plugin-dir / --plugin-url";

/// What a copy that lost says: a note, a debug log line, or neither.
#[derive(Default)]
struct LoserNote {
    note: Option<String>,
    debug_log: Option<String>,
}

/// What a copy that lost says, by where it came from.
fn loser_note(loser: &NameCandidate, winner: &NameCandidate) -> LoserNote {
    let name = &loser.name;
    match loser.origin {
        PluginOrigin::SessionOnly => LoserNote {
            note: (winner.managed_locked == Some(true)).then(|| {
                format!(
                    "--plugin-dir copy of \"{name}\" ignored: plugin is locked by managed settings"
                )
            }),
            debug_log: None,
        },
        PluginOrigin::Installed => LoserNote {
            note: None,
            debug_log: (winner.origin == PluginOrigin::SessionOnly).then(|| {
                format!("Plugin \"{name}\" from --plugin-dir overrides installed version")
            }),
        },
        PluginOrigin::SkillsDir => LoserNote {
            note: Some(match winner.origin {
                PluginOrigin::SessionOnly => format!(
                    "Not loaded \u{2014} the name \"{name}\" is already taken by a session-only plugin ({SESSION_FLAGS}), which takes precedence"
                ),
                PluginOrigin::SkillsDir => format!(
                    "Not loaded \u{2014} shadowed by {}",
                    winner
                        .skills_dir
                        .as_ref()
                        .map_or(winner.id.as_str(), |skills_dir| skills_dir.path.as_str())
                ),
                PluginOrigin::Installed | PluginOrigin::Synced => format!(
                    "Not loaded \u{2014} the name \"{name}\" is already taken by the installed plugin {}",
                    winner.id
                ),
            }),
            debug_log: None,
        },
        PluginOrigin::Synced => LoserNote {
            note: Some(format!(
                "Not loaded \u{2014} the name \"{name}\" is already taken by {}. To use the claude.ai copy instead, disable your own copy",
                winner.id
            )),
            debug_log: None,
        },
    }
}

/// The copy of a name that loads and what each other copy reports. A disabled copy never holds the name
/// ("A disabled session-only plugin doesn't shadow"), except that a managed-locked id keeps a
/// `--plugin-dir` copy of its name out even when its value is `false`.
#[must_use]
pub fn resolve_name_conflict(candidates: &[NameCandidate]) -> Vec<NameVerdict> {
    let locked = candidates
        .iter()
        .find(|candidate| candidate.managed_locked == Some(true));
    let winner = candidates
        .iter()
        .filter(|candidate| {
            candidate.enabled
                && !(candidate.origin == PluginOrigin::SessionOnly && locked.is_some())
        })
        .min_by_key(|candidate| (rank_of(candidate), scope_rank(candidate)));
    candidates
        .iter()
        .map(|candidate| {
            if winner.is_some_and(|winner| ptr::eq(candidate, winner)) {
                return NameVerdict {
                    id: candidate.id.clone(),
                    loads: true,
                    note: None,
                    debug_log: None,
                };
            }
            let kept_out = candidate.origin == PluginOrigin::SessionOnly && locked.is_some();
            let holder = if kept_out { locked } else { winner };
            let reports = candidate.enabled || kept_out;
            let said = match holder {
                Some(holder) if reports => loser_note(candidate, holder),
                Some(_) | None => LoserNote::default(),
            };
            NameVerdict {
                id: candidate.id.clone(),
                loads: false,
                note: said.note,
                debug_log: said.debug_log,
            }
        })
        .collect()
}

/// One copy of a plugin name that declares hooks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookCopy {
    /// The copy.
    pub id: String,
    /// The order Claude Code loads the copies in.
    pub load_order: f64,
    /// Managed settings turn this copy on.
    pub managed_enabled: bool,
    /// It has `hooks/hooks.json` hooks.
    pub hooks_json: bool,
    /// It has a hooks module.
    pub hooks_module: bool,
}

/// What a copy's hooks are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum HookNoteKind {
    /// `hooks.json`.
    #[serde(rename = "hooks.json")]
    HooksJson,
    /// `hooks module`.
    #[serde(rename = "hooks module")]
    HooksModule,
}

/// The note of a copy whose hooks are left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HookNote {
    /// The copy.
    pub id: String,
    /// What is left out.
    pub kind: HookNoteKind,
    /// The note of the **Errors** tab and of the copy's details.
    pub message: String,
}

const fn note_start(kind: HookNoteKind) -> &'static str {
    match kind {
        HookNoteKind::HooksJson => "Its hooks.json hooks do not run",
        HookNoteKind::HooksModule => "Its hooks module does not load",
    }
}

/// "When your organization's managed settings turn on one of the copies, that copy holds the name. Otherwise the copy Claude Code loads first holds it."
#[must_use]
pub fn name_holder(copies: &[HookCopy]) -> Option<&HookCopy> {
    let managed: Vec<&HookCopy> = copies.iter().filter(|copy| copy.managed_enabled).collect();
    let pool: Vec<&HookCopy> = if managed.is_empty() {
        copies.iter().collect()
    } else {
        managed
    };
    pool.into_iter().min_by(|a, b| {
        a.load_order
            .partial_cmp(&b.load_order)
            .unwrap_or(Ordering::Equal)
    })
}

/// The notes for the copies whose hooks are left out; one plugin per name registers `hooks/hooks.json` hooks and one loads a hooks module.
#[must_use]
pub fn hook_conflict_notes(copies: &[HookCopy]) -> Vec<HookNote> {
    let Some(holder) = name_holder(copies) else {
        return Vec::new();
    };
    let kinds = [HookNoteKind::HooksJson, HookNoteKind::HooksModule];
    let has = |copy: &HookCopy, kind: HookNoteKind| match kind {
        HookNoteKind::HooksJson => copy.hooks_json,
        HookNoteKind::HooksModule => copy.hooks_module,
    };
    copies
        .iter()
        .filter(|copy| !ptr::eq(*copy, holder))
        .flat_map(|copy| {
            kinds
                .into_iter()
                .filter(move |kind| has(copy, *kind))
                .filter(move |kind| has(holder, *kind))
                .map(move |kind| HookNote {
                    id: copy.id.clone(),
                    kind,
                    message: format!("{}: {} holds the name", note_start(kind), holder.id),
                })
        })
        .collect()
}

/// `--plugin-dir` plugin ids are `<name>@inline`: the entry that keeps one from loading.
#[must_use]
pub fn session_only_off_entry(name: &str) -> IndexMap<String, bool> {
    IndexMap::from([(format!("{name}@inline"), false)])
}

/// What disabling a copy allows and what is left.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AfterDisabling {
    /// Disabling is allowed.
    pub allowed: bool,
    /// The notes that remain.
    pub notes: Vec<HookNote>,
}

/// Disabling the holder (a copy managed settings do not turn on) and reloading registers the remaining copy's hooks.
#[must_use]
pub fn after_disabling(copies: &[HookCopy], disabled_id: &str) -> AfterDisabling {
    let target = copies.iter().find(|copy| copy.id == disabled_id);
    if target.is_some_and(|target| target.managed_enabled) {
        return AfterDisabling {
            allowed: false,
            notes: hook_conflict_notes(copies),
        };
    }
    let remaining: Vec<HookCopy> = copies
        .iter()
        .filter(|copy| copy.id != disabled_id)
        .cloned()
        .collect();
    AfterDisabling {
        allowed: true,
        notes: hook_conflict_notes(&remaining),
    }
}

crate::registry::exports! {
    "marketplace/name-conflicts" {
        "resolveNameConflict" => ok |candidates: Vec<NameCandidate>| resolve_name_conflict(&candidates);
        "nameHolder" => ok |copies: Vec<HookCopy>| name_holder(&copies).cloned();
        "hookConflictNotes" => ok |copies: Vec<HookCopy>| hook_conflict_notes(&copies);
        "sessionOnlyOffEntry" => ok |name: String| session_only_off_entry(&name);
        "afterDisabling" => ok |copies: Vec<HookCopy>, disabled_id: String| after_disabling(&copies, &disabled_id);
    }
}
