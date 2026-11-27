/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/text_pattern.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PermissionRule } from '../types.ts';

const call = coreModule('rules/bash/text-pattern');

/** The pattern a rule matches command text with, or `undefined` for a bare or non-text rule. */
export const textPatternOf = (rule: PermissionRule): string | undefined =>
  call('textPatternOf', rule);
