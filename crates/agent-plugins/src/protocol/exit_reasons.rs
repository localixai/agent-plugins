//! Port of `src/protocol/exit-reasons.ts`, switched.
//!
//! Why a session ends (mods reference.md, "Session": `e.reason` of `session.end`; `/branch` reports
//! `resume`), tied to the official `ExitReason` union: an SDK update that adds or drops a reason
//! fails the typecheck of the TypeScript wrapper.

/// Why a session ends (mods reference.md, "Session": `e.reason` of `session.end`; `/branch`
/// reports `resume`), tied to the official `ExitReason` union.
pub const SESSION_END_REASONS: [&str; 5] =
    ["clear", "resume", "logout", "prompt_input_exit", "other"];

crate::registry::exports! {
    "protocol/exit-reasons" {
        "SESSION_END_REASONS" => ok || SESSION_END_REASONS;
    }
}
