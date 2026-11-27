//! The core's counterpart of `src/rules/decide/context.ts` at sha256:f1743ec61972, which stays TypeScript: it declares types only, so it has nothing to run (docs/RUST.md, "What stays TypeScript").
//!
//! What `decide` reads beside the call: the session facts decided outside `decide`, the rules in
//! force, and the SDK's two types it imports (the session's permission mode and a hook's permission
//! decision).

use serde::{Deserialize, Serialize};

use crate::registry::{Callback, Set};
use crate::rules::types::{Decision, MatchContext, PermissionRule};

/// The SDK's `PermissionMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// `default`.
    Default,
    /// `acceptEdits`.
    AcceptEdits,
    /// `bypassPermissions`.
    BypassPermissions,
    /// `plan`.
    Plan,
    /// `dontAsk`.
    DontAsk,
    /// `auto`.
    Auto,
}

/// The SDK's `HookPermissionDecision`: a PreToolUse hook's `permissionDecision`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookPermissionDecision {
    /// `allow`.
    Allow,
    /// `deny`.
    Deny,
    /// `ask`.
    Ask,
    /// `defer`.
    Defer,
}

/// The paths beside the protected lists that a session protects (`ProtectedPathOptions`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedPathOptions {
    /// Files passed to `--settings`, `--mcp-config` or `--agents`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    /// The files that a symlinked settings file points to. Unlike `files`, a write to one prompts
    /// in the modes that route protected-path writes to the classifier, "even when an allow rule
    /// matches".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symlink_targets: Option<Vec<String>>,
    /// The directory `CLAUDE_CONFIG_DIR` points to, in place of `~/.claude` (default
    /// `~/.claude`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_directory: Option<String>,
    /// Directories loaded with `--plugin-dir`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directories: Option<Vec<String>>,
    /// The `plansDirectory` setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plans_directory: Option<String>,
    /// The signature, sync and consent files the host keeps beside `remote-settings.json`, by
    /// path. The docs name them by role only, so the kit protects these and guesses no name (Agent
    /// Plugins Extended J13.35).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_settings_files: Option<Vec<String>>,
    /// A session started with `--restricted`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restricted: Option<bool>,
}

/// What a PreToolUse hook answered (hooks.md, "PreToolUse decision control"): its
/// `permissionDecision`, whether the hook exited with code 2 (a block), and whether it returned
/// `updatedInput` beside an allow. `defer` is the hook asking the calling process to pause the
/// turn, so it is not an answer to the permission check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseHookOutcome {
    /// `decision`.
    #[serde(default)]
    pub decision: Option<HookPermissionDecision>,
    /// "A hook that exits with code 2 stops the tool call before permission rules are evaluated."
    #[serde(default)]
    pub blocked: Option<bool>,
    /// "`AskUserQuestion` and `ExitPlanMode` need `updatedInput` paired with `\"allow\"`."
    #[serde(default)]
    pub has_updated_input: Option<bool>,
    /// The block or `ask` comes from a hook in managed settings: no mod's approval lifts it, with
    /// or without `allowModsToOverrideDenyRules` (Agent Plugins Extended J14.14).
    #[serde(default)]
    pub managed: Option<bool>,
}

/// What a mod that handles `tool.check` answered (permissions.md, "Precedence among rules, hooks,
/// and mods"). `policyEnforced` is "managed settings on the machine, or a Team or Enterprise
/// `/login` as the session's credential": the setups where a mod's approval cannot undo a deny
/// rule, an ask rule or a hook's block or prompt. `blocked` is a mod that refuses the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModOutcome {
    /// `approved`.
    pub approved: bool,
    /// `policyEnforced`.
    pub policy_enforced: bool,
    /// `blocked`.
    #[serde(default)]
    pub blocked: Option<bool>,
    /// `allowModsToOverrideDenyRules` is on for this mod: under `policyEnforced` its approval still
    /// lifts what permission rules, the read block and hooks outside managed settings decided,
    /// while a managed hook's decision and the prompt put to the permission callback stay (Agent
    /// Plugins Extended §14.9.6, J14.14).
    #[serde(default)]
    pub overrides_deny_rules: Option<bool>,
}

