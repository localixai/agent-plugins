import type { PermissionRule } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * permission-modes.md, "How the classifier evaluates actions": "On entering auto mode, broad allow
 * rules that grant arbitrary code execution are dropped": a blanket `Bash(*)` or `PowerShell(*)`,
 * wildcarded interpreters like `Bash(python*)`, package-manager run commands, `Agent` allow rules
 * and `Monitor` allow rules. "Narrow rules like `Bash(npm test)` stay in effect", and the dropped
 * rules come back when auto mode is left (they are only left out of this view of the rules).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/auto_dropped.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/decide/auto-dropped');

/**
 * Whether `rule` is an allow rule auto mode drops. With `autoMode.classifyAllShell` "auto mode suspends
 * every shell allow rule" as well (settings-reference.md), `Bash(npm test)` included.
 */
export const isDroppedInAuto = (rule: PermissionRule, classifyAllShell = false): boolean =>
  call('isDroppedInAuto', rule, classifyAllShell);
