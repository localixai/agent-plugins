//! Ported from `src/hooks/verdict/plain-stdout.ts`; only the core uses it now, so it has no wrapper.
//!
//! Plain stdout on exit 0 that reaches Claude (hooks.md, "Exit code 0"): "`UserPromptSubmit`,
//! `UserPromptExpansion`, `SessionStart`, and `PostModelSwitch`: Claude Code adds plain-text stdout
//! as context that Claude can see and act on. It doesn't add stdout it failed to parse. `PreCompact`:
//! your stdout becomes instructions for the compaction summary". On every other event it goes to the
//! debug log.

use indexmap::IndexMap;

/// The events whose plain stdout reaches Claude, and how.
#[must_use]
pub fn plain_stdout() -> IndexMap<&'static str, &'static str> {
    IndexMap::from([
        ("UserPromptSubmit", "context"),
        ("UserPromptExpansion", "context"),
        ("SessionStart", "context"),
        ("PostModelSwitch", "context"),
        ("PreCompact", "compaction-instructions"),
    ])
}
