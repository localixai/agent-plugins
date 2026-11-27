/**
 * Every string of a JSON value substituted, each with the policy of its path.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/deep.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { FieldPolicy, SubstitutionScope } from './scope.ts';
import type { TokenReference } from './substitute.ts';

const call = coreModule('loading/resolve/deep');

export type ValuePath = readonly (string | number)[];

export interface DeepSubstituted {
  /** The input with every string substituted; callers narrow it (a schema parse), the walk keeps no type. */
  readonly value: unknown;
  readonly issues: readonly PluginIssue[];
  readonly referenced: readonly TokenReference[];
}

/**
 * Substitutes every string of a JSON value (an MCP server's `args` and `env`, an LSP server), each
 * with the policy of its path; numbers, booleans and `null` stay. The host is asked once per
 * variable for the whole value. Issues name the field (`env.TOKEN`, `args.1`), under `base` when
 * given.
 */
export const substituteDeep = (
  value: unknown,
  scope: SubstitutionScope,
  policyFor: (path: ValuePath) => FieldPolicy,
  base = '',
): Promise<DeepSubstituted> => call.async('substituteDeep', value, scope, policyFor, base);
