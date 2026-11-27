//! Port of `src/settings/general/keys.ts`, switched.
//!
//! The keys beyond the policy keys the kit combines: the ones settings.md names (the whole-value
//! keys, the keys that take an object, the keys whose managed value is not an override) and the
//! blocks the other areas read. A dotted name is a key inside a block that has a rule of its own
//! (settings.md, "Exceptions to managed settings precedence"); every other key of a file is
//! combined by the general rule of `merge.ts` and needs no row. The types of `keys.ts`
//! (`GeneralKey`, `SettingsKey`, `ValueOf`) are TypeScript's; a key is its name here.

/// The general keys.
pub const GENERAL_KEYS: [&str; 34] = [
    "agent",
    "attribution",
    "autoMode.classifyAllShell",
    "availableModels",
    "crossSessionInbound",
    "disableAgentView",
    "enableArtifact",
    "env",
    "fallbackModel",
    "fileSuggestion",
    "isolatePeerMachines",
    "maxEffortLevel",
    "model",
    "modelPicker",
    "outputStyle",
    "permissions.blockReadsOutsideWorkingDirectories",
    "permissions.blockWritesOutsideWorkingDirectories",
    "permissions.defaultMode",
    "permissions.denyInsteadOfPrompting",
    "processWrapper",
    "remoteControlAtStartup",
    "sandbox",
    "sandbox.filesystem.allowManagedReadPathsOnly",
    "sandbox.filesystem.allowRead",
    "sandbox.filesystem.disabled",
    "sandbox.network.allowManagedDomainsOnly",
    "sandbox.network.allowedDomains",
    "spinnerTipsEnabled",
    "statusLine",
    "subagentStatusLine",
    "teammateMode",
    "theme",
    "worktree",
    "worktree.location",
];

crate::registry::exports! {
    "settings/general/keys" {
        "GENERAL_KEYS" => ok || GENERAL_KEYS.as_slice();
    }
}
