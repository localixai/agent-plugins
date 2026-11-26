//! Port of `src/loading/local-settings-trust.ts`, switched.
//!
//! When `.claude/settings.local.json` needs trust (permissions.md, "When your local settings file
//! needs trust"). The file "is normally your own", so its allow rules and additional directories apply
//! without the trust step; "when the file is tracked in git, or `.claude` is a symlink", it is
//! repository-supplied and its rules wait for trust. Pure over the facts the host reports (git is the
//! host's effect, run only once the folder is trusted).

use serde::{Deserialize, Serialize};

use super::trust_gates::{RepositoryContent, TrustSituation, content_gate};

/// What the host reports of the file.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's JSON shape: one fact per flag"
)]
pub struct LocalSettingsFacts {
    /// The folder's trust (`trustSituation`): the dialog accepted for the exact folder (`trusted`), only
    /// for a parent whose trust extends to it, never (`dialog`), or a `-p` or SDK session, which "counts
    /// as accepted" for running git.
    pub trust: TrustSituation,
    /// The folder is the user's configuration home: the home directory, or one whose `.claude` is `CLAUDE_CONFIG_DIR`.
    pub in_configuration_home: bool,
    /// That `CLAUDE_CONFIG_DIR` directory sits inside a git repository and the local settings are kept
    /// at the repository root instead.
    pub config_home_kept_at_repository_root: bool,
    /// What git said, read only when `runsGit` is true: the file is tracked in a repository.
    pub tracked_in_git: bool,
    /// `.claude` is a symlink.
    pub claude_dir_is_symlink: bool,
}

/// What applies, and whether git runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSettingsVerdict {
    /// The file's allow rules and additional directories apply now.
    pub rules_apply: bool,
    /// Claude Code runs git to tell the cases apart.
    pub runs_git: bool,
}

/// Whether git may run: "once you've trusted the folder", for it or a parent, or in a `-p` or SDK session.
const fn git_may_run(trust: TrustSituation) -> bool {
    match trust {
        TrustSituation::Trusted
        | TrustSituation::ParentTrusted
        | TrustSituation::NonInteractive => true,
        TrustSituation::Dialog => false,
    }
}

/// What the local settings file's rules do, and whether git runs.
#[must_use]
pub const fn local_settings_verdict(facts: LocalSettingsFacts) -> LocalSettingsVerdict {
    if !git_may_run(facts.trust) {
        // The configuration-home exception skips only the trust step, unless the settings live at a repository root.
        let own = facts.in_configuration_home && !facts.config_home_kept_at_repository_root;
        return LocalSettingsVerdict {
            rules_apply: own,
            runs_git: false,
        };
    }
    let repository_supplied = facts.tracked_in_git || facts.claude_dir_is_symlink;
    // A repository-supplied file's rules are project allow rules: the trust table's `allow-rules` row.
    let released = content_gate(RepositoryContent::AllowRules, facts.trust).applies;
    LocalSettingsVerdict {
        rules_apply: !repository_supplied || released,
        runs_git: true,
    }
}

crate::registry::exports! {
    "loading/local-settings-trust" {
        "localSettingsVerdict" => ok |facts: LocalSettingsFacts| local_settings_verdict(facts);
    }
}
