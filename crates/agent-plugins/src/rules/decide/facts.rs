//! Ported from `src/rules/decide/facts.ts`; only the core uses it now, so it has no wrapper.

use super::block_exceptions::{block_paths, read_blocked, write_block_reason};
use super::context::DecideContext;
use super::critical_removal::{cannot_rule_out_removal, removes_critical_path};
use super::file_targets::command_targets;
use super::powershell_edits::powershell_verdict;
use super::powershell_removal::powershell_removal_denied;
use super::protected_paths::{is_protected_path, is_symlink_target_only};
use super::redirects::{NO_REDIRECTS, RedirectFacts, target_facts};
use super::scope::in_scope;
use super::shell_commands::{ShellVerdict, shell_verdict, unread};
use super::write_block_messages::WriteBlockReason;
use crate::js::JsValue;
use crate::registry::Callback;
use crate::rules::bash::shell::command_of;
use crate::rules::path::network::reads_network_path;
use crate::rules::path::tools::requested_path;
use crate::rules::target::input::input_string;
use crate::rules::types::{DecisionReason, PathContext, PreparedCall};

/// What a call is, read once from its tool, input and the session, so the modes ask questions
/// instead of re-reading the call. Every field is data about the call; none is a decision.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's shape: one fact per flag"
)]
pub struct CallFacts {
    /// Runs without a prompt in every mode that prompts: reads in scope, read-only commands, tools
    /// needing no permission.
    pub read_only: bool,
    /// Why it is read-only.
    pub read_reason: DecisionReason,
    /// A call of a file-reading tool (Read, Grep, Glob).
    pub reads_files: bool,
    /// A file-reading call whose path is a network path.
    pub network: bool,
    /// An edit tool whose path, and whose resolved paths, are in scope and not protected.
    pub edits_in_scope: bool,
    /// A Bash command made of read-only and filesystem commands in scope.
    pub shell_edits: bool,
    /// An edit that is in scope as asked and resolves outside it through a symlink.
    pub escapes_via_link: bool,
    /// A write to a protected path.
    pub protected_write: bool,
    /// Protected only where a symlink leads, not as asked.
    pub protected_via_link: bool,
    /// A Bash or PowerShell call.
    pub shell: bool,
    /// A recognized critical-path removal.
    pub critical_removal: bool,
    /// The tool requires user interaction.
    pub interaction: bool,
    /// A connector tool the organization set to `ask`.
    pub connector_ask: bool,
    /// Why a file tool call is refused by the `permissions.block*OutsideWorkingDirectories` keys,
    /// if it is.
    pub blocked_by: Option<DecisionReason>,
    /// A Bash command that prompts under `permissions.blockReadsOutsideWorkingDirectories`: it
    /// reads outside the working directories, or the parser cannot trace it.
    pub read_block_prompt: bool,
    /// The command, split, for a shell call.
    pub parts: ShellVerdict,
    /// What the redirect targets of a Bash command ask of the check.
    pub redirects: RedirectFacts,
    /// A tool no rule, hook or mode stops (`EndConversation`).
    pub exempt: bool,
    /// A Bash command that asks to run outside the sandbox (`dangerouslyDisableSandbox: true`).
    pub unsandboxed: bool,
    /// A `SubagentHandback` call, the delivery of a subagent's report.
    pub handback: bool,
    /// A PowerShell removal denied in every mode ("Remove-Item in PowerShell").
    pub removal_denied: bool,
    /// An `Artifact` read of another organization's public artifact.
    pub foreign_artifact: bool,
}

fn none() -> CallFacts {
    CallFacts {
        read_only: false,
        read_reason: DecisionReason::ReadInWorkingDirectory,
        reads_files: false,
        network: false,
        edits_in_scope: false,
        shell_edits: false,
        escapes_via_link: false,
        protected_write: false,
        protected_via_link: false,
        shell: false,
        critical_removal: false,
        interaction: false,
        connector_ask: false,
        blocked_by: None,
        read_block_prompt: false,
        parts: unread(),
        redirects: NO_REDIRECTS,
        exempt: false,
        unsandboxed: false,
        handback: false,
        removal_denied: false,
        foreign_artifact: false,
    }
}

/// Where a call acts: the path it asked for and where the host's symlink check says it leads.
fn touched(call: &PreparedCall, requested: String) -> Vec<String> {
    let mut all = vec![requested];
    all.extend(call.resolved_paths.iter().flatten().cloned());
    all
}

fn all_in_scope(paths: &[String], context: &PathContext) -> bool {
    paths.iter().all(|path| in_scope(path, context))
}

/// Read, Grep and Glob: "File reads, Grep ... No, within the working directory and additional
/// directories".
fn read_facts(call: &PreparedCall, ctx: &DecideContext, facts: &mut CallFacts) {
    facts.reads_files = true;
    let Some(requested) = requested_path(&call.call, ctx.paths()) else {
        return;
    };
    let all = touched(call, requested);
    let inside = all_in_scope(&all, ctx.paths());
    let network = reads_network_path(call, ctx.paths());
    facts.network = network;
    facts.read_only = inside && !network;
    facts.blocked_by = read_blocked(&all, ctx).then_some(DecisionReason::OutsideWorkingDirectory);
}

