//! Port of `src/loading/repository-messages.ts`, switched.
//!
//! The texts of repository plugins (plugins/loading.md, "Trust check and component limits for
//! repository plugins" and "Check why a repository plugin didn't load"). The docs quote no message for
//! a skipped monitor or bundle; the wording is the kit's (spec/judgments.md).

use serde::Serialize;

use super::repository_trust::{
    RepositoryDecision, RepositoryPluginFacts, RepositoryReason, SkillsDirBlock,
    decide_repository_plugin, is_repository_supplied, repository_causes,
};
use super::trust_gates::TrustSituation;

/// Why a repository plugin's monitors do not load.
pub const MONITORS_NOT_LOADED: &str = "Background monitors do not load from a repository plugin";

/// Why a repository plugin's bundles do not load.
pub const BUNDLES_NOT_LOADED: &str = "a repository plugin cannot load an MCP bundle: declare the server inline or in a .mcp.json inside the plugin directory";

/// What the docs say about each cause (plugins/loading.md, "Check why a repository plugin didn't load").
const fn managed_cause(block: SkillsDirBlock) -> &'static str {
    match block {
        SkillsDirBlock::StrictPluginOnlyCustomization => {
            "customization is locked to plugins (strictPluginOnlyCustomization), which skips the whole skills-directory plugin when any one kind is locked"
        }
        SkillsDirBlock::StrictKnownMarketplaces => {
            "the marketplace allowlist (strictKnownMarketplaces) has no { \"source\": \"skills-dir\" } entry"
        }
        SkillsDirBlock::BlockedMarketplaces => {
            "the marketplace blocklist (blockedMarketplaces) names { \"source\": \"skills-dir\" }"
        }
    }
}

const MANAGED_TAIL: &str = "/plugin and `claude plugin list` show no plugin and no error for a skipped folder; run /status and look for `Enterprise managed settings` on the `Setting sources` line";

/// What the trust cause adds for the folder's situation.
const fn trust_extra(situation: TrustSituation) -> &'static str {
    match situation {
        TrustSituation::Trusted | TrustSituation::Dialog => "",
        TrustSituation::ParentTrusted => " (trusting a parent folder is not enough)",
        TrustSituation::NonInteractive => {
            " (a `claude -p` or SDK run never shows the dialog, so it is no substitute)"
        }
    }
}

fn trust_cause(facts: RepositoryPluginFacts) -> String {
    format!(
        "the folder is not trusted: the plugin loads only after you accept the workspace trust dialog for that folder{}",
        trust_extra(facts.trust)
    )
}

/// One sentence per cause, the same wording whichever origin; `None` for a plugin that loads.
fn cause(reason: RepositoryReason, facts: RepositoryPluginFacts) -> Option<String> {
    match reason {
        RepositoryReason::Trusted | RepositoryReason::UserLevel => None,
        RepositoryReason::BlockedByManagedSettings => facts
            .managed_block
            .map(|block| format!("{}; {MANAGED_TAIL}", managed_cause(block))),
        RepositoryReason::NotPrimaryDirectory => Some(
            "the session did not start where the plugin is: Claude Code reads the .claude/skills/ of the session's primary working directory and does not search parent directories for plugins; launch from the repository root, or move the session there with /cd (v2.1.246 or later)".to_owned(),
        ),
        RepositoryReason::FolderNotTrusted => Some(trust_cause(facts)),
    }
}

/// Where a plugin of a worktree is read from.
pub const WORKTREE_NOTE: &str = "in a worktree the plugin loads from the main checkout's .claude/skills/ rather than the worktree's";

/// The decision and, when the plugin did not load, every cause in words.
#[derive(Clone, Debug, Serialize)]
pub struct RepositoryExplanation {
    /// The decision.
    pub decision: RepositoryDecision,
    /// Why it did not load: one sentence per cause that applies, in the decision's order.
    pub causes: Vec<String>,
    /// What else is worth knowing: where the plugin was read from.
    pub notes: Vec<String>,
}

/// The decision of `decideRepositoryPlugin` and, when the plugin did not load, every cause in words.
#[must_use]
pub fn explain_repository_plugin(facts: &RepositoryPluginFacts) -> RepositoryExplanation {
    RepositoryExplanation {
        decision: decide_repository_plugin(facts),
        causes: repository_causes(facts)
            .into_iter()
            .filter_map(|reason| cause(reason, *facts))
            .collect(),
        notes: if facts.worktree && is_repository_supplied(facts.origin) {
            vec![WORKTREE_NOTE.to_owned()]
        } else {
            Vec::new()
        },
    }
}

crate::registry::exports! {
    "loading/repository-messages" {
        "MONITORS_NOT_LOADED" => ok || MONITORS_NOT_LOADED;
        "BUNDLES_NOT_LOADED" => ok || BUNDLES_NOT_LOADED;
        "WORKTREE_NOTE" => ok || WORKTREE_NOTE;
        "explainRepositoryPlugin" => ok |facts: RepositoryPluginFacts| explain_repository_plugin(&facts);
    }
}
