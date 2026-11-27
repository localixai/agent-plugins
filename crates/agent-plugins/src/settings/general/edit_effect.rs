//! Port of `src/settings/general/edit-effect.ts`, switched.
//!
//! When an edit reaches a running session (settings.md, "When edits take effect";
//! managed-settings.md, "Where and when a policy applies"). A pure decision: the host watches the
//! files, reloads and fires the `ConfigChange` hook; this says what each change is owed.

use serde::{Deserialize, Serialize};

use super::reach::FileScope;

/// How the change arrived: a settings file Claude Code watches, or a managed source delivered on a
/// schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Arrival {
    /// A settings file.
    File,
    /// An MDM policy.
    Mdm,
    /// The claude.ai console.
    ServerManaged,
}

/// One edit of a settings source.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsEdit {
    /// The file's scope.
    pub scope: FileScope,
    /// How the change arrived.
    pub arrival: Arrival,
    /// The top-level key, or the dotted path of one inside a block; `undefined` for a change to the
    /// file as a whole.
    #[serde(default)]
    pub key: Option<String>,
    /// The edit removes the key rather than setting it.
    #[serde(default)]
    pub removed: Option<bool>,
}

/// When a change reaches the running session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Applies {
    /// The running session reloads it: "most edits ... without a restart".
    Now,
    /// A managed source reaches the session "on a schedule rather than on save".
    OnSchedule,
    /// "reads some keys only once, at session start".
    NextSession,
    /// The running session keeps the old behaviour until each session restarts.
    AfterRestart,
}

/// What an edit is owed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditEffect {
    /// When it reaches the session.
    pub applies: Applies,
    /// Claude Code runs the `ConfigChange` hook "for each settings-file change it detects, not for
    /// managed settings that arrive from MDM or the claude.ai console".
    pub config_change_hook: bool,
}

/// What each arrival owes before any key is looked at.
const fn by_arrival(arrival: Arrival) -> EditEffect {
    match arrival {
        Arrival::File => EditEffect {
            applies: Applies::Now,
            config_change_hook: true,
        },
        Arrival::Mdm | Arrival::ServerManaged => EditEffect {
            applies: Applies::OnSchedule,
            config_change_hook: false,
        },
    }
}

/// The keys read once, when a session starts. settings.md names `model`, `effortLevel` and
/// `modelSettings` as the ones most likely edited mid-session, and "Admin-side keys that also wait
/// for a restart, such as `requiredMinimumVersion`"; managed-settings.md adds
/// `forceRemoteSettingsRefresh` and says a new or changed `policyHelper` "takes effect at the next
/// launch".
const READ_AT_START: [&str; 6] = [
    "model",
    "effortLevel",
    "modelSettings",
    "requiredMinimumVersion",
    "forceRemoteSettingsRefresh",
    "policyHelper",
];

/// Keys a running session keeps in force after the edit that lifts them: "If you remove
/// `permissions.blockReadsOutsideWorkingDirectories` mid-session, the file tools keep the block in
/// the running session. To lift it, remove the key from every settings file that sets it, then
/// restart each running session."
const LIFTED_ON_RESTART: [&str; 1] = ["permissions.blockReadsOutsideWorkingDirectories"];

/// What an edit is owed: the arrival's effect, unless the key waits for a restart.
#[must_use]
pub fn edit_effect(edit: &SettingsEdit) -> EditEffect {
    let arrival = by_arrival(edit.arrival);
    let Some(key) = &edit.key else {
        return arrival;
    };
    if edit.removed == Some(true) && LIFTED_ON_RESTART.contains(&key.as_str()) {
        return EditEffect {
            applies: Applies::AfterRestart,
            ..arrival
        };
    }
    if READ_AT_START.contains(&key.as_str()) {
        EditEffect {
            applies: Applies::NextSession,
            ..arrival
        }
    } else {
        arrival
    }
}

/// Whether a settings file created while a session runs is loaded by it: "if its folder existed
/// when the session started. For the project's `.claude/` folder, it loads the file even when you
/// create the folder in the same session."
#[must_use]
pub const fn loads_file_created_mid_session(
    scope: FileScope,
    folder_existed_at_start: bool,
) -> bool {
    match scope {
        FileScope::User | FileScope::Local | FileScope::Managed => folder_existed_at_start,
        FileScope::Project => true,
    }
}

/// What the host knows of a managed change that has not reached a person.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedChangeFacts {
    /// The session has restarted since the administrator changed the managed source.
    pub restarted: bool,
    /// The managed source the administrator changed, as `/status` names sources.
    pub changed_source: String,
    /// The managed source `Setting sources` of `/status` names for the session.
    pub status_source: String,
}

/// What to tell a person whose managed change has not reached them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManagedChangeAdvice {
    /// Managed sources "reach a running session on the schedule in the delivery table, so restart
    /// the session first".
    Restart,
    /// `/status` names another source than the one changed: "a higher-priority source applies".
    HigherSourceApplies,
    /// The session runs the changed source: the change has reached it.
    Reached,
}

/// settings.md, "A managed change hasn't reached you": restart first; "If `/status` then names a
/// different source than the one your administrator changed, a higher-priority source applies"
/// (the order among managed sources is managed-settings.md, "How Claude Code combines managed
/// sources", the host's to show).
#[must_use]
pub fn managed_change_advice(facts: &ManagedChangeFacts) -> ManagedChangeAdvice {
    if !facts.restarted {
        return ManagedChangeAdvice::Restart;
    }
    if facts.status_source == facts.changed_source {
        ManagedChangeAdvice::Reached
    } else {
        ManagedChangeAdvice::HigherSourceApplies
    }
}

crate::registry::exports! {
    "settings/general/edit-effect" {
        "editEffect" => ok |edit: SettingsEdit| edit_effect(&edit);
        "loadsFileCreatedMidSession" => ok |scope: FileScope, folder_existed_at_start: bool| loads_file_created_mid_session(scope, folder_existed_at_start);
        "managedChangeAdvice" => ok |facts: ManagedChangeFacts| managed_change_advice(&facts);
    }
}
