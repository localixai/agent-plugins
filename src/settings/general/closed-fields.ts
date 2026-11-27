/**
 * The `worktree` and `attribution` blocks of a managed source, repaired per field instead of dropped
 * whole (managed-settings.md, "Keys that fail closed": "Claude Code repairs the `permissions`,
 * `autoMode`, `worktree`, and `attribution` blocks per field"). The docs give no field rule for these
 * two blocks: an invalid field is dropped with a warning and the field takes its default, except
 * `worktree.bgIsolation`, whose default `worktree` is the restrictive value and which reads as it
 * (spec/judgments.md). A field the kit does not know stays as written.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/closed_fields.rs`, docs/RUST.md).
 */
import type { Repair } from '../repair/support.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/general/closed-fields');

export const worktree: Repair = (value, at) => call('worktree', value, at);

/** `attribution` is also `false` ("to hide all attribution") or `true`. */
export const attribution: Repair = (value, at) => call('attribution', value, at);
