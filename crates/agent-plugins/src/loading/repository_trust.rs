//! Port of `src/loading/repository-trust.ts`, switched.
//!
//! Whether a plugin that a repository supplies loads (plugins/loading.md, "Trust check and component
//! limits for repository plugins" and "Check why a repository plugin didn't load"). Pure: the host
//! answers the facts (`WorkspaceTrust` tells it whether the folder is trusted), the kit decides.
//!
//! A project-scope plugin "loads only after the same trust check that applies to project allow rules
//! in `.claude/settings.json`. Trusting a parent folder or running with `-p` isn't enough." A plugin
//! under the user's own `~/.claude/skills/` is exempt from the check.

use serde::{Deserialize, Serialize};

use super::trust_gates::{RepositoryContent, TrustSituation, content_gate};

/// What stops skills-directory plugins in managed settings (plugins/loading.md, "Skills-directory
/// plugins blocked by managed settings"), as `src/loading/repository-types.ts` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum SkillsDirBlock {
    /// `strictPluginOnlyCustomization`.
    #[serde(rename = "strictPluginOnlyCustomization")]
    StrictPluginOnlyCustomization,
    /// `strictKnownMarketplaces`.
    #[serde(rename = "strictKnownMarketplaces")]
    StrictKnownMarketplaces,
    /// `blockedMarketplaces`.
    #[serde(rename = "blockedMarketplaces")]
    BlockedMarketplaces,
}

/// The places a repository-supplied (or user-skills-folder) plugin comes from: a `@skills-dir` plugin
/// of the project's `.claude/skills/`, a project-scope `@<marketplace>` plugin (enabled in the
/// repository's settings), and a `@skills-dir` plugin of the user's own `~/.claude/skills/`.
pub const REPOSITORY_ORIGINS: [RepositoryOrigin; 3] = [
    RepositoryOrigin::SkillsDir,
    RepositoryOrigin::MarketplaceProjectScope,
    RepositoryOrigin::UserSkillsDir,
];

/// One of `REPOSITORY_ORIGINS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepositoryOrigin {
    /// `skills-dir`.
    #[serde(rename = "skills-dir")]
    SkillsDir,
    /// `marketplace-project-scope`.
    #[serde(rename = "marketplace-project-scope")]
    MarketplaceProjectScope,
    /// `user-skills-dir`.
    #[serde(rename = "user-skills-dir")]
    UserSkillsDir,
}

/// What the host knows about the session and the folder.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryPluginFacts {
    /// Where the plugin comes from.
    pub origin: RepositoryOrigin,
    /// The folder's trust (`trustSituation`, `workspaceTrustState`): only `trusted`, the dialog accepted
    /// for this exact folder, lets a checked plugin load; a parent folder's trust and a `claude -p` or SDK
    /// run never do, and the messages say which of them it was.
    pub trust: TrustSituation,
    /// The session's primary working directory is the directory whose `.claude/skills/` holds the plugin.
    pub primary_dir_is_root: bool,
    /// The session runs in a worktree started from the desktop app.
    pub worktree: bool,
    /// The restriction of managed settings that stops skills-directory plugins, when one applies.
    #[serde(default)]
    pub managed_block: Option<SkillsDirBlock>,
}

/// Every reason a decision gives, in the package's order.
pub const REPOSITORY_REASONS: [RepositoryReason; 5] = [
    RepositoryReason::Trusted,
    RepositoryReason::UserLevel,
    RepositoryReason::BlockedByManagedSettings,
    RepositoryReason::NotPrimaryDirectory,
    RepositoryReason::FolderNotTrusted,
];

/// One of `REPOSITORY_REASONS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RepositoryReason {
    /// `trusted`.
    #[serde(rename = "trusted")]
    Trusted,
    /// `user-level`.
    #[serde(rename = "user-level")]
    UserLevel,
    /// `blocked-by-managed-settings`.
    #[serde(rename = "blocked-by-managed-settings")]
    BlockedByManagedSettings,
    /// `not-primary-directory`.
    #[serde(rename = "not-primary-directory")]
    NotPrimaryDirectory,
    /// `folder-not-trusted`.
    #[serde(rename = "folder-not-trusted")]
    FolderNotTrusted,
}

/// Where the plugin's files are read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RepositorySource {
    /// `primary-directory`.
    #[serde(rename = "primary-directory")]
    PrimaryDirectory,
    /// `main-checkout`.
    #[serde(rename = "main-checkout")]
    MainCheckout,
    /// `user-skills-directory`.
    #[serde(rename = "user-skills-directory")]
    UserSkillsDirectory,
}

/// Whether the plugin loads, why, and where from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RepositoryDecision {
    /// It loads.
    pub load: bool,
    /// Why.
    pub reason: RepositoryReason,
    /// Where its files are read from.
    pub from: RepositorySource,
}

