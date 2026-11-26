import { coreModule } from '../../wasm/bridge.ts';

/**
 * Bash rule patterns (permissions.md, "Wildcard patterns").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/wildcard.rs`, docs/RUST.md).
 */
const call = coreModule('rules/bash/wildcard');

/** `ls:*` is `ls *`: "an equivalent way to write a trailing wildcard", recognized only at the end. */
export const normalizePattern = (pattern: string): string => call('normalizePattern', pattern);

/** Whether a Bash pattern holds a `*`, `:*` suffix included: an exact rule holds none. */
export const hasWildcard = (pattern: string): boolean => call('hasWildcard', pattern);

export interface PatternOptions {
  /** PowerShell matches without regard to case. */
  readonly ignoreCase?: boolean;
}

/**
 * Whether a command matches a Bash rule pattern (permissions.md, "Wildcard patterns"):
 * - a `*` stands for any text, spaces included, and everything else is literal;
 * - a `*` at the end with a space before it, when it is the only wildcard, also matches the bare
 *   command: `ls *` matches `ls`, `* --help *` does not match `npm --help`;
 * - the space before a trailing `*` is part of the rule: `ls *` does not match `lsof`.
 */
export const matchBashPattern = (
  pattern: string,
  subcommand: string,
  options: PatternOptions = {},
): boolean => call('matchBashPattern', pattern, subcommand, options);
