//! Port of `src/format/host-errors.ts`, switched.
//!
//! The notices, warnings and errors Claude Code prints for the failures errors.md documents, each in
//! the wording the page quotes (docs/SPEC.md: message wording comes from the docs). Pure string
//! builders: the host decides when a failure happened and shows the text where the page says it
//! appears (conversation notice, stderr, debug log, `/status`).

use serde::Deserialize;

use crate::js::{number_to_string, to_fixed};

/// `~16.2k`: a token count as the notice shows it, one decimal in thousands.
fn thousands(tokens: f64) -> String {
    format!("{}k", to_fixed(tokens / 1000.0, 1))
}

/// The input of [`reserved_skill_name_notice`].
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ReservedSkillNameInput {
    /// What to rename: a folder or file path, or the `name:` line to edit, as the first refused
    /// item shows it.
    pub rename: String,
    /// The reserved name the item uses: `anthropic-skills` or a name that starts with
    /// `anthropic-skills:`.
    pub name: String,
    /// How many more items were refused.
    #[serde(default)]
    pub more: Option<f64>,
}

/// The startup notice for a skill, command or workflow whose name uses `anthropic-skills`.
#[must_use]
pub fn reserved_skill_name_notice(input: &ReservedSkillNameInput) -> String {
    let more = match input.more {
        Some(count) if count > 0.0 => format!(" · {} more", number_to_string(count)),
        Some(_) | None => String::new(),
    };
    format!(
        "Not loaded: rename {}, then restart — its name uses \"{}\", a name reserved for the skills synced from your claude.ai account{more}",
        input.rename, input.name
    )
}

/// The startup notice when the agents' names and descriptions exceed 15,000 tokens.
#[must_use]
pub fn agent_descriptions_over_limit(estimated_tokens: f64, folder: &str) -> String {
    format!(
        "Agent descriptions are over the 15.0k-token limit (~{} tokens) · ask Claude to trim agent descriptions in {folder}",
        thousands(estimated_tokens)
    )
}

/// The size above which `.mcp.json` is refused: 2 MiB.
const MCP_JSON_LIMIT_BYTES: u32 = 2_097_152;

/// The error of a command that finds `.mcp.json` is not a regular file or is over 2 MiB.
#[must_use]
pub fn mcp_json_unreadable() -> String {
    format!(
        "Can't read .mcp.json: it isn't a regular file or is larger than {MCP_JSON_LIMIT_BYTES} bytes. Fix or remove it, then run the command again."
    )
}

/// `claude mcp add` and `add-json` with `--scope managed`.
#[must_use]
pub fn cannot_add_mcp_to_managed_scope() -> String {
    "Cannot add MCP server to scope: managed".to_owned()
}

/// The reasons the page lists for organization settings that could not load, as it names them.
#[must_use]
pub fn organization_settings_unavailable(reason: &str, organization: &str) -> String {
    format!(
        "Claude Code couldn't reach Anthropic to load your organization's settings ({reason}). {organization} requires them before a session can start."
    )
}

/// What the `Remote managed settings failed to load` warning says about the policy the session
/// runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemotePolicy {
    /// `cached`
    Cached,
    /// `none`
    None,
}

/// The text of each [`RemotePolicy`].
const fn remote_policy_text(policy: RemotePolicy) -> &'static str {
    match policy {
        RemotePolicy::Cached => "using cached policy",
        RemotePolicy::None => "no remote policy applied",
    }
}

/// The warning when server-managed settings could not be fetched or applied; `cause` names what
/// failed.
#[must_use]
pub fn remote_managed_settings_failed(cause: &str, policy: RemotePolicy) -> String {
    format!(
        "Remote managed settings failed to load ({cause}): {}",
        remote_policy_text(policy)
    )
}

/// The state lines of the fetch that appear in the Code tab.
pub const REMOTE_MANAGED_SETTINGS_STILL_LOADING: &str = "Remote managed settings are still loading";
/// The state line of a fetch that never finished.
pub const REMOTE_MANAGED_SETTINGS_DID_NOT_LOAD: &str = "Remote managed settings did not load";

/// The line when the settings loaded after one of the warnings; `applies` says whether a remote
/// policy applies.
#[must_use]
pub fn remote_managed_settings_loaded(applies: bool) -> String {
    format!(
        "Remote managed settings have now loaded: {}",
        if applies {
            "remote policy applied"
        } else {
            "no remote policy applied"
        }
    )
}

