//! The types of `src/settings/layers.ts` that the ported settings modules read: where a settings
//! object comes from, and a layer. The module itself (the merge and the holders of a key) is not
//! ported yet.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;

/// Where a settings object comes from, highest precedence first (settings.md, "Settings
/// precedence": managed, command line, project local, shared project, user). `flag` is the
/// `--settings` JSON. `parent` is the policy-tier settings an embedding host passes through the
/// Agent SDK's `managedSettings`; `addDir` is a `--add-dir` directory's `.claude/settings*.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PolicyScope {
    /// Managed settings.
    Managed,
    /// An embedding host's policy-tier settings.
    Parent,
    /// The `--settings` JSON.
    Flag,
    /// The project's local file.
    Local,
    /// The project's shared file.
    Project,
    /// The user's file.
    User,
    /// A `--add-dir` directory's file.
    AddDir,
}

/// The scopes, highest precedence first.
pub const POLICY_SCOPES: [PolicyScope; 7] = [
    PolicyScope::Managed,
    PolicyScope::Parent,
    PolicyScope::Flag,
    PolicyScope::Local,
    PolicyScope::Project,
    PolicyScope::User,
    PolicyScope::AddDir,
];

impl PolicyScope {
    /// The scope's name as the package writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Parent => "parent",
            Self::Flag => "flag",
            Self::Local => "local",
            Self::Project => "project",
            Self::User => "user",
            Self::AddDir => "addDir",
        }
    }
}

/// One settings source with the scope it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyLayer {
    /// Where it comes from.
    pub scope: PolicyScope,
    /// The settings object.
    pub settings: JsValue,
    /// The keys this source carried with a value Claude Code could not read and no value can
    /// stand for (managed-settings.md, "Keys that fail closed"); `None` when every key was
    /// readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalid: Option<Vec<String>>,
}
