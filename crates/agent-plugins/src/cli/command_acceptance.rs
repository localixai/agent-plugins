//! Port of `src/cli/command-acceptance.ts`, switched.
//!
//! Accepting a marketplace-declared command before `claude plugin install` or `update` runs it
//! (plugins/cli-reference.md, "plugin install": `-y, --yes`, `--accept-command <sha256>`, the lines
//! `Aborted.` and `Aborted — the command was not run.`; "Accept a displayed install command";
//! plugins/host-marketplace.md, "How users accept a headersHelper command"). "Claude Code first prints
//! the command and asks `Run this command now? [y/N]`"; `-y` accepts it "Ignored when the command runs
//! inside a Claude Code session"; `--accept-command` "Can't be combined with `-y`" and "has no effect
//! inside a Claude Code session"; "The `sha256` counts as acceptance for exactly that command, plugin,
//! and marketplace catalog. If any of them changed since the command was displayed, Claude Code
//! doesn't accept the `sha256` and shows the command again. A change that the run's own marketplace
//! refresh fetches also counts as such a change." Judgments (spec/judgments.md): what the digest
//! covers (`digestOf`), that a `--accept-command` that does not match never prompts, and the line a
//! command source gets when it cannot be confirmed without a terminal.

use serde::{Deserialize, Serialize};

use super::result::{CommandOutput, usage_error};
use crate::js::{JsValue, json_stringify};
use crate::ports::marketplace::Sha256;
use crate::registry::Port;

/// What the entry asks Claude Code to run. A `headersHelper` wins when an entry has both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandOrigin {
    /// `command-source`.
    CommandSource,
    /// `headers-helper`.
    HeadersHelper,
}

/// A command the host displays.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayedCommand {
    /// The command.
    pub command: String,
    /// The plugin id the command belongs to.
    pub plugin: String,
    /// Where the command comes from.
    pub origin: CommandOrigin,
}

/// `-y` and `--accept-command`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceFlags {
    /// `-y`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yes: Option<bool>,
    /// `--accept-command`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_command: Option<String>,
}

/// "Can't be combined with `-y`": a flag problem, nothing is printed but the reason.
#[must_use]
pub fn exclusive_accept_flags(flags: &AcceptanceFlags) -> Option<CommandOutput> {
    if flags.yes == Some(true) && flags.accept_command.is_some() {
        Some(usage_error("--accept-command cannot be combined with -y"))
    } else {
        None
    }
}

/// The object a `--json` run's `failed` result carries when it displays a command and does not run it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownCommand {
    /// The command.
    pub command: String,
    /// The plugin.
    pub plugin: String,
    /// The digest of the command now displayed.
    pub sha256: String,
    /// Present when `--accept-command` was passed: whether its value is the digest of the command now displayed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_command_matched: Option<bool>,
}

/// What the run asks of the acceptance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceRequest {
    /// The command displayed.
    pub shown: DisplayedCommand,
    /// The marketplace catalog the run read, after its own refresh: a change the refresh fetched changes the digest.
    pub catalog: String,
    /// The flags.
    pub flags: AcceptanceFlags,
    /// The command runs inside a Claude Code session (the Bash tool, a hook).
    pub session: bool,
    /// Both stdin and stdout are a terminal.
    pub tty: bool,
    /// The answer to `Run this command now? [y/N]`, once the host has asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<bool>,
}

/// The question the host asks.
pub const PROMPT: &str = "Run this command now? [y/N]";
const ABORTED: &str = "Aborted.";
const ABORTED_NOT_RUN: &str = "Aborted — the command was not run.";
/// What a command source gets without a terminal and without `-y` or `--accept-command`: "the command was only displayed".
const ONLY_DISPLAYED: &str = "The install command was only displayed, not run. Pass -y, or --accept-command with the sha256 of the displayed command, to run it.";