/// LSP: no prompt of its own, but the read block refuses its `filePath` outside (§13.12.2 item 1,
/// J7.210).
fn lsp_facts(call: &PreparedCall, ctx: &DecideContext, facts: &mut CallFacts) {
    let Some(requested) = requested_path(&call.call, ctx.paths()) else {
        return;
    };
    if read_blocked(&touched(call, requested), ctx) {
        facts.blocked_by = Some(DecisionReason::OutsideWorkingDirectory);
    }
}

/// Edit, Write, `NotebookEdit` and `MultiEdit`.
fn edit_facts(call: &PreparedCall, ctx: &DecideContext, facts: &mut CallFacts) {
    let paths = ctx.paths();
    let Some(requested) = requested_path(&call.call, paths) else {
        return;
    };
    let options = ctx.options();
    let asked = is_protected_path(&requested, paths, options);
    let requested_in_scope = in_scope(&requested, paths);
    let all = touched(call, requested);
    let via_link = !asked
        && all.iter().any(|path| {
            is_protected_path(path, paths, options) || is_symlink_target_only(path, paths, options)
        });
    let inside = all_in_scope(&all, paths);
    facts.edits_in_scope = inside && !asked && !via_link;
    facts.escapes_via_link = requested_in_scope && !inside;
    facts.protected_write = asked || via_link;
    facts.protected_via_link = via_link;
    facts.blocked_by = write_block_reason(&all, ctx).map(|reason| match reason {
        WriteBlockReason::OutsideWorkingDirectory => DecisionReason::OutsideWorkingDirectory,
        WriteBlockReason::SettingsFile => DecisionReason::SettingsFile,
    });
}

/// Whether a Bash command prompts under the read block: it reads outside, or cannot be traced.
fn prompts_under_read_block(
    command: &str,
    parts: &ShellVerdict,
    call: &PreparedCall,
    ctx: &DecideContext,
) -> bool {
    if ctx.block_reads_outside_working_directories != Some(true)
        || ctx.sandbox_enforces_read_block == Some(true)
    {
        return false;
    }
    // A directory only repository settings add is outside the working directories for the block.
    if ctx.repository_directories.is_none() {
        return parts.reads_outside || parts.untraceable;
    }
    let resolved = call.resolved_paths.as_deref().unwrap_or_default();
    let counted = shell_verdict(command, &block_paths(ctx), ctx.options(), resolved);
    counted.reads_outside || counted.untraceable
}

/// "Commands that ask to run outside the sandbox": the unsandboxed retry's
/// `dangerouslyDisableSandbox` parameter.
fn asks_outside_sandbox(input: &JsValue) -> bool {
    input.get("dangerouslyDisableSandbox") == Some(&JsValue::Bool(true))
}

/// Redirect targets that need nothing: none denied, asked, protected, or written in the working
/// directories.
const fn keeps_read_only(redirects: RedirectFacts) -> bool {
    !redirects.denied
        && !redirects.approval
        && !redirects.protected_write
        && !redirects.in_scope_write
}

fn bash_facts(
    call: &PreparedCall,
    ctx: &DecideContext,
    facts: &mut CallFacts,
) -> Result<(), String> {
    facts.shell = true;
    let Some(command) = command_of(&call.call.input) else {
        return Ok(());
    };
    let resolved = call.resolved_paths.as_deref().unwrap_or_default();
    let parts = shell_verdict(command, ctx.paths(), ctx.options(), resolved);
    // The files the command names go through the `Read` and `Edit` rules as the redirect targets do.
    let redirects = target_facts(&command_targets(command), ctx, resolved)?;
    facts.redirects = redirects;
    facts.unsandboxed = asks_outside_sandbox(&call.call.input);
    facts.read_only = parts.read_only && keeps_read_only(redirects);
    facts.read_reason = DecisionReason::ReadOnlyCommand;
    // A read-only command whose redirect writes in the working directories is an edit there.
    facts.shell_edits = parts.edits_in_scope || (parts.read_only && redirects.in_scope_write);
    facts.protected_write = parts.writes_protected || redirects.protected_write;
    facts.protected_via_link = parts.writes_protected_via_link && !redirects.protected_write;
    facts.read_block_prompt = prompts_under_read_block(command, &parts, call, ctx);
    facts.critical_removal =
        removes_critical_path(command, ctx.paths(), 0, false) || cannot_rule_out_removal(command);
    facts.parts = parts;
    Ok(())
}

