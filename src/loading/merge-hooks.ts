import type { HooksSettings } from '../protocol/hook-config.ts';
import { coreModule } from '../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/loading/merge_hooks.rs`, docs/RUST.md). */
const call = coreModule('loading/merge-hooks');

/**
 * Joins the hooks of a plugin's sources: "Claude Code merges whatever you declare with
 * `hooks/hooks.json` when that file exists" and "Hooks in `hooks/hooks.json` and in the `hooks`
 * manifest key both load" (manifest-reference.md, "`hooks`"; components.md, "Hooks"). Judgment
 * (spec/judgments.md): merging is per event, the matcher groups of each source one after the other
 * in loading order (the default file first), so no source's hook is lost or overwritten.
 */
export const mergeHooks = (sources: readonly HooksSettings[]): HooksSettings =>
  call('mergeHooks', sources);
