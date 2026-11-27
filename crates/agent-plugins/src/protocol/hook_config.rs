//! Port of `src/protocol/hook-config.ts`, switched.
//!
//! The shape of hook configuration, derived from the official `Settings` type (the `hooks` key of
//! `settings.json`, which a plugin's `hooks/hooks.json` wraps the same way; hooks.md,
//! "Configuration"). Nothing here is written by hand: an SDK update that adds a handler field or
//! type changes these types with it. The types are the SDK's and stay in the TypeScript wrapper;
//! the one list of values is here.

/// Every handler type, in the order hooks.md lists them ("Hook handler fields").
pub const HOOK_HANDLER_TYPES: &[&str] = &["command", "http", "mcp_tool", "prompt", "agent"];

crate::registry::exports! {
    "protocol/hook-config" {
        "HOOK_HANDLER_TYPES" => ok || HOOK_HANDLER_TYPES;
    }
}
