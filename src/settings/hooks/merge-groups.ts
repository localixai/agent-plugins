/**
 * The `hooks` key across settings files (settings-reference.md, `hooks`: "Hooks merge across files
 * rather than replacing each other, and hooks from managed settings can't be removed from other
 * files"; hooks.md, "Hook locations"). Every file's matcher groups are kept, per event, so a file
 * adds hooks and never takes one away. A handler defined in more than one file runs once (hooks.md,
 * "Hook handler fields"): the copy from the lower file is dropped from a group with the same
 * matcher. The same handler under two matchers is kept: which groups match one occurrence is known
 * only when an event fires, so dropping that repeat is the hooks engine's (an open requirement,
 * spec/judgments.md). Only settings files come here; a plugin's or skill's copy stays separate.
 * Running the hooks is the hooks engine's; this only combines what the files define.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/hooks/merge_groups.rs`, docs/RUST.md);
 * `isHooksSettings`, a guard over a host value, stays here (docs/RUST.md, "What stays TypeScript").
 */
import type { HooksSettings } from '../../protocol/hook-config.ts';
import { isRecord } from '../../protocol/is-record.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/hooks/merge-groups');

/** A value that has the shape of an event map (an object of arrays); the reader checks the rest. */
export const isHooksSettings = (value: unknown): value is HooksSettings =>
  isRecord(value) && Object.values(value).every(Array.isArray);

/**
 * Combines the `hooks` of several files, highest precedence first: per event, the groups of the
 * highest file come first, then those of the next, and so on. Within one file nothing is dropped.
 */
export const mergeHookSettings = (files: readonly HooksSettings[]): HooksSettings =>
  call('mergeHookSettings', files);
