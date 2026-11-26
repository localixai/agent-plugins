//! Port of `src/loading/trust-gates.ts`, switched.
//!
//! What runs before a folder is trusted (permissions.md, "What runs before you trust a folder"): one
//! row of the docs' table per kind of repository content, with the two situations in which the folder
//! itself is not trusted: only a parent folder was trusted, or `claude -p` or the SDK ran there, which
//! never shows the trust dialog. Pure: the settings and MCP streams ask `contentGate`.

use serde::{Deserialize, Serialize};

/// The kinds of repository content the table lists.
pub const REPOSITORY_CONTENT: [RepositoryContent; 12] = [
    RepositoryContent::SettingsHooks,
    RepositoryContent::EnvAndHelperCommands,
    RepositoryContent::SkillHooks,
    RepositoryContent::SkillAllowedTools,
    RepositoryContent::AllowRules,
    RepositoryContent::AdditionalDirectories,
    RepositoryContent::SubagentFrontmatterHooks,
    RepositoryContent::SkillsDirPlugin,
    RepositoryContent::ExtraKnownMarketplaces,
    RepositoryContent::SubagentInlineMcp,
    RepositoryContent::McpJsonServers,
    RepositoryContent::HeadersHelper,
];

/// One of `REPOSITORY_CONTENT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepositoryContent {
    /// Hooks of settings files.
    SettingsHooks,
    /// `env` and helper commands.
    EnvAndHelperCommands,
    /// A skill's hooks.
    SkillHooks,
    /// A skill's `allowed-tools`.
    SkillAllowedTools,
    /// `permissions.allow` rules.
    AllowRules,
    /// `permissions.additionalDirectories`.
    AdditionalDirectories,
    /// A subagent's frontmatter hooks.
    SubagentFrontmatterHooks,
    /// A skills-directory plugin.
    SkillsDirPlugin,
    /// `extraKnownMarketplaces`.
    ExtraKnownMarketplaces,
    /// A subagent's inline MCP servers.
    SubagentInlineMcp,
    /// The servers of `.mcp.json`.
    McpJsonServers,
    /// `headersHelper`.
    HeadersHelper,
}

/// What a gate asks of the user before the content takes effect: nothing, the trust dialog (shown
/// again, naming the content), a per-server approval, or never (no dialog is offered).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Consent {
    /// Nothing.
    None,
    /// The trust dialog.
    TrustDialog,
    /// A per-server approval.
    ServerApproval,
    /// No dialog is offered.
    Never,
}

/// What is used while the content waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fallback {
    /// The headers the file holds.
    StaticHeaders,
}

/// What the table says about one kind of content in one situation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentGate {
    /// The content takes effect now.
    pub applies: bool,
    /// What the gate asks of the user.
    pub consent: Consent,
    /// What is used meanwhile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Fallback>,
    /// The line Claude Code prints to stderr, as the docs name it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<&'static str>,
    /// Whether approvals the repository commits in its own settings count (`.mcp.json` servers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_approvals: Option<bool>,
}

/// The columns of the table, and the folder's other states.
pub const TRUST_SITUATIONS: [TrustSituation; 4] = [
    TrustSituation::Trusted,
    TrustSituation::ParentTrusted,
    TrustSituation::NonInteractive,
    TrustSituation::Dialog,
];

/// One of `TRUST_SITUATIONS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustSituation {
    /// The dialog was accepted for this folder.
    Trusted,
    /// Only a parent folder was trusted.
    ParentTrusted,
    /// `claude -p` or the SDK.
    NonInteractive,
    /// The dialog is still to come.
    Dialog,
}

/// One gate per situation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Row {
    /// The folder is trusted.
    pub trusted: ContentGate,
    /// Only a parent is trusted.
    pub parent_trusted: ContentGate,
    /// `claude -p` or the SDK.
    pub non_interactive: ContentGate,
    /// The dialog is still to come.
    pub dialog: ContentGate,
}

impl Row {
    /// The gate of a situation.
    const fn at(&self, situation: TrustSituation) -> ContentGate {
        match situation {
            TrustSituation::Trusted => self.trusted,
            TrustSituation::ParentTrusted => self.parent_trusted,
            TrustSituation::NonInteractive => self.non_interactive,
            TrustSituation::Dialog => self.dialog,
        }
    }
}

const fn gate(applies: bool, consent: Consent) -> ContentGate {
    ContentGate {
        applies,
        consent,
        fallback: None,
        stderr: None,
        repository_approvals: None,
    }
}

const USED: ContentGate = gate(true, Consent::None);
const WAITS: ContentGate = gate(false, Consent::TrustDialog);
const NEVER: ContentGate = gate(false, Consent::Never);