/// The session facts decided outside `decide`, and the rules in force. Each optional field is
/// something the host knows and the docs name: the kit reads no settings, environment or login.
/// The host's three functions do not cross inside it (a record that holds a function crosses as a
/// port, read member by member): the wrapper passes them beside it, and `decide` sets them here.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecideContext {
    /// `mode`.
    pub mode: PermissionMode,
    /// `rules`.
    pub rules: Vec<PermissionRule>,
    /// `match`.
    #[serde(rename = "match")]
    pub match_context: MatchContext,
    /// `autoAvailable`.
    pub auto_available: bool,
    /// `bypassAvailable`.
    pub bypass_available: bool,
    /// `interactive`.
    pub interactive: bool,
    /// `useAutoModeDuringPlan`.
    #[serde(default)]
    pub use_auto_mode_during_plan: Option<bool>,
    /// `classifyAllShell`.
    #[serde(default)]
    pub classify_all_shell: Option<bool>,
    /// `hook`.
    #[serde(default)]
    pub hook: Option<PreToolUseHookOutcome>,
    /// `mod`.
    #[serde(default, rename = "mod")]
    pub mod_outcome: Option<ModOutcome>,
    /// `repositoryDirectories`.
    #[serde(default)]
    pub repository_directories: Option<Vec<String>>,
    /// `blockReadsOutsideWorkingDirectories`.
    #[serde(default)]
    pub block_reads_outside_working_directories: Option<bool>,
    /// `blockWritesOutsideWorkingDirectories`.
    #[serde(default)]
    pub block_writes_outside_working_directories: Option<bool>,
    /// `managedSettingsFolderUnresolved`.
    #[serde(default)]
    pub managed_settings_folder_unresolved: Option<bool>,
    /// `sandboxEnforcesReadBlock`.
    #[serde(default)]
    pub sandbox_enforces_read_block: Option<bool>,
    /// `sessionDirectories`.
    #[serde(default)]
    pub session_directories: Option<Vec<String>>,
    /// `requiresPermission`: the host's function, passed beside the context.
    #[serde(skip)]
    pub requires_permission: Option<Callback>,
    /// `preapprovedFetchDomain`: the host's function, passed beside the context.
    #[serde(skip)]
    pub preapproved_fetch_domain: Option<Callback>,
    /// `requiresUserInteraction`.
    #[serde(default)]
    pub requires_user_interaction: Option<Set<String>>,
    /// `connectorToolsSetToAsk`.
    #[serde(default)]
    pub connector_tools_set_to_ask: Option<Set<String>>,
    /// `cannotPrompt`.
    #[serde(default)]
    pub cannot_prompt: Option<bool>,
    /// `denyInsteadOfPrompting`.
    #[serde(default)]
    pub deny_instead_of_prompting: Option<bool>,
    /// `permissionCallback`.
    #[serde(default)]
    pub permission_callback: Option<bool>,
    /// `criticalPathPromptOff`.
    #[serde(default)]
    pub critical_path_prompt_off: Option<bool>,
    /// `powershellCmdRemovalDenyOff`.
    #[serde(default)]
    pub powershell_cmd_removal_deny_off: Option<bool>,
    /// `foreignArtifact`: the host's function, passed beside the context.
    #[serde(skip)]
    pub foreign_artifact: Option<Callback>,
    /// `protectedPaths`.
    #[serde(default)]
    pub protected_paths: Option<ProtectedPathOptions>,
}

/// How far a mod's approval reaches over an outcome: over anything (`any`), unless a permission
/// callback cannot be bypassed (`unbypassable`), only where no managed policy applies or the guard
/// option lifts it (`unenforced`), as that and only for a hook outside managed settings (`hook`),
/// or never.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModReach {
    /// `any`.
    Any,
    /// `unbypassable`.
    Unbypassable,
    /// `unenforced`.
    Unenforced,
    /// `hook`.
    Hook,
    /// `never`.
    Never,
}

/// An outcome and how far a mod's approval reaches over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending {
    /// `decision`.
    pub decision: Decision,
    /// `reach`.
    pub reach: ModReach,
}