/// The error of a name that imitates an official marketplace, on every plugin it affects.
#[must_use]
pub fn marketplace_name_refused(name: &str) -> String {
    format!("Claude Code refuses the marketplace name \"{name}\"")
}

/// The marketplace's own error for an imitating name.
pub const MARKETPLACE_NAME_LOOKS_OFFICIAL: &str =
    "Claude Code refuses this marketplace's name: it looks like one of Anthropic's own";

/// What `claude plugin marketplace add` says for an impersonating name.
pub const MARKETPLACE_NAME_IMPERSONATES: &str =
    "Marketplace name impersonates an official Anthropic/Claude marketplace";

/// The refusal to add a marketplace whose name is another spelling of a reserved name.
#[must_use]
pub fn reserved_spelling_refusal(name: &str, reserved: &str, needs_quoting: bool) -> String {
    if needs_quoting {
        format!(
            "This marketplace's name is another spelling of \"{reserved}\", a reserved marketplace name. It is not exactly the reserved name it appears to be."
        )
    } else {
        format!(
            "Failed to add marketplace: \"{name}\" is another spelling of \"{reserved}\", a reserved marketplace name."
        )
    }
}

/// The warning for a registered entry whose name is another spelling of a reserved name.
#[must_use]
pub fn reserved_spelling_ignored(name: &str, reserved: &str) -> String {
    format!(
        "known_marketplaces.json has an entry named \"{name}\", another spelling of the reserved marketplace name \"{reserved}\", so it is ignored. Remove it with: claude plugin marketplace remove {name}"
    )
}

/// The stderr warning when denying the whole Bash tool leaves a Windows session with no shell
/// tool.
pub const BASH_DENY_TURNS_OFF_POWERSHELL: &str = "Denying Bash also turns off the PowerShell tool, so Claude has neither. To use PowerShell, set CLAUDE_CODE_USE_POWERSHELL_TOOL=1.";

/// The notification for an `asyncRewake` shell-form hook whose script path has a space and no
/// quotes.
#[must_use]
pub fn hook_needs_quotes(event: &str, script: &str) -> String {
    format!("{event} hook needs quotes around a path with a space ({script})")
}

/// A rule's behavior, `deny` or `ask`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DenyOrAsk {
    /// `deny`
    Deny,
    /// `ask`
    Ask,
}

/// A rule's behavior, `allow`, `deny` or `ask`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Behavior {
    /// `allow`
    Allow,
    /// `deny`
    Deny,
    /// `ask`
    Ask,
}

/// The startup banner row for a deny or ask rule of a file tool that file checks never consult.
#[must_use]
pub fn file_tool_rule_banner_row(behavior: DenyOrAsk, rule: &str, use_rule: &str) -> String {
    let label = match behavior {
        DenyOrAsk::Deny => "Deny",
        DenyOrAsk::Ask => "Ask",
    };
    format!("⚠ {label} rule {rule} never matches · use {use_rule}")
}

/// The input of [`file_tool_rule_warning`].
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FileToolRuleWarningInput {
    /// `allow`, `deny` or `ask`.
    pub behavior: Behavior,
    /// A settings file path or the flag.
    pub source: String,
    /// The rule that never matches.
    pub rule: String,
    /// `Edit` or `Read`, the rule kind to write instead.
    #[serde(rename = "use")]
    pub use_rule: String,
    /// `Write(docs/**)` becomes `Edit(docs/**)`.
    pub replacement: String,
}

/// The full warning for such a rule: `source` is a settings file path or the flag.
#[must_use]
pub fn file_tool_rule_warning(input: &FileToolRuleWarningInput) -> String {
    let behavior = match input.behavior {
        Behavior::Allow => "allow",
        Behavior::Deny => "deny",
        Behavior::Ask => "ask",
    };
    format!(
        "Permission {behavior} rule ({}): {} is not matched by file permission checks — only {}(path) rules are. Use {} instead (Edit rules cover all file-editing tools).",
        input.source, input.rule, input.use_rule, input.replacement
    )
}

/// The startup banner row for a deny or ask rule whose tool name is unknown.
#[must_use]
pub fn matches_no_tool_banner_row(behavior: DenyOrAsk, rule: &str) -> String {
    let label = match behavior {
        DenyOrAsk::Deny => "Deny",
        DenyOrAsk::Ask => "Ask",
    };
    format!("⚠ {label} rule {rule} matches no tool · check for typos")
}

/// The error when a `claude plugin` command cannot ask for approval of managed settings.
pub const MANAGED_SETTINGS_NEED_APPROVAL: &str = "Your organization's managed settings need your approval, which couldn't be asked for here. Run `claude` in a terminal and approve them, then run this command again.";

