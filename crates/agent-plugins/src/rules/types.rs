//! The core's counterpart of `src/rules/types.ts` at sha256:1e21dfa4a0ff, which stays TypeScript: it declares types only, so it has nothing to run (docs/RUST.md, "What stays TypeScript").
//!
//! The contracts every part of the rules module shares (docs/ARCHITECTURE.md, "Layers"). A rule is
//! the official `PermissionRuleValue` (`{ toolName, ruleContent? }`) plus what the kit reads out of
//! it once, so matching never re-parses text. The SDK's `PermissionBehavior` and
//! `PermissionRuleValue`, which it imports, are declared here too.

use serde::{Deserialize, Serialize};

/// The SDK's `PermissionBehavior`: what a rule does to a call it matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    /// `allow`.
    Allow,
    /// `deny`.
    Deny,
    /// `ask`.
    Ask,
}

/// The SDK's `PermissionRuleValue`: the tool-name position and the text between the parentheses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRuleValue {
    /// The tool-name position.
    pub tool_name: String,
    /// The text between the parentheses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

/// Where a rule was written (permissions.md, "Settings precedence" and "Managed settings"). It
/// matters to the parser (a settings file skips an `mcp__` rule with parentheses, the command line
/// does not) and to the path rules, where `/path` anchors differently per source; each such table is
/// a registry typed over the `kind`s (`sources.ts`), never a comparison. `managed` is the managed
/// tier (server-managed settings, MDM, `managed-settings.json`), the highest: no other source can
/// override its rules. `embedder-managed` is the policy an embedding host passes through the SDK
/// `managedSettings` option.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RuleSource {
    /// `managed`.
    Managed,
    /// `embedder-managed`.
    EmbedderManaged,
    /// `project`.
    Project,
    /// `local`.
    Local,
    /// `user`.
    User,
    /// `flag-settings`, with the file.
    FlagSettings {
        /// The `--settings` file.
        file: String,
    },
    /// `cli`.
    Cli,
    /// `session`.
    Session,
}

/// What follows the tool name inside the parentheses, once read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "form", rename_all = "lowercase")]
pub enum Specifier {
    /// `Agent(model:opus)`: one top-level input parameter (permissions.md, "Match by input parameter").
    Param {
        /// The parameter.
        name: String,
        /// Its value pattern.
        value: String,
    },
    /// `WebFetch(domain:*.example.com)`: the `domain:` prefix is part of the form, not the pattern.
    Domain {
        /// The domain pattern.
        pattern: String,
    },
    /// A command, path, agent or skill pattern, as the tool's own format reads it.
    Text {
        /// The pattern.
        pattern: String,
    },
}

/// One permission rule, parsed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRule {
    /// The official shape: the tool-name position and the text between the parentheses.
    pub value: PermissionRuleValue,
    /// What the rule does.
    pub behavior: PermissionBehavior,
    /// Where it was written.
    pub source: RuleSource,
    /// The tool-name position holds a glob (`mcp__*`, `*`): permissions.md, "Tool name wildcards".
    pub tool_glob: bool,
    /// `undefined` is a bare tool name: it covers every use of the tool (`Bash(*)` reads as bare).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specifier: Option<Specifier>,
    /// The rule as it was written, trimmed: what a warning repeats ("as you wrote it", errors.md).
    pub text: String,
}

/// `PathContext['platform']`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// `posix`.
    Posix,
    /// `windows`.
    Windows,
}

/// Why `decide` answered as it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(missing_docs, reason = "each variant is its kebab-case string")]
pub enum DecisionReason {
    DenyRule,
    AskRule,
    AllowRule,
    ReadInWorkingDirectory,
    ReadOnlyCommand,
    PreapprovedDomain,
    AcceptEdits,
    ProtectedPath,
    NetworkPath,
    RequiresUserInteraction,
    OrganizationAsk,
    PromptUnavailable,
    NoApprovalSurface,
    ModeBypass,
    ModeDontask,
    ModePlan,
    ModeAuto,
    Hook,
    Mod,
    DefaultPrompt,
    ToolRemoved,
    NoPermissionRequired,
    CriticalPath,
    OutsideWorkingDirectory,
    SettingsFile,
    UnresolvablePath,
    PowershellRemoval,
    ForeignArtifact,
}

/// The outcome of a permission check. `classify` means the host's reviewer decides (auto mode): the
/// kit holds no classifier (permissions.md, "Permission modes").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum Decision {
    /// `allow`.
    Allow {
        /// Why.
        reason: DecisionReason,
    },
    /// `ask`.
    Ask {
        /// Why.
        reason: DecisionReason,
    },
    /// `deny`.
    Deny {
        /// Why.
        reason: DecisionReason,
    },
    /// `classify`.
    Classify {
        /// Why.
        reason: DecisionReason,
        /// The reviewer's approval does not run the call: it "still goes to the prompt" (hooks.md,
        /// "PreToolUse `ask` decisions in auto mode"). Its denial stands.
        #[serde(
            default,
            rename = "thenPrompt",
            skip_serializing_if = "Option::is_none"
        )]
        then_prompt: Option<bool>,
    },
}

/// The folders and platform path rules are read against. Plain data the host supplies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathContext {
    /// The session's current directory.
    pub cwd: String,
    /// The home directory.
    pub home: String,
    /// The primary working directory.
    pub primary_working_directory: String,
    /// The additional directories.
    pub additional_directories: Vec<String>,
    /// How the host writes paths.
    pub platform: Platform,
    /// Directories as a deny or ask rule spells them (its anchor, among them the current directory, and
    /// the directory it names) and the working directories, mapped to where they really are (`/etc` to
    /// `/private/etc`), built with `resolveRuleDirectories`: "a deny or ask rule written through a
    /// symlinked directory ... also applies at the directory's real location" (permissions.md, "How rules
    /// match a symlinked path"). Allow rules never read it. Absent when nothing goes through a link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub real_directories: Option<crate::registry::Map<String, String>>,
}

/// A tool call as a rule sees it: the canonical tool name and the input Claude sent.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    /// The tool's name.
    pub tool_name: String,
    /// What the tool is called with.
    #[serde(default)]
    pub input: crate::js::JsValue,
}

/// A tool call with what the host resolved for it (`PreparedCall`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedCall {
    /// The call.
    #[serde(flatten)]
    pub call: ToolCall,
    /// The paths the call resolves to.
    #[serde(default)]
    pub resolved_paths: Option<Vec<String>>,
    /// Every name the called skill goes by (its alias, its display name, a qualified form), which a
    /// `Skill(skill:<name>)` deny rule matches (permissions.md, "Match by input parameter"). The
    /// host knows its skills; the kit holds no skill catalog.
    #[serde(default)]
    pub skill_names: Option<Vec<String>>,
    /// The hosts a sandboxed Bash, PowerShell or Monitor command names for itself in auto mode
    /// (sandboxing.md, "Per-command allowed domains in auto mode"). The SDK types no field for
    /// them, so the host reads them from its input. A command that carries hosts "goes to the
    /// classifier instead of being approved by a permission rule".
    #[serde(default)]
    pub allowed_domains: Option<Vec<String>>,
}

/// Everything a rule needs beside the call itself (`MatchContext`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchContext {
    /// The folders and platform.
    pub paths: PathContext,
    /// Environment variables an allow rule may see through when it strips `VAR=value` prefixes.
    #[serde(default)]
    pub safe_env_vars: Option<crate::registry::Set<String>>,
    /// The rule type whose depth rule reads a single-directory relative path pattern, when it is
    /// not the rule's own behaviour.
    #[serde(default)]
    pub depth_of: Option<PermissionBehavior>,
}
