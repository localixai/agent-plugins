/**
 * One run of a policy helper and what it comes to (settings-reference.md, `policyHelper`, "Write
 * the helper output", "Helper failures"). The process is the shared `CommandRunner`; the file check
 * is this module's `HelperFileProbe`; time is the `Clock`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/helper_run.rs`,
 * docs/RUST.md).
 */
import type { Settings } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HelperHost } from './ports.ts';
import type { PathFlavor, PolicyHelper } from './policy-helper.ts';

const call = coreModule('settings/managed/helper-run');

export const HELPER_PHASES: readonly ['startup', 'fetchRemoved', 'refresh'] = call('HELPER_PHASES');
export type HelperPhase = (typeof HELPER_PHASES)[number];

/** What a failed run costs in each phase. */
export interface PhaseOutcome {
  /**
   * "When the startup run fails, Claude Code prints the reason and refuses to start"; a run that
   * starts when "a later settings fetch reports the server-managed settings removed" "ends the
   * session with the same message as a failed startup run".
   */
  readonly stopsClaudeCode: boolean;
  /** "When a background refresh fails, Claude Code keeps the last successful policy in effect." */
  readonly keepsLastPolicy: boolean;
  /** "`/status` shows the failing refresh with its reason until a refresh succeeds." */
  readonly shownInStatus: boolean;
}

export const PHASE_OUTCOMES: { readonly [P in HelperPhase]: PhaseOutcome } = call('PHASE_OUTCOMES');

export type HelperRun =
  | {
      readonly ok: true;
      /** The emitted `managedSettings`; `undefined` when the envelope omits it (the helper contributes nothing). */
      readonly settings: Settings | undefined;
      /** What the helper wrote to stderr: "With `--debug`, Claude Code writes the helper's stderr from every run to the debug log." */
      readonly stderr: string;
    }
  | {
      readonly ok: false;
      readonly reason: string;
      readonly outcome: PhaseOutcome;
      readonly stderr: string;
    };

export interface HelperRunInput {
  readonly helper: PolicyHelper;
  readonly host: HelperHost;
  readonly flavor: PathFlavor;
  readonly phase: HelperPhase;
  readonly signal: AbortSignal;
}

/**
 * Runs the helper with no arguments and `CLAUDE_CODE_VERSION` set, within `timeoutMs` counted from
 * the file check (the check and the run share one budget).
 */
export const runPolicyHelper = (input: HelperRunInput): Promise<HelperRun> =>
  call.async('runPolicyHelper', input);
