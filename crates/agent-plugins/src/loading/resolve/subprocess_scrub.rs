//! Port of `src/loading/resolve/subprocess-scrub.ts`, switched.
//!
//! `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` (env-vars.md, "Variables" and "What the subprocess environment
//! scrub removes"): with it on, "Claude Code removes credentials from the environments of the
//! subprocesses it starts, such as Bash commands, hooks, and stdio MCP servers" and keeps them for
//! its own API calls. The table (`format/credential-env`, `scrubAction`) decides per variable; this
//! file applies it to a snapshot, pure.
//!
//! Host duties the kit cannot do (L3): on Linux the scrub "also runs Bash subprocesses in an isolated
//! PID namespace so they can't read host process environments through `/proc`" (a process option,
//! not an environment edit), and `claude-code-action` sets the variable itself when
//! `allowed_non_write_users` is configured.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::format::credential_env::{ScrubAction, scrub_action, without_userinfo};
use crate::format::env_flag::is_truthy_env;
use crate::format::versions::compare_versions;

/// The switch's own name: the one variable the scrub reads.
const SWITCH: &str = "CLAUDE_CODE_SUBPROCESS_ENV_SCRUB";

/// Whether the snapshot turns the scrub on: an on/off variable (`format/env-flag`).
#[must_use]
pub fn subprocess_scrub_enabled(snapshot: &IndexMap<String, String>) -> bool {
    is_truthy_env(snapshot.get(SWITCH).map(String::as_str))
}

/// The value a variable has after an action; `None` removes it.
fn apply(action: ScrubAction, value: &str) -> Option<String> {
    match action {
        ScrubAction::Remove => None,
        ScrubAction::Keep => Some(value.to_owned()),
        ScrubAction::CutUserinfo => Some(without_userinfo(value)),
    }
}

/// "Before v2.1.251, the scrub removed `ANTHROPIC_API_KEY` and `AWS_SECRET_ACCESS_KEY` and left the table's other example variables unchanged."
static LEGACY_REMOVED: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["ANTHROPIC_API_KEY", "AWS_SECRET_ACCESS_KEY"]));

/// The two scrubbers: the one before `2.1.251` and the table.
#[derive(Clone, Copy)]
enum Scrubber {
    Legacy,
    Table,
}

impl Scrubber {
    fn scrub(self, name: &str, value: &str) -> Option<String> {
        match self {
            Self::Legacy => (!LEGACY_REMOVED.contains(name)).then(|| value.to_owned()),
            Self::Table => apply(scrub_action(name, value), value),
        }
    }
}

/// The table applies from `2.1.251` on; a version the kit cannot read is the current table (judgment).
fn scrubber_for(version: &str) -> Scrubber {
    if compare_versions(version, "2.1.251").unwrap_or(Ordering::Equal) == Ordering::Less {
        Scrubber::Legacy
    } else {
        Scrubber::Table
    }
}

/// What `scrubEnvironment` is given besides the snapshot.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ScrubOptions {
    /// The Claude Code release whose behaviour to follow, such as `2.1.319`; absent is the current table.
    #[serde(default)]
    pub version: Option<String>,
}

/// The environment a subprocess gets: `snapshot` with the credentials the scrub removes taken out and
/// the userinfo cut from registry URLs, when `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` is on; the snapshot
/// unchanged (a copy) when it is not. A secret whose name and value look like no credential stays:
/// "use it as one layer alongside narrow permission rules".
#[must_use]
pub fn scrub_environment(
    snapshot: &IndexMap<String, String>,
    options: &ScrubOptions,
) -> IndexMap<String, String> {
    if !subprocess_scrub_enabled(snapshot) {
        return snapshot.clone();
    }
    let scrubber = options
        .version
        .as_deref()
        .map_or(Scrubber::Table, scrubber_for);
    let mut out = IndexMap::new();
    for (name, value) in snapshot {
        if let Some(kept) = scrubber.scrub(name, value) {
            out.insert(name.clone(), kept);
        }
    }
    out
}

crate::registry::exports! {
    "loading/resolve/subprocess-scrub" {
        "subprocessScrubEnabled" => ok |snapshot: IndexMap<String, String>|
            subprocess_scrub_enabled(&snapshot);
        "scrubEnvironment" => ok |snapshot: IndexMap<String, String>, options: Option<ScrubOptions>|
            scrub_environment(&snapshot, &options.unwrap_or_default());
    }
}
