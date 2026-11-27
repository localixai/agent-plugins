//! Port of `src/loading/workspace-trust.ts`, switched.
//!
//! Workspace trust (permissions.md, "Project allow rules and workspace trust"; hooks.md, "Workspace
//! trust"): which folder the trust is keyed on, how far it reaches, and what a session counts as
//! trusted. Pure over what the host reports; the answer to "was the dialog accepted" is the
//! `WorkspaceTrust` port's.

use serde::{Deserialize, Serialize};

use super::ports::WorkspaceTrust;
use super::trust_gates::{
    RepositoryContent, TrustSituation, TrustSituationFacts, content_gate, trust_situation,
};
use crate::ports::Signal;
use crate::registry::{Callback, Port};

/// What the host knows of the folder.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustFacts {
    /// The directory the session started in.
    pub start_dir: String,
    /// The root of the git repository that holds `startDir` (a submodule or clone is its own); absent outside any repository.
    #[serde(default)]
    pub repository_root: Option<String>,
    /// In a worktree: the root of the main checkout.
    #[serde(default)]
    pub main_checkout_root: Option<String>,
    /// The home directory.
    pub home_dir: String,
    /// Outside a repository: the directories above `startDir`, nearest first.
    #[serde(default)]
    pub ancestors: Option<Vec<String>>,
}

/// The key the trust is stored under.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TrustKey {
    /// The folder the dialog's answer is stored under.
    pub key: String,
    /// False in the home directory: "holds the trust for the current session only and doesn't write it to disk".
    pub persisted: bool,
}

/// "In a repository, Claude Code keys the trust on the git repository root ... In a worktree, it uses
/// the main checkout's root. Outside a repository, Claude Code keys the trust on the directory you
/// started it from." A git repository nested inside another (a submodule, a clone) is a repository of
/// its own, so it has its own key and the outer trust does not cover it.
#[must_use]
pub fn workspace_trust_key(facts: &TrustFacts) -> TrustKey {
    let key = facts
        .main_checkout_root
        .clone()
        .or_else(|| facts.repository_root.clone())
        .unwrap_or_else(|| facts.start_dir.clone());
    let persisted = key != facts.home_dir;
    TrustKey { key, persisted }
}

/// The directories above the folder whose trust extends to it: none in a repository, else `ancestors`, nearest first.
fn covering_parents(facts: &TrustFacts) -> &[String] {
    if facts.repository_root.is_some() || facts.main_checkout_root.is_some() {
        &[]
    } else {
        facts.ancestors.as_deref().unwrap_or_default()
    }
}

/// Whether the dialog was accepted for a directory above the folder whose trust extends to it.
async fn parent_trusted(
    trust: &impl WorkspaceTrust,
    facts: &TrustFacts,
    signal: Option<&Signal>,
) -> bool {
    for ancestor in covering_parents(facts) {
        if trust.is_folder_trusted(ancestor, signal).await {
            return true;
        }
    }
    false
}

/// What the state is asked with.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustStateOptions {
    /// `claude -p` or the SDK, which never shows the dialog.
    #[serde(default)]
    pub non_interactive: Option<bool>,
    /// Cancels the asking.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The folder's column of the trust table (`trustSituation`) from what the port answers: `trusted`
/// when the dialog was accepted for the key; else `non-interactive` in a `-p` or SDK session; else
/// `parent-trusted` when, outside a repository, a directory above was trusted, whose trust extends to
/// every subdirectory "apart from a git repository nested inside it" (in a repository no parent is
/// asked, so a nested repository is never covered); else `dialog`.
pub async fn workspace_trust_state(
    trust: &impl WorkspaceTrust,
    facts: &TrustFacts,
    options: &TrustStateOptions,
) -> TrustSituation {
    let signal = options.signal.as_ref();
    let non_interactive = options.non_interactive == Some(true);
    let folder_trusted = trust
        .is_folder_trusted(&workspace_trust_key(facts).key, signal)
        .await;
    let asked = if folder_trusted || non_interactive {
        false
    } else {
        parent_trusted(trust, facts, signal).await
    };
    trust_situation(TrustSituationFacts {
        folder_trusted,
        parent_trusted: asked,
        non_interactive,
        nested_repository: false,
    })
}

/// What the synchronous form is asked with.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SituationOptions {
    /// `claude -p` or the SDK, which never shows the dialog.
    #[serde(default)]
    pub non_interactive: Option<bool>,
}

