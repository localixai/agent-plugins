import type { HookEvent } from '../../protocol/official.ts';
import type { HookOutcome } from '../outcome.ts';
import { isAbsolute, resolveRelative } from './worktree-path.ts';

/**
 * `watchPaths` of `SessionStart`, `CwdChanged` and `FileChanged` asks for absolute paths (hooks.md,
 * "CwdChanged decision control"); a relative entry is resolved against the hook input's `cwd`, `.`
 * and `..` collapsed, and each path is kept once (Agent Plugins Extended §7.5.28, Judgment J7.131).
 * `CwdChanged`'s `cwd` is the new directory, so its paths are read where the session now is.
 */
export function withResolvedWatchPaths<E extends HookEvent>(
  outcome: HookOutcome<E>,
  cwd: string,
): HookOutcome<E> {
  const given = outcome.effects.watchPaths;
  if (given === undefined || given.every(isAbsolute)) return outcome;
  const watchPaths = [
    ...new Set(given.map((path) => (isAbsolute(path) ? path : resolveRelative(cwd, path)))),
  ];
  return { ...outcome, effects: { ...outcome.effects, watchPaths } };
}
