import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HookOutcome } from '../outcome.ts';

/**
 * `watchPaths` of `SessionStart`, `CwdChanged` and `FileChanged` asks for absolute paths (hooks.md,
 * "CwdChanged decision control"); a relative entry is resolved against the hook input's `cwd`, `.`
 * and `..` collapsed, and each path is kept once (Agent Plugins Extended §7.5.28, Judgment J7.131).
 * `CwdChanged`'s `cwd` is the new directory, so its paths are read where the session now is.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/watch_paths.rs`,
 * docs/RUST.md).
 */
const call = coreModule('hooks/output/watch-paths');

export const withResolvedWatchPaths = <E extends HookEvent>(
  outcome: HookOutcome<E>,
  cwd: string,
): HookOutcome<E> => call('withResolvedWatchPaths', outcome, cwd);
