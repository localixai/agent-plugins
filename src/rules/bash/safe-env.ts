/**
 * "Certain known-safe environment variables" (permissions.md, "Bash > Wrappers"): the docs name
 * `NODE_ENV`, `LANG` and `NO_COLOR`; the rest of the set, and the values each accepts, are what
 * Claude Code 2.1.322 let through, read-only or past `Bash(echo hi)` (spec/judgments.md). A value
 * a variable or substitution writes is never safe. The same set serves the read-only set and the
 * allow rules, which a host extends with `MatchContext.safeEnvVars` (any value).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/safe_env.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { ShellWord } from './words.ts';

const call = coreModule('rules/bash/safe-env');

/** The variable a word assigns (`LANG=C`), or `undefined` when it is not an assignment. */
export const assignedVariable = (word: ShellWord): string | undefined =>
  call('assignedVariable', word);

/**
 * Whether an assignment word is a safe one: a known variable with a value it accepts, or a variable
 * the host names (`hostSafe`, any value). A value the shell expands is never safe.
 */
export const isSafeAssignment = (
  word: ShellWord,
  hostSafe: ReadonlySet<string> = new Set(),
): boolean => call('isSafeAssignment', word, hostSafe);