/// What differs per origin: one row each, so a new origin fails to compile until it has its rules.
struct OriginRules {
    /// The row of the trust table (`trust-gates`) whose check applies, or none: a skills-directory plugin
    /// is its own row, a project-scope plugin "loads only after the same trust check that applies to
    /// project allow rules".
    trust_gate: Option<RepositoryContent>,
    /// Found by reading the primary directory's `.claude/skills/`, not by a parent search or a setting.
    located_by_primary_directory: bool,
    /// A folder of `~/.claude/skills/` or `.claude/skills/`, which managed settings may block.
    skills_directory: bool,
    /// Where the files come from, in a worktree or not.
    source: fn(bool) -> RepositorySource,
}

const fn from_checkout(worktree: bool) -> RepositorySource {
    if worktree {
        RepositorySource::MainCheckout
    } else {
        RepositorySource::PrimaryDirectory
    }
}

const fn user_skills_directory(_worktree: bool) -> RepositorySource {
    RepositorySource::UserSkillsDirectory
}

const fn origin_rules(origin: RepositoryOrigin) -> OriginRules {
    match origin {
        RepositoryOrigin::SkillsDir => OriginRules {
            trust_gate: Some(RepositoryContent::SkillsDirPlugin),
            located_by_primary_directory: true,
            skills_directory: true,
            source: from_checkout,
        },
        RepositoryOrigin::MarketplaceProjectScope => OriginRules {
            trust_gate: Some(RepositoryContent::AllowRules),
            located_by_primary_directory: false,
            skills_directory: false,
            source: from_checkout,
        },
        RepositoryOrigin::UserSkillsDir => OriginRules {
            trust_gate: None,
            located_by_primary_directory: false,
            skills_directory: true,
            source: user_skills_directory,
        },
    }
}

/// One cause that keeps a plugin from loading, and whether it applies.
struct Check {
    reason: RepositoryReason,
    stops: fn(&RepositoryPluginFacts, &OriginRules) -> bool,
}

/// The causes of the docs' list, in the order the decision reports them (judgment, spec/judgments.md:
/// the docs list them without an order). `~/.claude/skills/` plugins skip the trust check, not the
/// managed block.
const CHECKS: [Check; 3] = [
    Check {
        reason: RepositoryReason::BlockedByManagedSettings,
        stops: |facts, rules| rules.skills_directory && facts.managed_block.is_some(),
    },
    Check {
        reason: RepositoryReason::NotPrimaryDirectory,
        stops: |facts, rules| rules.located_by_primary_directory && !facts.primary_dir_is_root,
    },
    Check {
        reason: RepositoryReason::FolderNotTrusted,
        stops: |facts, rules| {
            rules
                .trust_gate
                .is_some_and(|gate| !content_gate(gate, facts.trust).applies)
        },
    },
];

/// The causes that apply to these facts, in the order of `CHECKS`.
#[must_use]
pub fn repository_causes(facts: &RepositoryPluginFacts) -> Vec<RepositoryReason> {
    let rules = origin_rules(facts.origin);
    CHECKS
        .iter()
        .filter(|check| (check.stops)(facts, &rules))
        .map(|check| check.reason)
        .collect()
}

/// Whether a repository plugin loads, why not, and which checkout it is read from.
#[must_use]
pub fn decide_repository_plugin(facts: &RepositoryPluginFacts) -> RepositoryDecision {
    let rules = origin_rules(facts.origin);
    let from = (rules.source)(facts.worktree);
    if let Some(cause) = repository_causes(facts).first() {
        return RepositoryDecision {
            load: false,
            reason: *cause,
            from,
        };
    }
    RepositoryDecision {
        load: true,
        reason: if rules.trust_gate.is_none() {
            RepositoryReason::UserLevel
        } else {
            RepositoryReason::Trusted
        },
        from,
    }
}

/// Whether the origin is "repository" content: the trust check and the component limits apply to it.
/// "Neither ... apply to a plugin under your own `~/.claude/skills/`."
#[must_use]
pub const fn is_repository_supplied(origin: RepositoryOrigin) -> bool {
    origin_rules(origin).trust_gate.is_some()
}

crate::registry::exports! {
    "loading/repository-trust" {
        "REPOSITORY_ORIGINS" => ok || REPOSITORY_ORIGINS;
        "REPOSITORY_REASONS" => ok || REPOSITORY_REASONS;
        "repositoryCauses" => ok |facts: RepositoryPluginFacts| repository_causes(&facts);
        "decideRepositoryPlugin" => ok |facts: RepositoryPluginFacts| decide_repository_plugin(&facts);
        "isRepositorySupplied" => ok |origin: RepositoryOrigin| is_repository_supplied(origin);
    }
}