/// Whether the displayed command runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum CommandAcceptance {
    /// `{ run: true }`.
    Run {
        /// `true`.
        run: bool,
    },
    /// The host prints the command, asks `prompt` and calls again with the answer.
    Ask {
        /// `false`.
        run: bool,
        /// `true`.
        ask: bool,
        /// The question.
        prompt: String,
        /// The command displayed.
        #[serde(rename = "shownCommand")]
        shown_command: ShownCommand,
    },
    /// A line to print, and nothing runs.
    Line {
        /// `false`.
        run: bool,
        /// `false`.
        ask: bool,
        /// The line.
        line: String,
        /// The command displayed.
        #[serde(rename = "shownCommand")]
        shown_command: ShownCommand,
    },
}

/// The digest `--accept-command` names: SHA-256 of the command, the plugin and the catalog it came from (judgment: the docs do not say how it is built).
pub async fn digest_of(shown: &DisplayedCommand, catalog: &str, sha256: &dyn Sha256) -> String {
    let text = json_stringify(&JsValue::from([
        ("command", shown.command.as_str().into()),
        ("plugin", shown.plugin.as_str().into()),
        ("catalog", catalog.into()),
    ]))
    .unwrap_or_default();
    sha256.hex(text.as_bytes()).await
}

fn declined(origin: CommandOrigin) -> &'static str {
    match origin {
        CommandOrigin::CommandSource => ABORTED,
        CommandOrigin::HeadersHelper => ABORTED_NOT_RUN,
    }
}

fn unconfirmed(origin: CommandOrigin) -> &'static str {
    match origin {
        CommandOrigin::CommandSource => ONLY_DISPLAYED,
        CommandOrigin::HeadersHelper => ABORTED_NOT_RUN,
    }
}

fn refused(line: &str, shown_command: ShownCommand) -> CommandAcceptance {
    CommandAcceptance::Line {
        run: false,
        ask: false,
        line: line.to_owned(),
        shown_command,
    }
}

/// Without `-y` or a matching digest: ask at a terminal, refuse without one.
fn confirm(request: &AcceptanceRequest, shown_command: ShownCommand) -> CommandAcceptance {
    let origin = request.shown.origin;
    if !request.tty {
        return refused(unconfirmed(origin), shown_command);
    }
    match request.answer {
        None => CommandAcceptance::Ask {
            run: false,
            ask: true,
            prompt: PROMPT.to_owned(),
            shown_command,
        },
        Some(true) => CommandAcceptance::Run { run: true },
        Some(false) => refused(declined(origin), shown_command),
    }
}

/// Whether the displayed command runs. Inside a session neither `-y` nor `--accept-command` counts.
/// A digest that does not match the command now displayed is shown again with
/// `acceptCommandMatched: false` and never prompts.
pub async fn decide_acceptance(
    request: &AcceptanceRequest,
    sha256: &dyn Sha256,
) -> CommandAcceptance {
    let digest = digest_of(&request.shown, &request.catalog, sha256).await;
    let base = ShownCommand {
        command: request.shown.command.clone(),
        plugin: request.shown.plugin.clone(),
        sha256: digest.clone(),
        accept_command_matched: None,
    };
    let given = if request.session {
        None
    } else {
        request.flags.accept_command.as_deref()
    };
    if let Some(given) = given {
        if given == digest {
            return CommandAcceptance::Run { run: true };
        }
        return refused(
            declined(request.shown.origin),
            ShownCommand {
                accept_command_matched: Some(false),
                ..base
            },
        );
    }
    if !request.session && request.flags.yes == Some(true) {
        return CommandAcceptance::Run { run: true };
    }
    confirm(request, base)
}

crate::registry::exports! {
    "cli/command-acceptance" {
        "exclusiveAcceptFlags" => ok |flags: AcceptanceFlags| exclusive_accept_flags(&flags);
        "PROMPT" => ok || PROMPT;
        "digestOf" => async ok |shown: DisplayedCommand, catalog: String, sha256: Port| digest_of(&shown, &catalog, &sha256).await;
        "decideAcceptance" => async ok |request: AcceptanceRequest, sha256: Port| decide_acceptance(&request, &sha256).await;
    }
}
