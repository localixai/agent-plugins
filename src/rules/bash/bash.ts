import { coreModule } from '../../wasm/bridge.ts';
import type { SpecifierMatcher } from '../matchers.ts';

/**
 * `Bash(...)` and `Monitor(...)` rules with a command pattern: each form a subcommand takes once
 * wrappers are stripped, as written and with its command name unquoted, and for a deny or ask rule
 * also its words as the shell passes them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/bash.rs`, docs/RUST.md).
 */
const call = coreModule('rules/bash/bash');

/** `Bash(...)` and `Monitor(...)` rules with a command pattern (tools-reference.md, "Rule format"). */
export const matchBashRule: SpecifierMatcher = (rule, prepared, ctx) =>
  call('matchBashRule', rule, prepared, ctx);
