//! Ported from `src/cli/install-lookup.ts`; only the core uses it now, so it has no wrapper.
//!
//! Looking the plugin up for `claude plugin install` (plugins/cli-reference.md, "plugin install";
//! plugins/install.md, "Add and install from your shell"; plugins/troubleshooting.md, `Plugin "<name>"
//! not found in any configured marketplace`): which catalog entry the argument names, what prints
//! ahead of the result and which command the entry asks Claude Code to run.

use serde::Deserialize;

use super::command_acceptance::{CommandOrigin, DisplayedCommand};
use super::install_plan::{AddedMarketplace, InstallFound};
use super::install_start::PendingInstall;
use super::marketplace_add::name_refusal;
use super::plugin_ref::{matching_ids, qualified_commands};
use super::result::{CommandOutput, Extras, command_failed};
use crate::format::marketplace::types::MarketplaceSource;
use crate::js::{JsValue, key_order};
use crate::marketplace::dependency_graph::CatalogPlugin;
use crate::marketplace::install_errors::already_added_from_different_source;
use crate::marketplace::known_marketplaces::same_marketplace_source;
use crate::updates::refresh_before_install::NOT_REFRESHED;

/// The marketplace the plugin is installed from is named after the `@`.
fn ids_for(found: &InstallFound) -> Vec<String> {
    let all = key_order(found.graph.catalog.keys());
    let added = found
        .added_marketplace
        .as_ref()
        .map(|added| added.name.as_str());
    all.into_iter()
        .filter(|id| {
            added.is_none_or(|added| {
                found
                    .graph
                    .catalog
                    .get(id.as_str())
                    .is_some_and(|entry| entry.marketplace == added)
            })
        })
        .cloned()
        .collect()
}

fn failed(message: &str, code: &str) -> CommandOutput {
    command_failed(
        "install",
        message,
        Extras {
            failure_code: Some(code.to_owned()),
            fields: None,
        },
    )
}

/// The catalog entry the argument names, or what ends the command.
pub fn locate<'f>(
    pending: &PendingInstall,
    found: &'f InstallFound,
) -> Result<(String, &'f CatalogPlugin), Box<CommandOutput>> {
    let matching = matching_ids(&pending.plugin, &ids_for(found));
    let id = matching.first();
    let entry = id.and_then(|id| found.graph.catalog.get(id.as_str()));
    let (Some(id), Some(entry)) = (id, entry) else {
        return Err(Box::new(failed(
            &format!(
                "Plugin \"{}\" not found in any configured marketplace",
                pending.plugin
            ),
            "not_found",
        )));
    };
    if matching.len() == 1 {
        return Ok((id.clone(), entry));
    }
    Err(Box::new(failed(
        &format!(
            "\"{}\" is offered by more than one marketplace; run one of: {}",
            pending.plugin,
            qualified_commands("install", &matching).join(", ")
        ),
        "ambiguous_plugin",
    )))
}

/// The refusal of a marketplace `--marketplace` just added, by the checks of `marketplace add` (an
/// invalid or reserved name, the download-folder rule): `invalid_name`, before any lookup. Nothing
/// prints ahead of it (judgment: the add did not happen).
#[must_use]
pub fn marketplace_refusal(
    pending: &PendingInstall,
    found: &InstallFound,
) -> Option<CommandOutput> {
    let added = found.added_marketplace.as_ref()?;
    if added.already_known {
        return same_name_refusal(pending, added);
    }
    let source = pending.add_marketplace.as_ref().map(|add| &add.source);
    name_refusal(&added.name, &added.registered, source)
        .map(|reason| failed(&reason, "invalid_name"))
}

/// A catalog whose name is a marketplace already added from another source: the existing one is kept
/// and nothing is installed (§17.20.2). Judgment: the failure code is `invalid_source`
/// (spec/judgments.md).
fn same_name_refusal(pending: &PendingInstall, added: &AddedMarketplace) -> Option<CommandOutput> {
    let given = &pending.add_marketplace.as_ref()?.source;
    let existing = added.existing_source.as_ref()?;
    let same = MarketplaceSource::deserialize(existing.clone())
        .is_ok_and(|existing| same_marketplace_source(&existing, given));
    if same {
        return None;
    }
    Some(failed(
        &already_added_from_different_source(&added.name, existing),
        "invalid_source",
    ))
}

/// What prints ahead of the result: the marketplace that was added, a refresh that failed.
#[must_use]
pub fn before_lines(found: &InstallFound) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(added) = found
        .added_marketplace
        .as_ref()
        .filter(|added| !added.already_known)
    {
        lines.push(format!(
            "Successfully added marketplace: {} (declared in user settings)",
            added.name
        ));
    }
    if found.refresh_failed == Some(true) {
        lines.push(NOT_REFRESHED.to_owned());
    }
    lines
}

/// `SOURCE_COMMANDS`: per source kind, the command it asks Claude Code to run: only a `command`
/// source has one.
fn source_command(kind: &str, source: &JsValue) -> Option<String> {
    match kind {
        "command" => source
            .get("command")
            .and_then(JsValue::as_str)
            .map(str::to_owned),
        // relative, github, url, git-subdir, npm, archive: `NO_COMMAND`.
        _ => None,
    }
}

/// The command the entry asks Claude Code to run: a `headersHelper` first, then a `command` source.
#[must_use]
pub fn displayed_of(id: &str, entry: &CatalogPlugin) -> Option<DisplayedCommand> {
    if let Some(helper) = &entry.headers_helper {
        return Some(DisplayedCommand {
            command: helper.clone(),
            plugin: id.to_owned(),
            origin: CommandOrigin::HeadersHelper,
        });
    }
    let kind = entry.source.get("kind").and_then(JsValue::as_str)?;
    source_command(kind, &entry.source).map(|command| DisplayedCommand {
        command,
        plugin: id.to_owned(),
        origin: CommandOrigin::CommandSource,
    })
}
