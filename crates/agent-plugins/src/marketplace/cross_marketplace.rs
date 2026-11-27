//! Port of `src/marketplace/cross-marketplace.ts`, switched.
//!
//! Cross-marketplace dependencies (plugins/dependencies.md, "Depend on a plugin from another
//! marketplace"; marketplace-reference.md, "Top-level fields"): a dependency in a marketplace other
//! than the declaring plugin's installs only when the root marketplace, the one that hosts the plugin
//! the user is installing, lists it in `allowCrossMarketplaceDependenciesOn` (its list alone applies,
//! for the whole dependency chain), or when the user already has the dependency enabled at the same
//! scope. The kit decides; the host installs or refuses.

use serde::{Deserialize, Serialize};

use super::dependencies::dependency_id;
use super::types::NormalizedDependency;
use crate::js::JsValue;
use crate::registry::Set;

/// Where the dependency is declared: it decides what a refusal does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclaredIn {
    /// A marketplace entry's.
    #[serde(rename = "entry")]
    Entry,
    /// `plugin.json`'s.
    #[serde(rename = "plugin.json")]
    PluginJson,
}

/// What a refusal does: a marketplace entry's refuses the install; `plugin.json`'s installs without the dependency, and the plugin then fails to load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RefusalEffect {
    /// The install is refused.
    #[serde(rename = "refuse-install")]
    RefuseInstall,
    /// The plugin is installed without the dependency.
    #[serde(rename = "install-without-dependency")]
    InstallWithoutDependency,
}

const fn effect_of(declared_in: DeclaredIn) -> RefusalEffect {
    match declared_in {
        DeclaredIn::Entry => RefusalEffect::RefuseInstall,
        DeclaredIn::PluginJson => RefusalEffect::InstallWithoutDependency,
    }
}

/// What the kit decides.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum CrossMarketplaceDecision {
    /// The dependency installs.
    Allowed {
        /// `true`.
        allowed: bool,
    },
    /// It does not.
    Refused {
        /// `false`.
        allowed: bool,
        /// What the refusal does.
        effect: RefusalEffect,
        /// Why.
        message: String,
    },
}

/// The plugin that declares the dependency; `marketplace` is absent for one that has none (loaded from claude.ai).
#[derive(Clone, Debug, Deserialize)]
pub struct RequiredBy {
    /// Its name.
    pub name: String,
    /// Its marketplace.
    #[serde(default)]
    pub marketplace: Option<String>,
}

/// The marketplace that hosts the plugin the user is installing: its list alone decides.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootMarketplace {
    /// Its name.
    pub name: String,
    /// The marketplaces a plugin of it may depend on.
    pub allow_cross_marketplace_dependencies_on: Vec<String>,
}

/// What is asked.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossMarketplaceQuestion {
    /// The dependency.
    pub dep: NormalizedDependency,
    /// The plugin that declares the dependency.
    pub required_by: RequiredBy,
    /// The marketplace that hosts the plugin the user is installing.
    pub root: RootMarketplace,
    /// The marketplace of the declaring plugin when it is not the root, for a link deeper in the
    /// dependency chain. Its own `allowCrossMarketplaceDependenciesOn` is never consulted: only the
    /// root's list applies, for the whole chain (marketplace-reference.md, "Top-level fields").
    #[serde(default)]
    pub declaring_marketplace: Option<JsValue>,
    /// `name@marketplace` of every plugin already enabled at the scope the install is for.
    pub enabled_at_scope: Set<String>,
    /// Where it is declared.
    pub declared_in: DeclaredIn,
}

fn refusal(question: &CrossMarketplaceQuestion) -> CrossMarketplaceDecision {
    let CrossMarketplaceQuestion {
        dep,
        required_by,
        root,
        ..
    } = question;
    let by = dependency_id(&NormalizedDependency {
        name: required_by.name.clone(),
        marketplace: required_by.marketplace.clone(),
        range: None,
    });
    CrossMarketplaceDecision::Refused {
        allowed: false,
        effect: effect_of(question.declared_in),
        // The text up to "allowlist" is quoted by the docs; the rest names the field to set (docs: "and names the field to set").
        message: format!(
            "Dependency \"{}\" (required by {by}) is in marketplace \"{}\", \
             which is not in the allowlist; add it to \"allowCrossMarketplaceDependenciesOn\" in the marketplace.json of \"{}\"",
            dependency_id(dep),
            dep.marketplace.as_deref().unwrap_or("undefined"),
            root.name
        ),
    }
}

/// Judgments (spec/judgments.md): a dependency in the root marketplace itself needs no entry on its
/// list (the user chose that marketplace); a name is matched exactly; a dependency with no
/// marketplace is never cross-marketplace.
#[must_use]
pub fn cross_marketplace_decision(question: &CrossMarketplaceQuestion) -> CrossMarketplaceDecision {
    let CrossMarketplaceQuestion {
        dep,
        required_by,
        root,
        ..
    } = question;
    let allowed = CrossMarketplaceDecision::Allowed { allowed: true };
    let Some(target) = dep.marketplace.as_deref() else {
        return allowed;
    };
    if Some(target) == required_by.marketplace.as_deref() || target == root.name {
        return allowed;
    }
    if root
        .allow_cross_marketplace_dependencies_on
        .iter()
        .any(|listed| listed == target)
    {
        return allowed;
    }
    if question
        .enabled_at_scope
        .members
        .contains(&dependency_id(dep))
    {
        allowed
    } else {
        refusal(question)
    }
}

crate::registry::exports! {
    "marketplace/cross-marketplace" {
        "crossMarketplaceDecision" => ok |question: CrossMarketplaceQuestion| cross_marketplace_decision(&question);
    }
}
