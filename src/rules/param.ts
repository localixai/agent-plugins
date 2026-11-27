/**
 * Parameter rules (permissions.md, "Match by input parameter").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/param.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PermissionRule, PreparedCall, Specifier } from './types.ts';

const call = coreModule('rules/param');

type ParamSpecifier = Extract<Specifier, { readonly form: 'param' }>;

/**
 * Whether a parameter rule covers a call: the literal input field, a scalar, present, and equal to
 * the rule's value where `*` stands for any sequence of characters, and, for a
 * `Skill(skill:<name>)` deny rule, any name the host reports for the called skill.
 */
export const matchesCallParam = (
  rule: PermissionRule,
  specifier: ParamSpecifier,
  prepared: PreparedCall,
): boolean => call('matchesCallParam', rule, specifier, prepared);