/// A row that is used in a trusted folder and in both columns of the table, and waits for the dialog before.
const fn always_used(before: ContentGate) -> Row {
    Row {
        trusted: USED,
        parent_trusted: USED,
        non_interactive: USED,
        dialog: before,
    }
}

/// A row not used in either column and never offered a dialog.
const NEVER_UNTIL_TRUSTED: Row = Row {
    trusted: USED,
    parent_trusted: NEVER,
    non_interactive: NEVER,
    dialog: WAITS,
};

const ALLOW_RULES: Row = Row {
    trusted: USED,
    parent_trusted: WAITS,
    non_interactive: ContentGate {
        stderr: Some("this workspace has not been trusted"),
        ..NEVER
    },
    dialog: WAITS,
};

const STATIC_HEADERS: Option<Fallback> = Some(Fallback::StaticHeaders);

/// One row per kind of content. Judgment (spec/judgments.md): the table has no column for a folder
/// that nothing was trusted for in an interactive session (`dialog`), nor for a trusted one; there the
/// content that needs the exact folder waits for the dialog the session shows, hooks of settings
/// files do too (hooks.md, "Workspace trust"), and what the table calls "Used" in both columns, a
/// skill's `allowed-tools` (never gated "in any session"), is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ContentGates {
    /// Hooks of settings files.
    pub settings_hooks: Row,
    /// `env` and helper commands.
    pub env_and_helper_commands: Row,
    /// A skill's hooks.
    pub skill_hooks: Row,
    /// A skill's `allowed-tools`.
    pub skill_allowed_tools: Row,
    /// `permissions.allow` rules.
    pub allow_rules: Row,
    /// `permissions.additionalDirectories`.
    pub additional_directories: Row,
    /// A subagent's frontmatter hooks.
    pub subagent_frontmatter_hooks: Row,
    /// A skills-directory plugin.
    pub skills_dir_plugin: Row,
    /// `extraKnownMarketplaces`.
    pub extra_known_marketplaces: Row,
    /// A subagent's inline MCP servers.
    pub subagent_inline_mcp: Row,
    /// The servers of `.mcp.json`.
    pub mcp_json_servers: Row,
    /// `headersHelper`.
    pub headers_helper: Row,
}

/// The table.
pub const CONTENT_GATES: ContentGates = ContentGates {
    settings_hooks: always_used(WAITS),
    env_and_helper_commands: always_used(WAITS),
    skill_hooks: always_used(WAITS),
    skill_allowed_tools: always_used(USED),
    allow_rules: ALLOW_RULES,
    additional_directories: ALLOW_RULES,
    subagent_frontmatter_hooks: NEVER_UNTIL_TRUSTED,
    skills_dir_plugin: NEVER_UNTIL_TRUSTED,
    extra_known_marketplaces: NEVER_UNTIL_TRUSTED,
    subagent_inline_mcp: NEVER_UNTIL_TRUSTED,
    mcp_json_servers: Row {
        trusted: ContentGate {
            repository_approvals: Some(true),
            ..gate(false, Consent::ServerApproval)
        },
        parent_trusted: ContentGate {
            repository_approvals: Some(false),
            ..gate(false, Consent::ServerApproval)
        },
        non_interactive: USED,
        dialog: ContentGate {
            repository_approvals: Some(false),
            ..gate(false, Consent::ServerApproval)
        },
    },
    headers_helper: Row {
        trusted: USED,
        parent_trusted: ContentGate {
            fallback: STATIC_HEADERS,
            ..WAITS
        },
        non_interactive: ContentGate {
            fallback: STATIC_HEADERS,
            stderr: Some("headersHelper not run"),
            ..NEVER
        },
        dialog: ContentGate {
            fallback: STATIC_HEADERS,
            ..WAITS
        },
    },
};

impl ContentGates {
    /// The row of one kind of content.
    const fn of(&self, content: RepositoryContent) -> &Row {
        match content {
            RepositoryContent::SettingsHooks => &self.settings_hooks,
            RepositoryContent::EnvAndHelperCommands => &self.env_and_helper_commands,
            RepositoryContent::SkillHooks => &self.skill_hooks,
            RepositoryContent::SkillAllowedTools => &self.skill_allowed_tools,
            RepositoryContent::AllowRules => &self.allow_rules,
            RepositoryContent::AdditionalDirectories => &self.additional_directories,
            RepositoryContent::SubagentFrontmatterHooks => &self.subagent_frontmatter_hooks,
            RepositoryContent::SkillsDirPlugin => &self.skills_dir_plugin,
            RepositoryContent::ExtraKnownMarketplaces => &self.extra_known_marketplaces,
            RepositoryContent::SubagentInlineMcp => &self.subagent_inline_mcp,
            RepositoryContent::McpJsonServers => &self.mcp_json_servers,
            RepositoryContent::HeadersHelper => &self.headers_helper,
        }
    }
}

