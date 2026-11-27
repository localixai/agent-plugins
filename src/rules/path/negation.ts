import type { MatchContext, PermissionRule, PreparedCall } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * `!` negation (permissions.md, "Special characters and `!` negation"): "A deny or ask pattern that
 * starts with `!` is a gitignore negation. It carves the paths it matches out of the `path` or
 * `./path` rules listed before it." The carve-out reaches only rules of the same source, and only
 * deny and ask rules carve (an allow rule that starts with `!` approves nothing).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/path/negation.rs`, docs/RUST.md).
 */
const call = coreModule('rules/path/negation');

/**
 * Whether `rules[index]`, which matched `call`, is carved out of by a later `!` rule of its own
 * source (so it covers the call no longer).
 */
export const isCarvedOut = (
  rules: readonly PermissionRule[],
  index: number,
  prepared: PreparedCall,
  ctx: MatchContext,
): boolean => call('isCarvedOut', rules, index, prepared, ctx);
