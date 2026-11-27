/**
 * Pure helpers over command names and host paths, for `resolveFolderSkills`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/scan/names.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('skills/scan/names');

/** The parts of a host path, whichever separator it uses. */
export const segmentsOf = (path: string): readonly string[] => call('segmentsOf', path);

/**
 * How synced skills' names are compared (skills.md, "When a synced skill name matches another
 * command"): "Claude Code ignores case, spacing, and invisible characters, and treats compatibility
 * forms such as fullwidth letters and dash variants as their plain equivalents". A look-alike letter
 * from another alphabet stays different.
 */
export const foldName = (name: string): string => call('foldName', name);

/**
 * The directory `base` as a path below `cwd` (`apps/web`), `/`-separated: how a nested skill's
 * qualified name starts (skills.md, "How a skill gets its command name": "Subdirectory path relative
 * to the working directory"). A base that is not below `cwd` is its folder's name (judgment).
 */
export const relativeBelow = (cwd: string, base: string): string =>
  call('relativeBelow', cwd, base);

/** Whether `path` is inside `parent`, and not `parent` itself. */
export const isStrictlyBelow = (parent: string, path: string): boolean =>
  call('isStrictlyBelow', parent, path);
