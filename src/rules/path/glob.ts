/**
 * The gitignore engine of the path rules: "Read and Edit rules both use gitignore pattern syntax"
 * (permissions.md, "How path patterns match"), over a path relative to the rule's anchor. Case:
 * Windows paths are compared without it, POSIX paths with it (a judgment, the docs are silent).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/path/glob.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { Platform } from './posix.ts';

const call = coreModule('rules/path/glob');

/** A pattern, and whether it is a `!` negation of what came before. */
export interface GlobEntry {
  readonly pattern: string;
  readonly negated: boolean;
}

/**
 * Compiled patterns: they cross the boundary as the platform and the lines they were compiled
 * from, and the core keeps the engine it compiled from them (`glob.rs`).
 */
export interface Globs {
  readonly platform: Platform;
  readonly lines: readonly string[];
}

/**
 * Whether gitignore would keep the pattern as a rule at all: a blank line and a `#` comment are not
 * rules. "A deny or ask rule whose path isn't usable as a gitignore pattern still guards that exact
 * path. An allow rule with an unusable pattern doesn't approve anything." (permissions.md)
 */
export const isUsablePattern = (pattern: string): boolean => call('isUsablePattern', pattern);

/** The engine for one platform and pattern list; built once per list. */
export const compileGlobs = (entries: readonly GlobEntry[], platform: Platform): Globs =>
  call('compileGlobs', entries, platform);

/**
 * Whether the compiled patterns select `relative`. `''` and a parent climb (`..`, `../x`) never reach
 * the engine, which throws on them; a name that merely starts with two dots (`..hidden`) is a file.
 * A search tool's `path` is a directory, which gitignore only matches through the files below it
 * (`docs/**` does not match `docs`), so a directory is also tried as the parent of a name no
 * pattern is written for (a judgment, spec/judgments.md).
 */
export const gitignoreMatches = (globs: Globs, relative: string, directory: boolean): boolean =>
  call('gitignoreMatches', globs, relative, directory);
