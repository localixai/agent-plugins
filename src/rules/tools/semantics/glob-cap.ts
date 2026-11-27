/**
 * The Glob tool's result cap, its null-byte check and its `.gitignore` switch (tools-reference.md,
 * "Glob tool behavior").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/glob_cap.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import type { Environment } from '../../../ports/environment.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/glob-cap');

/** tools-reference.md, "Glob tool behavior": "capped at 100 files". */
export const GLOB_RESULT_CAP: number = call('GLOB_RESULT_CAP');

export interface GlobMatch {
  readonly path: string;
  /** Modification time, in milliseconds. */
  readonly mtimeMs: number;
}

export interface CappedGlob {
  readonly paths: readonly string[];
  /** "If the cap is hit, Claude sees a truncation flag in the result." */
  readonly truncated: boolean;
}

/**
 * A Glob result as Claude receives it: sorted by modification time and capped at 100 files, with a
 * truncation flag when files were left out (tools-reference.md, "Glob tool behavior"). The docs do
 * not say which end of the order survives the cap: the newest files do, and files of one time keep
 * their order of arrival (spec/judgments.md). Walking the disk and reading times are the host's.
 */
export const capGlobMatches = (matches: readonly GlobMatch[]): CappedGlob =>
  call('capGlobMatches', matches);

/**
 * "A `pattern` or `path` value that contains a null byte returns an error asking Claude to remove
 * it" (tools-reference.md, "Glob tool behavior"): one issue at `tool_input.<field>` per field that
 * holds one. The wording is a judgment (spec/judgments.md); a value that is not a string is left to
 * `parseToolInput`.
 */
export const validateGlobInput = (input: unknown): readonly PluginIssue[] =>
  call('validateGlobInput', input);

/**
 * "Glob doesn't respect `.gitignore` by default ... To make Glob respect `.gitignore`, set
 * `CLAUDE_CODE_GLOB_NO_IGNORE=false`" (tools-reference.md; env-vars.md: "Set to `false`"). Only the
 * text `false` does; any other value, or none, leaves gitignored files in (spec/judgments.md).
 */
export const globRespectsIgnore = (env: Pick<Environment, 'get'>): boolean =>
  call('globRespectsIgnore', env);
