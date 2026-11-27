//! Port of `src/cli/install-start.ts`, switched.
//!
//! The first half of `claude plugin install <plugin>` (plugins/cli-reference.md, "plugin install"):
//! what the arguments mean and what the host does before it looks the plugin up: add the
//! `--marketplace` source first, refresh the marketplace the install names (plugins/loading.md, "When
//! Claude Code refreshes a marketplace before an install", through `refreshBeforeInstall`). The rest
//! is `planInstall`. A marketplace added by `--marketplace` is "declared in your user settings even
//! when you pass `--scope project`" (plugins/install.md, "Add and install from your shell"); it was
//! just fetched, so no refresh follows it (judgment, spec/judgments.md).

use serde::{Deserialize, Serialize};

use super::command_acceptance::{AcceptanceFlags, exclusive_accept_flags};
use super::install_config::{ParsedConfigFlags, parse_config_flags};
use super::result::{CommandOutput, Extras, command_failed, usage_error};
use super::scope::{ChosenOrError, ScopeChoice, ScopedCommand, choose_scope};
use crate::format::marketplace::ids::parse_plugin_ref;
use crate::format::marketplace::types::MarketplaceSource;
use crate::format::path_text::PathPlatform;
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::marketplace::install_scopes::InstallScope;
use crate::marketplace::marketplace_input::{
    MarketplaceInputOptions, Parsed as InputParsed, parse_marketplace_input,
};
use crate::registry::Port;
use crate::updates::refresh_before_install::{
    InstallCommand, KnownMarketplace, RefreshDecision, RefreshRequest, SkipReason,
    refresh_before_install,
};

/// What `install` was given.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallArguments {
    /// `name`, or `name@marketplace`; the bare name alone with `--marketplace`.
    pub plugin: String,
    /// `--scope`.
    #[serde(default)]
    pub scope: Option<String>,
    /// Each `--config` flag as written, `key=value`.
    #[serde(default)]
    pub config: Option<Vec<String>>,
    /// `--json`.
    #[serde(default)]
    pub json: Option<bool>,
    /// `-y`.
    #[serde(default)]
    pub yes: Option<bool>,
    /// `--accept-command`.
    #[serde(default)]
    pub accept_command: Option<String>,
    /// `--marketplace <source>`.
    #[serde(default)]
    pub marketplace: Option<String>,
    /// The host's platform.
    #[serde(default)]
    pub platform: Option<PathPlatform>,
}

/// What the install reads of the host: its marketplaces, its environment and clock.
#[derive(Clone, Debug, Deserialize)]
pub struct InstallContext {
    /// The marketplaces the user registered.
    pub marketplaces: Vec<KnownMarketplace>,
    /// The environment.
    pub env: Port,
    /// The clock.
    pub clock: Port,
    /// The command runs inside a Claude Code session.
    pub session: bool,
    /// Both stdin and stdout are a terminal.
    pub tty: bool,
}

/// The scope a marketplace `--marketplace` adds is declared at: always the user settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddScope {
    /// `user`.
    User,
}

/// Add this marketplace first, to the user settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddMarketplace {
    /// Its source.
    pub source: MarketplaceSource,
    /// Always `user`.
    pub scope: AddScope,
}

/// What the host does before the lookup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's record of the install: independent yes-or-no facts, as the interface has them"
)]
#[serde(rename_all = "camelCase")]
pub struct PendingInstall {
    /// The plugin as given.
    pub plugin: String,
    /// The scope it is installed at.
    pub scope: InstallScope,
    /// `--json`.
    pub json: bool,
    /// The `--config` values by key.
    pub given: crate::js::JsObject,
    /// `-y`.
    pub yes: bool,
    /// `--accept-command`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_command: Option<String>,
    /// The command runs inside a Claude Code session.
    pub session: bool,
    /// Both stdin and stdout are a terminal.
    pub tty: bool,
    /// Add this marketplace first, to the user settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add_marketplace: Option<AddMarketplace>,
    /// The `RefreshDecision`, as the host carries it out; the plan does not read it.
    pub refresh: JsValue,
}

/// The arguments' meaning: the output that ends the command, or what to do before the lookup.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum InstallStart {
    /// The command ends.
    Done {
        /// What it prints.
        output: CommandOutput,
    },
    /// The host prepares the lookup.
    Prepare {
        /// What it does first.
        pending: PendingInstall,
    },
}

const fn done(output: CommandOutput) -> InstallStart {
    InstallStart::Done { output }
}