/// What the host knows of the folder's trust.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four facts the docs' table reads"
)]
pub struct TrustSituationFacts {
    /// The trust dialog was accepted for this exact folder.
    pub folder_trusted: bool,
    /// Only a parent folder was trusted.
    pub parent_trusted: bool,
    /// `claude -p` or the SDK.
    pub non_interactive: bool,
    /// The folder is a git repository nested inside the trusted one (a submodule, a clone).
    pub nested_repository: bool,
}

/// Which column applies. "The parent-folder column doesn't apply inside a nested repository: in an
/// interactive session Claude Code shows the trust dialog for it, and a `claude -p` or SDK run there
/// follows the `claude -p` column."
#[must_use]
pub const fn trust_situation(facts: TrustSituationFacts) -> TrustSituation {
    if facts.folder_trusted {
        return TrustSituation::Trusted;
    }
    if facts.non_interactive {
        return TrustSituation::NonInteractive;
    }
    if facts.parent_trusted && !facts.nested_repository {
        TrustSituation::ParentTrusted
    } else {
        TrustSituation::Dialog
    }
}

/// The gate of one kind of content in one situation.
#[must_use]
pub const fn content_gate(content: RepositoryContent, situation: TrustSituation) -> ContentGate {
    CONTENT_GATES.of(content).at(situation)
}

/// The kinds of entry a project settings file's `permissions` holds, and whether each grants capability.
pub const PROJECT_PERMISSION_ENTRIES: [ProjectPermissionEntry; 4] = [
    ProjectPermissionEntry::Allow,
    ProjectPermissionEntry::Ask,
    ProjectPermissionEntry::Deny,
    ProjectPermissionEntry::AdditionalDirectories,
];

/// One of `PROJECT_PERMISSION_ENTRIES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectPermissionEntry {
    /// `allow`.
    Allow,
    /// `ask`.
    Ask,
    /// `deny`.
    Deny,
    /// `additionalDirectories`.
    AdditionalDirectories,
}

impl ProjectPermissionEntry {
    /// Whether the entry grants capability.
    const fn grants(self) -> bool {
        match self {
            Self::Allow | Self::AdditionalDirectories => true,
            Self::Ask | Self::Deny => false,
        }
    }
}

/// "`permissions.allow` rules and `permissions.additionalDirectories` entries in a project's
/// `.claude/settings.json` grant capability, so Claude Code applies them only after you accept the
/// workspace trust dialog for that folder ... `deny` and `ask` rules aren't affected, since they only
/// restrict."
#[must_use]
pub const fn project_entry_applies(entry: ProjectPermissionEntry, folder_trusted: bool) -> bool {
    !entry.grants() || folder_trusted
}

/// The part of `permissions` the dialog lists.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogPermissions {
    /// The allow rules.
    #[serde(default)]
    pub allow: Option<Vec<String>>,
    /// The added directories.
    #[serde(default)]
    pub additional_directories: Option<Vec<String>>,
}

/// What the dialog lists "so you can review them first": the rules and directories the folder would grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustDialogListing {
    /// The allow rules.
    pub allow: Vec<String>,
    /// The added directories.
    pub additional_directories: Vec<String>,
}

/// What the dialog lists "so you can review them first": the rules and directories the folder would grant.
#[must_use]
pub fn trust_dialog_listing(permissions: DialogPermissions) -> TrustDialogListing {
    TrustDialogListing {
        allow: permissions.allow.unwrap_or_default(),
        additional_directories: permissions.additional_directories.unwrap_or_default(),
    }
}

crate::registry::exports! {
    "loading/trust-gates" {
        "REPOSITORY_CONTENT" => ok || REPOSITORY_CONTENT;
        "TRUST_SITUATIONS" => ok || TRUST_SITUATIONS;
        "CONTENT_GATES" => ok || CONTENT_GATES;
        "trustSituation" => ok |facts: TrustSituationFacts| trust_situation(facts);
        "contentGate" => ok |content: RepositoryContent, situation: TrustSituation| content_gate(content, situation);
        "PROJECT_PERMISSION_ENTRIES" => ok || PROJECT_PERMISSION_ENTRIES;
        "projectEntryApplies" => ok |entry: ProjectPermissionEntry, folder_trusted: bool| project_entry_applies(entry, folder_trusted);
        "trustDialogListing" => ok |permissions: DialogPermissions| trust_dialog_listing(permissions);
    }
}
