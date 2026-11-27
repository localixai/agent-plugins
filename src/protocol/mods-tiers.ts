/**
 * Implemented by the Rust core (`crates/agent-plugins/src/protocol/mods_tiers.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('protocol/mods-tiers');

/**
 * A mod's priority group, in the order the chain runs them (events.md, "The order mods run in"):
 * mods your organization lists in `prependPlugins`, mods you install, `appendPlugins`, other mods
 * built into Claude Code, then Claude Code's own behaviour (`core`).
 */
export const MOD_TIERS: readonly ['prepend', 'user', 'append', 'builtin', 'core'] =
  call('MOD_TIERS');

export type ModTier = (typeof MOD_TIERS)[number];
