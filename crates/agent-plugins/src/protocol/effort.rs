//! Port of `src/protocol/effort.ts`, switched.
//!
//! The named reasoning-effort levels, lowest first (skills and agents `effort` frontmatter, the
//! `effortLevel` setting, `CLAUDE_CODE_EFFORT_LEVEL`). Asserted equal to the SDK's `EffortLevel`:
//! a level Anthropic adds fails the typecheck of the TypeScript wrapper.

/// The named reasoning-effort levels, lowest first (skills and agents `effort` frontmatter, the
/// `effortLevel` setting, `CLAUDE_CODE_EFFORT_LEVEL`).
pub const EFFORT_LEVELS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// The levels a file may save: `max` is "session-only and is not written" (settings-reference.md,
/// `effortLevel` and `modelSettings` list low, medium, high and xhigh; `max` is valid only for
/// `maxEffortLevel`). Asserted equal to the SDK's `Settings['effortLevel']`.
pub const SAVED_EFFORT_LEVELS: &[&str] = &["low", "medium", "high", "xhigh"];

crate::registry::exports! {
    "protocol/effort" {
        "EFFORT_LEVELS" => ok || EFFORT_LEVELS;
        "SAVED_EFFORT_LEVELS" => ok || SAVED_EFFORT_LEVELS;
    }
}
