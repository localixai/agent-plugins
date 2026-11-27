//! Port of `src/cli/flags/sideload.ts`, switched.
//!
//! What managed `disableSideloadFlags` rejects (plugins/org.md, control matrix: "`--plugin-dir`,
//! `--plugin-url`, `--agents`, the Agent SDK `plugins` option, and folders named in the
//! `CLAUDE_CODE_PLUGIN_DIRS` variable at startup, and `--mcp-config` too unless every server in it
//! is an in-process `type: "sdk"` entry"; plugins/cli-reference.md, "Flags that load a plugin for
//! one session": Claude Code "prints that the flag is disabled by your organization's managed
//! settings and exits `1` without starting"). The wording follows `claude --plugin-dir ./x`'s
//! "message naming `disableSideloadFlags`" (plugins/org.md) and is the one
//! `mods/load/plugin-dirs.ts` prints.

use super::types::FlagFacts;
use crate::js::trim;
use crate::settings::managed::sideload::SIDELOAD_CLOSING;

/// The flags it rejects, as `FlagName`s spell them.
pub const SIDELOAD_FLAGS: [&str; 4] = ["--plugin-dir", "--plugin-url", "--agents", "--mcp-config"];

/// The refusal of the flag or variable `name`.
#[must_use]
pub fn sideload_message(name: &str) -> String {
    format!("{name} is disabled by your organization's managed settings (disableSideloadFlags)")
}

/// The refusal for `flag`, or `None` when managed settings allow it.
#[must_use]
pub fn sideload_refusal(flag: &str, facts: &FlagFacts) -> Option<String> {
    (facts.sideload_disabled && SIDELOAD_FLAGS.contains(&flag)).then(|| sideload_message(flag))
}

/// The refusal for the folders `CLAUDE_CODE_PLUGIN_DIRS` names, when it names any: it says to
/// unset it (J14.8).
#[must_use]
pub fn plugin_dirs_variable_refusal(facts: &FlagFacts) -> Option<String> {
    let named = facts
        .env
        .get("CLAUDE_CODE_PLUGIN_DIRS")
        .and_then(Option::as_deref)
        .map_or("", trim);
    (facts.sideload_disabled && !named.is_empty()).then(|| {
        format!(
            "{}: unset the variable",
            sideload_message("CLAUDE_CODE_PLUGIN_DIRS")
        )
    })
}

const SIDELOAD_MARK: &str = "(disableSideloadFlags)";

/// The refusals with the docs' closing line after the rejected flags, once, when any was rejected
/// (plugins/troubleshooting.md, "`--plugin-dir is disabled …`"), as `sideloadVerdict` words it.
#[must_use]
pub fn with_sideload_closing(refusals: Vec<String>) -> Vec<String> {
    let sideloaded = refusals.iter().any(|line| line.contains(SIDELOAD_MARK));
    if sideloaded {
        let mut all = refusals;
        all.push(SIDELOAD_CLOSING.to_owned());
        all
    } else {
        refusals
    }
}

crate::registry::exports! {
    "cli/flags/sideload" {
        "SIDELOAD_FLAGS" => ok || SIDELOAD_FLAGS;
        "sideloadMessage" => ok |name: String| sideload_message(&name);
    }
}
