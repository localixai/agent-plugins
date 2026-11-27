//! Port of `src/cli/uninstall-flags.ts`, switched.
//!
//! The flags of `claude plugin uninstall` that bear on whether it can run.

use serde::Deserialize;

/// The flags of `claude plugin uninstall` that bear on whether it can run.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct UninstallFlags {
    /// `--prune`.
    #[serde(default)]
    pub prune: Option<bool>,
    /// `-y`.
    #[serde(default)]
    pub yes: Option<bool>,
    /// `--json`.
    #[serde(default)]
    pub json: Option<bool>,
}

/// `--json` and a missing `-y` are flag problems: nothing is printed but the reason.
#[must_use]
pub fn flag_problem(args: UninstallFlags, tty: bool) -> Option<&'static str> {
    if args.json == Some(true) && args.prune == Some(true) {
        return Some("--json cannot be combined with --prune");
    }
    if args.prune == Some(true) && args.yes != Some(true) && !tty {
        return Some("--prune needs -y when stdin or stdout is not a terminal");
    }
    None
}

crate::registry::exports! {
    "cli/uninstall-flags" {
        "flagProblem" => ok |args: UninstallFlags, tty: bool| flag_problem(args, tty);
    }
}
