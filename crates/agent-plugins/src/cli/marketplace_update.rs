//! Port of `src/cli/marketplace-update.ts`, switched.
//!
//! `claude plugin marketplace update [name]` as a plan (plugins/cli-reference.md, "plugin marketplace
//! update"; plugins/install.md, "Update plugins now"; plugins/troubleshooting.md, "Marketplace is
//! seed-managed"): which marketplaces' listings are refreshed, from what, and what is printed. It
//! refreshes listings only (`planUpdate`'s `marketplace-listing`): installed plugins keep their
//! versions. In a session the line is `✔ Updated 1 marketplace` (plugins/troubleshooting.md, "Install
//! a plugin"). Pure; the host fetches. Judgment (spec/judgments.md): a session counts every listing
//! it refreshed the way a shell does, `✔ Updated N marketplaces`, 0 for an empty registry. After a
//! git refresh that left LFS files as pointer files the host adds `lfsPointerLine(count)`.

use serde::{Deserialize, Serialize};

use super::result::{
    CommandOutput, MarketplaceCommand, marketplace_failure, marketplace_success, usage_error,
};
use crate::updates::update_targets::{UpdateRequest, plan_update};

/// Where the command was typed (`marketplace/install-plans.ts`): a shell or a Claude Code session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandSurface {
    /// A shell.
    Shell,
    /// A Claude Code session.
    Session,
}

/// The command's arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateArguments {
    /// Absent: every marketplace.
    #[serde(default)]
    pub name: Option<String>,
    /// `--json`.
    #[serde(default)]
    pub json: Option<bool>,
    /// Where the command was typed; absent: a shell.
    #[serde(default)]
    pub surface: Option<CommandSurface>,
}

/// One registered marketplace.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredForUpdate {
    /// Its name.
    pub name: String,
    /// The branch or tag it was added with.
    #[serde(default, rename = "ref")]
    pub reference: Option<String>,
    /// `CLAUDE_CODE_PLUGIN_SEED_DIR` supplied it: the directory; it is read-only.
    #[serde(default)]
    pub seeded: Option<String>,
}

/// What the host knows.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFacts {
    /// The registered marketplaces.
    pub registered: Vec<RegisteredForUpdate>,
}

/// A marketplace to refresh.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTarget {
    /// Its name.
    pub name: String,
    /// "Updates to the latest commit of that ref, not the repository's default branch."
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// What the command does.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceUpdatePlan {
    /// What is printed.
    pub output: CommandOutput,
    /// The listings to refresh.
    pub refresh: Vec<RefreshTarget>,
    /// Plugin ids updated: always none, "leaves your installed plugins at their current versions".
    pub update: Vec<String>,
}

fn nothing(output: CommandOutput) -> MarketplaceUpdatePlan {
    MarketplaceUpdatePlan {
        output,
        refresh: Vec::new(),
        update: Vec::new(),
    }
}

fn target_of(marketplace: &RegisteredForUpdate) -> RefreshTarget {
    RefreshTarget {
        name: marketplace.name.clone(),
        reference: marketplace.reference.clone(),
    }
}

fn plural(count: usize) -> String {
    format!(
        "{count} {}",
        if count == 1 {
            "marketplace"
        } else {
            "marketplaces"
        }
    )
}

/// The success line: `Successfully updated ...` from a shell, `✔ Updated ...` in a session.
fn updated(surface: CommandSurface, count: usize, name: Option<&str>) -> String {
    match surface {
        CommandSurface::Shell => match name {
            None => format!("Successfully updated {}", plural(count)),
            Some(name) => format!("Successfully updated marketplace: {name}"),
        },
        CommandSurface::Session => format!("✔ Updated {}", plural(count)),
    }
}

fn named(
    name: &str,
    found: Option<&RegisteredForUpdate>,
    surface: CommandSurface,
) -> MarketplaceUpdatePlan {
    let Some(found) = found else {
        return nothing(marketplace_failure(
            MarketplaceCommand::MarketplaceUpdate,
            Some(name),
            &format!("Marketplace '{name}' not found"),
            "not_found",
        ));
    };
    if let Some(seeded) = &found.seeded {
        return nothing(marketplace_failure(
            MarketplaceCommand::MarketplaceUpdate,
            Some(name),
            &format!(
                "Marketplace '{name}' is seed-managed ({seeded}). Ask your administrator to update the seed directory"
            ),
            "seed_managed",
        ));
    }
    MarketplaceUpdatePlan {
        output: marketplace_success(
            MarketplaceCommand::MarketplaceUpdate,
            Some(name),
            &updated(surface, 1, Some(name)),
        ),
        refresh: vec![target_of(found)],
        update: Vec::new(),
    }
}

/// What `plugin marketplace update` refreshes and prints.
#[must_use]
pub fn plan_marketplace_update(
    args: &UpdateArguments,
    facts: &UpdateFacts,
) -> MarketplaceUpdatePlan {
    let surface = args.surface.unwrap_or(CommandSurface::Shell);
    if let Some(name) = &args.name {
        return named(
            name,
            facts
                .registered
                .iter()
                .find(|marketplace| marketplace.name == *name),
            surface,
        );
    }
    if args.json == Some(true) {
        return nothing(usage_error(
            "--json needs a marketplace name: pass the marketplace to update",
        ));
    }
    let seedless: Vec<String> = facts
        .registered
        .iter()
        .filter(|marketplace| marketplace.seeded.is_none())
        .map(|one| one.name.clone())
        .collect();
    let listed = plan_update(
        &UpdateRequest::MarketplaceListing { marketplace: None },
        &[],
        &seedless,
    );
    let refresh: Vec<RefreshTarget> = facts
        .registered
        .iter()
        .filter(|marketplace| listed.refresh.contains(&marketplace.name))
        .map(target_of)
        .collect();
    MarketplaceUpdatePlan {
        output: marketplace_success(
            MarketplaceCommand::MarketplaceUpdate,
            None,
            &updated(surface, refresh.len(), None),
        ),
        refresh,
        update: listed.update,
    }
}

crate::registry::exports! {
    "cli/marketplace-update" {
        "planMarketplaceUpdate" => ok |args: UpdateArguments, facts: UpdateFacts| plan_marketplace_update(&args, &facts);
    }
}
