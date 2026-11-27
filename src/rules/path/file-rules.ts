/**
 * The matcher of the `read-path` and `edit-path` rule formats, with the paths a call is checked at
 * (permissions.md, "Symlinks") and the `!` rules that carve paths out of a rule.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/path/file_rules.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PermissionRule } from '../types.ts';

const call = coreModule('rules/path/file-rules');

/**
 * Whether `rule` covers a call of a tool other than its own: an `Edit` rule reaches Write, a `Read`
 * rule Grep, and so on (`COVERAGE`). Only a text path rule on a named tool is routed.
 */
export const routesToCall = (rule: PermissionRule, toolName: string): boolean =>
  call('routesToCall', rule, toolName);
