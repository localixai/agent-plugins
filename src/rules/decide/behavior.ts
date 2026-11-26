import { coreModule } from '../../wasm/bridge.ts';
import type { PermissionRule } from '../types.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/behavior.rs`, docs/RUST.md). */
const call = coreModule('rules/decide/behavior');

/** Whether `rule` is an allow rule. */
export const isAllowRule = (rule: PermissionRule): boolean => call('isAllowRule', rule);
