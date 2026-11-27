import type { MatchContext, PermissionRule } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * `Cd(path)` rules (permissions.md, "Cd"): "Path patterns share the `//`, `~/`, and `/` anchors from
 * Read and Edit rules, but matching is anchored to the whole directory path rather than
 * gitignore-style. `*` matches exactly one path segment and `**` matches across segments. A trailing
 * `/**` also matches its named root."
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/path/cd.rs`, docs/RUST.md).
 */
const call = coreModule('rules/path/cd');

/** A directory a `/cd` moves to: where it resolves, and every spelling it was reached by. */
export interface CdTarget {
  readonly resolved: string;
  readonly spellings: readonly string[];
}

/**
 * Whether a `Cd(<pattern>)` rule covers the target: "Deny rules check every spelling of the target,
 * including each symlink hop it resolves through"; an allow rule needs "the resolved target
 * directory" to match. A bare `Cd` rule has no pattern and is the host's to read.
 */
export const matchCdRule = (rule: PermissionRule, target: CdTarget, ctx: MatchContext): boolean =>
  call('matchCdRule', rule, target, ctx);