/// What Claude Code prints when the security approval dialog is declined.
pub const MANAGED_SETTINGS_NOT_APPROVED: &str =
    "Managed settings were not approved; exiting without applying them.";

/// The result an MCP tool call gets when the server is turned off.
#[must_use]
pub fn mcp_server_disabled(name: &str) -> String {
    format!("MCP server \"{name}\" is disabled — re-enable it via /mcp to use its tools")
}

/// The warning for an MCP server whose name Claude Code reserves.
#[must_use]
pub fn mcp_server_name_reserved(name: &str) -> String {
    format!("\"{name}\" is a reserved MCP server name and was not loaded")
}

/// The message for a Java MCP SDK server that rejects the elicitation capability.
pub const ELICITATION_REJECTED: &str = "This server (Java MCP SDK 0.17.0 or older) rejected Claude Code's elicitation capability. Set \"bareElicitationCapability\": true for it, or upgrade to Java MCP SDK 0.17.1+ (Spring AI 1.0.7+ or 1.1.6+).";

/// The error result for a call to a tool that is not in the session's tool list; `hint` follows
/// the name.
#[must_use]
pub fn no_such_tool(name: &str, hint: Option<&str>) -> String {
    let tail = match hint {
        None => String::new(),
        Some(hint) => format!(".{}{hint}", if hint.starts_with(' ') { "" } else { " " }),
    };
    format!("Error: No such tool available: {name}{tail}")
}

/// Codes reported under another name: `ESTALE` is reported and handled as `EIO` (spec J17.29).
const REPORTED_AS: [(&str, &str); 1] = [("ESTALE", "EIO")];

/// The code a path check reports: its own name, except those `REPORTED_AS` folds.
#[must_use]
pub fn reported_check_code(code: &str) -> String {
    REPORTED_AS
        .iter()
        .find(|(from, _)| *from == code)
        .map_or(code, |(_, to)| *to)
        .to_owned()
}

/// `/plugin` Errors tab: a path the operating system could not check (any error but "not found").
#[must_use]
pub fn path_could_not_be_checked(component: &str, path: &str, code: &str) -> String {
    format!(
        "{component} path could not be checked: {path} ({})",
        reported_check_code(code)
    )
}

/// The same error as `claude plugin list` prints it.
#[must_use]
pub fn path_could_not_be_checked_in_list(component: &str, path: &str, code: &str) -> String {
    format!(
        "Path not found: {path} ({component}, {})",
        reported_check_code(code)
    )
}

/// `<component> path escapes plugin directory: <path>` as `claude plugin` commands print it:
/// `Path escapes plugin directory: ./../shared.md (commands)` (errors.md, "Path escapes plugin
/// directory").
#[must_use]
pub fn path_escapes_in_list(component: &str, path: &str) -> String {
    format!("Path escapes plugin directory: {path} ({component})")
}

/// Where a marketplace plugin's manifest resolves when it is not inside the plugin or its
/// marketplace.
#[must_use]
pub fn manifest_not_read(manifest: &str, reason: &str) -> String {
    format!("Plugin not loaded: its manifest {manifest} was not read, because {reason}")
}

/// Which managed lock an ignored allow list belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxLock {
    /// `read`
    Read,
    /// `domains`
    Domains,
}

/// The input of [`sandbox_allow_ignored`].
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SandboxAllowIgnoredInput {
    /// The lock that ignores the entries.
    pub lock: SandboxLock,
    /// How many entries the file holds.
    pub count: f64,
    /// The file whose entries are ignored.
    pub file: String,
}

/// The `/status` warning for `allowRead` or `allowedDomains` entries a managed lock ignores.
#[must_use]
pub fn sandbox_allow_ignored(input: &SandboxAllowIgnoredInput) -> String {
    let read = input.lock == SandboxLock::Read;
    let key = if read {
        "sandbox.filesystem.allowRead"
    } else {
        "sandbox.network.allowedDomains"
    };
    let lock = if read {
        "allowManagedReadPathsOnly"
    } else {
        "allowManagedDomainsOnly"
    };
    let honored = if read {
        "only allowRead paths from managed settings are honored"
    } else {
        "only allowed domains (and WebFetch domain rules) from managed settings are honored"
    };
    // "The <N> entries in <file>" (errors.md): the docs give one form, whatever the count.
    format!(
        "{key} is ignored — managed settings set {lock}, so {honored}. The {} entries in {} will NOT apply (see https://code.claude.com/docs/en/settings#sandbox-settings).",
        number_to_string(input.count),
        input.file
    )
}

