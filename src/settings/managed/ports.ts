import type { Clock } from '../../ports/clock.ts';
import type { CommandRunner } from '../../ports/command-runner.ts';

/**
 * The one effect the managed sources need beyond the shared ports: asking whether a regular file is
 * at a path, within a time budget (settings-reference.md, "Helper failures": "Claude Code checks for
 * the file before starting the helper, within the same `timeoutMs` budget, so an unresponsive
 * network mount can cause the run to fail"). Running the helper itself is the shared `CommandRunner`.
 */
export interface HelperFileProbe {
  /** `regularFile`: a regular file is at `path`. `timedOut`: the answer did not come within `budgetMs`. */
  check(
    path: string,
    budgetMs: number,
    signal: AbortSignal,
  ): Promise<{ readonly regularFile: boolean; readonly timedOut: boolean }>;
}

/** What a host hands to run a helper. */
export interface HelperHost {
  readonly runner: CommandRunner;
  readonly probe: HelperFileProbe;
  readonly clock: Clock;
  /**
   * The environment Claude Code was launched with, before any settings `env` applies: "The
   * helper's environment takes nothing from the `env` of user settings or `--settings`; what it
   * needs comes from the managed `env` or the launching shell" (spec §14.2.5 item 3, 2.1.323).
   * `CLAUDE_CODE_VERSION` is added.
   */
  readonly env: Readonly<Record<string, string>>;
  /** The `env` of the managed settings, whose values replace the shell's for the helper. */
  readonly managedEnv?: Readonly<Record<string, string>>;
  readonly cwd: string;
  /** The Claude Code version the helper is told in `CLAUDE_CODE_VERSION`. */
  readonly version: string;
}