/// `PowerShell`: split into subcommands for the rules, and the `acceptEdits` cmdlets read; no
/// read-only set.
fn powershell_facts(call: &PreparedCall, ctx: &DecideContext, facts: &mut CallFacts) {
    facts.shell = true;
    let Some(command) = command_of(&call.call.input) else {
        return;
    };
    let resolved = call.resolved_paths.as_deref().unwrap_or_default();
    let parts = powershell_verdict(command, ctx.paths(), ctx.options(), resolved);
    facts.shell_edits = parts.edits_in_scope;
    facts.protected_write = parts.writes_protected;
    facts.protected_via_link = parts.writes_protected_via_link;
    facts.removal_denied = powershell_removal_denied(
        command,
        ctx.paths(),
        ctx.powershell_cmd_removal_deny_off == Some(true),
    );
    facts.parts = parts;
}

/// The host's predicate answered `true` for `text`.
async fn says_yes(check: Option<Callback>, text: &str) -> bool {
    match check {
        Some(check) => {
            check.call::<JsValue>(vec![JsValue::from(text)]).await == JsValue::Bool(true)
        }
        None => false,
    }
}

/// An `Artifact` read the host says is another organization's public artifact.
async fn artifact_facts(call: &PreparedCall, ctx: &DecideContext, facts: &mut CallFacts) {
    let url = input_string(&call.call.input, "url");
    let read = input_string(&call.call.input, "action") == Some("read");
    facts.foreign_artifact = match url.filter(|_| read) {
        Some(url) => says_yes(ctx.foreign_artifact, url).await,
        None => false,
    };
}

/// `WebFetch`: a URL whose domain is in the host's preapproved set fetches without a prompt.
async fn web_fetch_facts(call: &PreparedCall, ctx: &DecideContext, facts: &mut CallFacts) {
    let hostname = call
        .call
        .input
        .get("url")
        .and_then(JsValue::as_str)
        .and_then(hostname_of);
    facts.read_only = match hostname {
        Some(hostname) => says_yes(ctx.preapproved_fetch_domain, &hostname).await,
        None => false,
    };
    facts.read_reason = DecisionReason::PreapprovedDomain;
}

/// `new URL(text).hostname.toLowerCase()`, or `None` where `new URL` throws.
fn hostname_of(text: &str) -> Option<String> {
    crate::js::url::parse(text).map(|url| url.hostname.to_lowercase())
}

/// `BUILDERS.get(call.toolName)?.(call, ctx)`, each tool's facts over `NONE`.
async fn built(call: &PreparedCall, ctx: &DecideContext) -> Result<CallFacts, String> {
    let mut facts = none();
    match call.call.tool_name.as_str() {
        "Read" | "Grep" | "Glob" => read_facts(call, ctx, &mut facts),
        "LSP" => lsp_facts(call, ctx, &mut facts),
        "Edit" | "Write" | "NotebookEdit" | "MultiEdit" => edit_facts(call, ctx, &mut facts),
        "Bash" => bash_facts(call, ctx, &mut facts)?,
        "PowerShell" => powershell_facts(call, ctx, &mut facts),
        "WebFetch" => web_fetch_facts(call, ctx, &mut facts).await,
        "Artifact" => artifact_facts(call, ctx, &mut facts).await,
        _ => {}
    }
    Ok(facts)
}

/// "The built-in `AskUserQuestion` tool and MCP tools marked `requiresUserInteraction`."
const INTERACTION_TOOLS: [&str; 1] = ["AskUserQuestion"];

/// tools-reference.md, "`EndConversation` tool behavior": "The tool never prompts for permission,
/// and `PreToolUse` hooks don't run for it. While any other tool remains, you can't block it
/// either: deny and ask rules naming `EndConversation` have no effect". Whether another tool
/// remains is the tool list's business (`availableTools`), so `decide` always lets it run
/// (spec/judgments.md).
const EXEMPT_TOOLS: [&str; 1] = ["EndConversation"];

const HANDBACK_TOOL: &str = "SubagentHandback";

/// What `call` is, in the session `ctx` describes.
///
/// # Errors
///
/// What a rule's matcher throws.
pub async fn facts_of(call: &PreparedCall, ctx: &DecideContext) -> Result<CallFacts, String> {
    let mut facts = built(call, ctx).await?;
    let tool = call.call.tool_name.as_str();
    // tools-reference.md: file-access tools marked No "still prompt for paths outside the working
    // directory", so the column never overrides what the path facts of a file read decided.
    let no_permission = match ctx.requires_permission {
        Some(check) => {
            check.call::<JsValue>(vec![JsValue::from(tool)]).await == JsValue::Bool(false)
                && !facts.reads_files
        }
        None => false,
    };
    if no_permission && !facts.read_only {
        facts.read_reason = DecisionReason::NoPermissionRequired;
    }
    facts.read_only = facts.read_only || no_permission;
    facts.exempt = EXEMPT_TOOLS.contains(&tool);
    facts.handback = tool == HANDBACK_TOOL;
    let has = |set: &Option<crate::registry::Set<String>>| {
        set.as_ref()
            .is_some_and(|set| set.members.iter().any(|name| name == tool))
    };
    facts.interaction = INTERACTION_TOOLS.contains(&tool) || has(&ctx.requires_user_interaction);
    facts.connector_ask = has(&ctx.connector_tools_set_to_ask);
    Ok(facts)
}