/// The startup notice for skill or command names Claude Code refused; `names` is the count.
#[must_use]
pub fn skill_names_refused(count: f64) -> String {
    format!(
        "{} skill or command name{} refused: run with --debug to see which ([names]), rename, then restart — a name may not hold hidden, direction-changing or blank-like characters, < or >, or be over 256 characters",
        number_to_string(count),
        // JavaScript's `count === 1`.
        if count == 1.0 { "" } else { "s" }
    )
}

crate::registry::exports! {
    "format/host-errors" {
        "reservedSkillNameNotice" => ok |input: ReservedSkillNameInput| reserved_skill_name_notice(&input);
        "agentDescriptionsOverLimit" => ok |estimated_tokens: f64, folder: String| agent_descriptions_over_limit(estimated_tokens, &folder);
        "mcpJsonUnreadable" => ok mcp_json_unreadable;
        "cannotAddMcpToManagedScope" => ok cannot_add_mcp_to_managed_scope;
        "organizationSettingsUnavailable" => ok |reason: String, organization: String| organization_settings_unavailable(&reason, &organization);
        "remoteManagedSettingsFailed" => ok |cause: String, policy: RemotePolicy| remote_managed_settings_failed(&cause, policy);
        "REMOTE_MANAGED_SETTINGS_STILL_LOADING" => ok || REMOTE_MANAGED_SETTINGS_STILL_LOADING;
        "REMOTE_MANAGED_SETTINGS_DID_NOT_LOAD" => ok || REMOTE_MANAGED_SETTINGS_DID_NOT_LOAD;
        "remoteManagedSettingsLoaded" => ok |applies: bool| remote_managed_settings_loaded(applies);
        "marketplaceNameRefused" => ok |name: String| marketplace_name_refused(&name);
        "MARKETPLACE_NAME_LOOKS_OFFICIAL" => ok || MARKETPLACE_NAME_LOOKS_OFFICIAL;
        "MARKETPLACE_NAME_IMPERSONATES" => ok || MARKETPLACE_NAME_IMPERSONATES;
        "reservedSpellingRefusal" => ok |name: String, reserved: String, needs_quoting: bool| reserved_spelling_refusal(&name, &reserved, needs_quoting);
        "reservedSpellingIgnored" => ok |name: String, reserved: String| reserved_spelling_ignored(&name, &reserved);
        "BASH_DENY_TURNS_OFF_POWERSHELL" => ok || BASH_DENY_TURNS_OFF_POWERSHELL;
        "hookNeedsQuotes" => ok |event: String, script: String| hook_needs_quotes(&event, &script);
        "fileToolRuleBannerRow" => ok |behavior: DenyOrAsk, rule: String, use_rule: String| file_tool_rule_banner_row(behavior, &rule, &use_rule);
        "fileToolRuleWarning" => ok |input: FileToolRuleWarningInput| file_tool_rule_warning(&input);
        "matchesNoToolBannerRow" => ok |behavior: DenyOrAsk, rule: String| matches_no_tool_banner_row(behavior, &rule);
        "MANAGED_SETTINGS_NEED_APPROVAL" => ok || MANAGED_SETTINGS_NEED_APPROVAL;
        "MANAGED_SETTINGS_NOT_APPROVED" => ok || MANAGED_SETTINGS_NOT_APPROVED;
        "mcpServerDisabled" => ok |name: String| mcp_server_disabled(&name);
        "mcpServerNameReserved" => ok |name: String| mcp_server_name_reserved(&name);
        "ELICITATION_REJECTED" => ok || ELICITATION_REJECTED;
        "noSuchTool" => ok |name: String, hint: Option<String>| no_such_tool(&name, hint.as_deref());
        "reportedCheckCode" => ok |code: String| reported_check_code(&code);
        "pathCouldNotBeChecked" => ok |component: String, path: String, code: String| path_could_not_be_checked(&component, &path, &code);
        "pathCouldNotBeCheckedInList" => ok |component: String, path: String, code: String| path_could_not_be_checked_in_list(&component, &path, &code);
        "pathEscapesInList" => ok |component: String, path: String| path_escapes_in_list(&component, &path);
        "manifestNotRead" => ok |manifest: String, reason: String| manifest_not_read(&manifest, &reason);
        "sandboxAllowIgnored" => ok |input: SandboxAllowIgnoredInput| sandbox_allow_ignored(&input);
        "skillNamesRefused" => ok |count: f64| skill_names_refused(count);
    }
}