/// `workspaceTrustState` for a host that knows synchronously whether the dialog was accepted for a
/// folder (it read the state file already): the same column, the same folders asked in the same order.
pub async fn workspace_trust_situation(
    is_folder_trusted: Callback,
    facts: &TrustFacts,
    options: &SituationOptions,
) -> TrustSituation {
    let non_interactive = options.non_interactive == Some(true);
    let folder_trusted = is_folder_trusted
        .call::<bool>(vec![workspace_trust_key(facts).key.into()])
        .await;
    let mut parent = false;
    if !folder_trusted && !non_interactive {
        for folder in covering_parents(facts) {
            if is_folder_trusted
                .call::<bool>(vec![folder.as_str().into()])
                .await
            {
                parent = true;
                break;
            }
        }
    }
    trust_situation(TrustSituationFacts {
        folder_trusted,
        parent_trusted: parent,
        non_interactive,
        nested_repository: false,
    })
}

/// Whether hooks of a settings file may run (hooks.md, "Workspace trust"): the trust table's
/// `settings-hooks` row. In an interactive session once the dialog was accepted "for the folder, or
/// for a parent directory whose trust extends to it"; in a `-p` or SDK session always, because it
/// "never shows the dialog and treats the folder as trusted".
#[must_use]
pub const fn settings_hooks_may_run(situation: TrustSituation) -> bool {
    content_gate(RepositoryContent::SettingsHooks, situation).applies
}

/// The entry that trusts a folder without the dialog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ManualTrustEntry {
    /// The file.
    pub file: &'static str,
    /// The setting's path.
    pub path: [String; 3],
    /// What to set.
    pub value: bool,
}

/// The entry that trusts a folder without the dialog: `projects["<path>"].hasTrustDialogAccepted`
/// set to `true` in `~/.claude.json`, `<path>` being the key.
#[must_use]
pub fn manual_trust_entry(key: &str) -> ManualTrustEntry {
    ManualTrustEntry {
        file: "~/.claude.json",
        path: [
            "projects".to_owned(),
            key.to_owned(),
            "hasTrustDialogAccepted".to_owned(),
        ],
        value: true,
    }
}

/// What the warning is written from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntrustedInput {
    /// How many entries were skipped.
    pub count: f64,
    /// `permissions.allow` or `permissions.additionalDirectories`.
    pub setting: String,
    /// The settings file.
    pub file: String,
    /// The `projects` key of the folder.
    pub key: String,
    /// The path of `~/.claude.json` as the host shows it.
    pub claude_json: String,
}

/// The warning for project allow rules or additional directories a folder that is not trusted
/// cannot supply (errors.md, "Workspace has not been trusted"). `count` and `file` say how many
/// entries of which settings file were skipped, `setting` is `permissions.allow` or
/// `permissions.additionalDirectories`, `key` is the `projects` key of the folder and `claudeJson`
/// the path of `~/.claude.json` as the host shows it.
#[must_use]
pub fn untrusted_workspace_warning(input: &UntrustedInput) -> String {
    let noun = if input.count.to_bits() == 1.0_f64.to_bits() {
        "entry"
    } else {
        "entries"
    };
    format!(
        "Ignoring {} {} {noun} from {}: this workspace has not been trusted. Run Claude Code interactively here once and accept the trust dialog, or set projects[\"{}\"].hasTrustDialogAccepted: true in {}.",
        crate::js::number_to_string(input.count),
        input.setting,
        input.file,
        input.key,
        input.claude_json
    )
}

crate::registry::exports! {
    "loading/workspace-trust" {
        "workspaceTrustKey" => ok |facts: TrustFacts| workspace_trust_key(&facts);
        "workspaceTrustState" => async ok |trust: Port, facts: TrustFacts, options: Option<TrustStateOptions>|
            workspace_trust_state(&trust, &facts, &options.unwrap_or_default()).await;
        "workspaceTrustSituation" => async ok |is_folder_trusted: Callback, facts: TrustFacts, options: Option<SituationOptions>|
            workspace_trust_situation(is_folder_trusted, &facts, &options.unwrap_or_default()).await;
        "settingsHooksMayRun" => ok |situation: TrustSituation| settings_hooks_may_run(situation);
        "manualTrustEntry" => ok |key: String| manual_trust_entry(&key);
        "untrustedWorkspaceWarning" => ok |input: UntrustedInput| untrusted_workspace_warning(&input);
    }
}
