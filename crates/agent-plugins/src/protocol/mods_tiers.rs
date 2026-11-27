//! Port of `src/protocol/mods-tiers.ts`, switched.

/// A mod's priority group, in the order the chain runs them (events.md, "The order mods run in"):
/// mods your organization lists in `prependPlugins`, mods you install, `appendPlugins`, other mods
/// built into Claude Code, then Claude Code's own behaviour (`core`).
pub const MOD_TIERS: &[&str] = &["prepend", "user", "append", "builtin", "core"];

crate::registry::exports! {
    "protocol/mods-tiers" {
        "MOD_TIERS" => ok || MOD_TIERS;
    }
}
