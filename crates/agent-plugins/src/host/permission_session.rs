//! Port of `src/host/permission-session.ts`, switched.
//!
//! The rules, the starting mode and the session facts of the permission settings as the input of
//! `decide` (docs/API.md, "Permissions in a session"): where the settings (layer 3) and the rules
//! (layer 3) meet. Pure over the settings policy and what the host knows of the launch. The policy
//! is the host's object (`PermissionsPolicy`, with its `startingMode`): its data members are read
//! by name, and the launch's match context is kept as the host wrote it.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::js::JsValue;
use crate::registry::Port;
use crate::rules::decide::context::PermissionMode;
use crate::rules::parse::{RuleOrigin, parse_rule};
use crate::rules::types::{PermissionBehavior, PermissionRule, RuleSource};
use crate::settings::general::permission_rules::PolicyScope;

/// Where a settings file's rules are written, for the parser and the path anchors (permissions.md,
/// "Settings precedence").
fn rule_source(scope: PolicyScope) -> RuleSource {
    match scope {
        PolicyScope::Managed => RuleSource::Managed,
        PolicyScope::Parent => RuleSource::EmbedderManaged,
        PolicyScope::Flag => RuleSource::FlagSettings {
            file: "--settings".to_owned(),
        },
        PolicyScope::Local => RuleSource::Local,
        PolicyScope::Project | PolicyScope::AddDir => RuleSource::Project,
        PolicyScope::User => RuleSource::User,
    }
}

/// Rules the launch adds: `--allowedTools` and `--disallowedTools` (permissions.md, "Settings
/// precedence").
#[derive(Clone, Debug, Default, Deserialize)]
pub struct CommandLineRules {
    /// `--allowedTools`.
    #[serde(default)]
    pub allowed: Option<Vec<String>>,
    /// `--disallowedTools`.
    #[serde(default)]
    pub disallowed: Option<Vec<String>>,
}

/// What a session's permissions start from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSessionInput {
    /// The settings' `PermissionsPolicy`, the host's object.
    pub policy: Port,
    /// The launch (`StartInput`), handed to the policy as written.
    pub start: JsValue,
    /// The folders rules are read against; the settings' `additionalDirectories` are added to the
    /// session's own.
    pub paths: JsValue,
    /// The rest of the match context.
    #[serde(default, rename = "match")]
    pub match_context: JsValue,
    /// The launch's rules.
    #[serde(default)]
    pub command_line: Option<CommandLineRules>,
    /// A `--permission-prompt-tool` or an SDK `canUseTool` callback answers what would prompt; not
    /// asked under `permissions.denyInsteadOfPrompting`.
    #[serde(default)]
    pub permission_callback: Option<bool>,
    /// Someone can answer a prompt in this session. Defaults to a `terminal` surface; a host with
    /// its own approval interface (a web or desktop chat) says `true` without claiming to be a
    /// terminal, which would also give it the built-in `auto` default (spec/judgments.md).
    #[serde(default)]
    pub interactive: Option<bool>,
}

/// A rule as written in a settings file (`SettingsRule`).
#[derive(Clone, Debug, Deserialize)]
struct SettingsRule {
    behavior: PermissionBehavior,
    text: String,
    scope: PolicyScope,
}

/// A `permissions.additionalDirectories` entry (`SettingsDirectory`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsDirectory {
    path: String,
    repository_only: bool,
}

/// What `decide` reads of the auto mode settings (`AutoModeSettings`).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoModeFlags {
    classify_all_shell: bool,
    use_auto_mode_during_plan: bool,
}

/// The policy's `startingMode` answer (`StartingMode`), as far as a session reads it.
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "outcome",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum StartingMode {
    Started {
        mode: PermissionMode,
        bypass_available: bool,
        auto_available: bool,
        notices: Vec<String>,
        issues: Vec<PluginIssue>,
    },
    Refused {
        reason: String,
    },
}

/// What `decide` takes besides the call: the mode, the rules and the facts the settings decide
/// (`DecideContext`, the fields a session sets).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's JSON shape: one fact per flag"
)]
pub struct DecideContext {
    mode: PermissionMode,
    rules: Vec<PermissionRule>,
    #[serde(rename = "match")]
    match_context: JsValue,
    auto_available: bool,
    bypass_available: bool,
    interactive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    permission_callback: Option<bool>,
    cannot_prompt: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    deny_instead_of_prompting: Option<bool>,
    use_auto_mode_during_plan: bool,
    classify_all_shell: bool,
    repository_directories: Vec<String>,
    block_reads_outside_working_directories: bool,
    block_writes_outside_working_directories: bool,
}

/// A start the settings allow or refuse (`StartedPermissions`).
#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "outcome",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StartedPermissions {
    /// The session's mode, the context `decide` reads, the start notices and the issues.
    Started {
        /// The mode.
        mode: PermissionMode,
        /// What `decide` takes besides the call.
        context: DecideContext,
        /// The notices the session shows at its start, verbatim from the docs.
        notices: Vec<String>,
        /// The issues.
        issues: Vec<PluginIssue>,
    },
    /// Why the settings refuse the start.
    Refused {
        /// The reason.
        reason: String,
    },
}