/// The source `--marketplace` names, or what ends the command.
fn marketplace_source(args: &InstallArguments) -> Result<MarketplaceSource, Box<CommandOutput>> {
    let Some(marketplace) = &args.marketplace else {
        return Err(Box::new(usage_error("no --marketplace")));
    };
    if parse_plugin_ref(&args.plugin).is_some_and(|reference| reference.marketplace.is_some()) {
        return Err(Box::new(usage_error(
            "--marketplace needs the plugin name by itself, without @marketplace",
        )));
    }
    let options = MarketplaceInputOptions {
        platform: args.platform,
        sparse: None,
    };
    match parse_marketplace_input(marketplace, &options) {
        InputParsed::Source(source) => Ok(source),
        InputParsed::Issue(issue) => Err(Box::new(command_failed(
            "install",
            &issue.message,
            Extras {
                failure_code: Some("invalid_source".to_owned()),
                fields: None,
            },
        ))),
    }
}

/// The refresh decision of the install: none for a marketplace that was just added.
async fn refresh_of(args: &InstallArguments, context: &InstallContext) -> RefreshDecision {
    if args.marketplace.is_some() {
        return RefreshDecision::Skip {
            outcome: "skip",
            reason: SkipReason::MarketplaceUnknown,
        };
    }
    refresh_before_install(&RefreshRequest {
        reference: args.plugin.clone(),
        command: InstallCommand::ClaudePluginInstall,
        marketplaces: context.marketplaces.clone(),
        lookup_missed: false,
        env: context.env,
        clock: context.clock,
    })
    .await
}

/// The flags the install reads.
struct ParsedArguments {
    scope: InstallScope,
    given: crate::js::JsObject,
    added: Option<AddMarketplace>,
}

/// The flags the install reads, or the output that ends the command before anything else happens.
fn parse_arguments(args: &InstallArguments) -> Result<ParsedArguments, Box<CommandOutput>> {
    let scope = match choose_scope(ScopedCommand::Install, args.scope.as_deref()) {
        ChosenOrError::Error(output) => return Err(Box::new(output)),
        ChosenOrError::Choice(ScopeChoice::Scope { scope }) => scope,
        // `install` writes to one scope, `user` when `--scope` is omitted (`DefaultedCommand`).
        ChosenOrError::Choice(ScopeChoice::AutoDetect { .. } | ScopeChoice::AllScopes { .. }) => {
            InstallScope::User
        }
    };
    let exclusive = exclusive_accept_flags(&AcceptanceFlags {
        yes: args.yes,
        accept_command: args.accept_command.clone(),
    });
    if let Some(exclusive) = exclusive {
        return Err(Box::new(exclusive));
    }
    let given = match parse_config_flags(args.config.as_deref().unwrap_or_default()) {
        ParsedConfigFlags::Given { given } => given,
        ParsedConfigFlags::Usage(output) => return Err(Box::new(output)),
    };
    if args.marketplace.is_none() {
        return Ok(ParsedArguments {
            scope,
            given,
            added: None,
        });
    }
    let source = marketplace_source(args)?;
    Ok(ParsedArguments {
        scope,
        given,
        added: Some(AddMarketplace {
            source,
            scope: AddScope::User,
        }),
    })
}

fn pending_of(
    args: &InstallArguments,
    context: &InstallContext,
    parsed: ParsedArguments,
    refresh: &RefreshDecision,
) -> PendingInstall {
    PendingInstall {
        plugin: args.plugin.clone(),
        scope: parsed.scope,
        json: args.json == Some(true),
        given: parsed.given,
        yes: args.yes == Some(true),
        accept_command: args.accept_command.clone(),
        session: context.session,
        tty: context.tty,
        add_marketplace: parsed.added,
        refresh: to_js_value(refresh).unwrap_or_default(),
    }
}

/// Reads the arguments and says what to do before the lookup, or what the command ends with.
pub async fn start_install(args: &InstallArguments, context: &InstallContext) -> InstallStart {
    let parsed = match parse_arguments(args) {
        Ok(parsed) => parsed,
        Err(end) => return done(*end),
    };
    let refresh = refresh_of(args, context).await;
    if let RefreshDecision::Refuse { issue, .. } = &refresh {
        return done(command_failed(
            "install",
            &issue.message,
            Extras {
                failure_code: Some("marketplace_blocked".to_owned()),
                fields: None,
            },
        ));
    }
    InstallStart::Prepare {
        pending: pending_of(args, context, parsed, &refresh),
    }
}

crate::registry::exports! {
    "cli/install-start" {
        "startInstall" => async ok |args: InstallArguments, context: InstallContext|
            start_install(&args, &context).await;
    }
}