/// The rules of the settings and the launch, parsed.
fn parsed(
    entries: &[SettingsRule],
    command_line: CommandLineRules,
    managed_only: bool,
) -> (Vec<PermissionRule>, Vec<PluginIssue>) {
    let mut cli: Vec<SettingsRule> = Vec::new();
    // "ignores `--allowedTools`" under `allowManagedPermissionRulesOnly`; `--disallowedTools` still applies.
    if !managed_only {
        cli.extend(text_rules(PermissionBehavior::Allow, command_line.allowed));
    }
    cli.extend(text_rules(
        PermissionBehavior::Deny,
        command_line.disallowed,
    ));
    let results = entries
        .iter()
        .map(|entry| {
            let at = RuleOrigin {
                behavior: entry.behavior,
                source: rule_source(entry.scope),
            };
            parse_rule(&entry.text, &at)
        })
        .chain(cli.iter().map(|entry| {
            let at = RuleOrigin {
                behavior: entry.behavior,
                source: RuleSource::Cli,
            };
            parse_rule(&entry.text, &at)
        }));
    let mut rules = Vec::new();
    let mut issues = Vec::new();
    for result in results {
        rules.extend(result.rule);
        issues.extend(result.issues);
    }
    (rules, issues)
}

fn text_rules(
    behavior: PermissionBehavior,
    texts: Option<Vec<String>>,
) -> impl Iterator<Item = SettingsRule> {
    texts
        .unwrap_or_default()
        .into_iter()
        .map(move |text| SettingsRule {
            behavior,
            text,
            scope: PolicyScope::Flag,
        })
}

/// `{ ...input.match, paths: { ...paths, additionalDirectories } }`.
fn match_of(input: &PermissionSessionInput, additional: Vec<String>) -> JsValue {
    let mut paths = input.paths.as_object().cloned().unwrap_or_default();
    paths.insert(
        "additionalDirectories".to_owned(),
        additional
            .into_iter()
            .map(JsValue::String)
            .collect::<Vec<_>>()
            .into(),
    );
    let mut context = input.match_context.as_object().cloned().unwrap_or_default();
    context.insert("paths".to_owned(), paths.into());
    context.into()
}

/// Starts the permission side of a session: the mode, then the context `decide` reads. A start the
/// settings refuse (`--dangerously-skip-permissions` with `disableBypassPermissionsMode`, a
/// background bypass request nobody accepted) comes back as `refused`.
pub async fn start_permissions(input: PermissionSessionInput) -> StartedPermissions {
    let policy = input.policy;
    let start: StartingMode = policy
        .method("startingMode", vec![input.start.clone()])
        .await;
    let (mode, auto_available, bypass_available, notices, start_issues) = match start {
        StartingMode::Refused { reason } => return StartedPermissions::Refused { reason },
        StartingMode::Started {
            mode,
            bypass_available,
            auto_available,
            notices,
            issues,
        } => (mode, auto_available, bypass_available, notices, issues),
    };
    let settings_rules: Vec<SettingsRule> = policy.member("rules").await;
    let managed_rules_only: bool = policy.member("managedRulesOnly").await;
    let (rules, issues) = parsed(
        &settings_rules,
        input.command_line.clone().unwrap_or_default(),
        managed_rules_only,
    );
    let directories: Vec<SettingsDirectory> = policy.member("additionalDirectories").await;
    let own: Vec<String> = input
        .paths
        .get("additionalDirectories")
        .and_then(JsValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let additional: IndexSet<String> = own
        .into_iter()
        .chain(directories.iter().map(|each| each.path.clone()))
        .collect();
    let surface = input.start.get("surface").and_then(JsValue::as_str);
    let interactive = input
        .interactive
        .unwrap_or_else(|| surface == Some("terminal"));
    // `permissions.denyInsteadOfPrompting`: "Claude Code doesn't ask your `canUseTool` callback or
    // `--permission-prompt-tool`" (settings-reference.md, "What else `denyInsteadOfPrompting` stops").
    let unprompted: bool = policy.member("denyInsteadOfPrompting").await;
    let callback = input.permission_callback == Some(true) && !unprompted;
    let auto_mode: AutoModeFlags = policy.member("autoMode").await;
    let context = DecideContext {
        mode,
        rules,
        match_context: match_of(&input, additional.into_iter().collect()),
        auto_available,
        bypass_available,
        interactive,
        permission_callback: callback.then_some(true),
        // "`claude -p` and Agent SDK runs ... Calls needing approval are denied unless you approve
        // them with an allow rule, hook, `canUseTool` callback, or `--permission-prompt-tool`."
        cannot_prompt: surface == Some("headless") && !callback && !interactive,
        deny_instead_of_prompting: unprompted.then_some(true),
        use_auto_mode_during_plan: auto_mode.use_auto_mode_during_plan,
        classify_all_shell: auto_mode.classify_all_shell,
        repository_directories: directories
            .into_iter()
            .filter(|each| each.repository_only)
            .map(|each| each.path)
            .collect(),
        block_reads_outside_working_directories: policy
            .member("blockReadsOutsideWorkingDirectories")
            .await,
        block_writes_outside_working_directories: policy
            .member("blockWritesOutsideWorkingDirectories")
            .await,
    };
    StartedPermissions::Started {
        mode,
        context,
        notices,
        issues: start_issues.into_iter().chain(issues).collect(),
    }
}

crate::registry::exports! {
    "host/permission-session" {
        "startPermissions" => async ok |input: PermissionSessionInput| start_permissions(input).await;
    }
}
