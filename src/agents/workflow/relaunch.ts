/**
 * Resume after you leave the session (workflows.md, "Resume after you leave the session"): "the run
 * doesn't continue on its own. The run continues only when Claude relaunches it with a `Workflow` call,
 * which replays the run from its saved results." Pure decisions over facts the host holds: which way
 * the person left, whether the earlier run's saved results can be found.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/workflow/relaunch.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('agents/workflow/relaunch');

export const LEAVE_WAYS: readonly [
  'background',
  'move-to-background-and-exit',
  'exit-and-stop-tasks',
] = call('LEAVE_WAYS');
export type LeaveWay = (typeof LEAVE_WAYS)[number];

/** Where the run can be picked up again. */
export type RelaunchIn = 'background-session' | 'resumed-session';

export interface Leaving {
  /** The run stopped with the session (`Exit and stop tasks`); in the other two it is only not running. */
  readonly stoppedWithSession: boolean;
  /** Where Claude relaunches it: ask Claude in the background session, or resume the session first. */
  readonly relaunchIn: RelaunchIn;
  /** The shell command that brings the session back, when it does not stay in the background. */
  readonly resumeCommand: string | undefined;
}

/**
 * How each way of leaving leaves the run. In none of the three does the run continue on its own: it
 * continues only through a relaunch (`relaunchAfter`), which is where the three differ.
 */
export const LEAVING: { readonly [W in LeaveWay]: Leaving } = call('LEAVING');

/** The error of a relaunch that finds no saved results at all: it never starts the run over on its own. */
export const NOTHING_TO_RESUME: 'nothing to resume' = call('NOTHING_TO_RESUME');

export interface RelaunchFacts {
  /** Claude has an earlier run of this workflow to relaunch (a fresh session has none). */
  readonly earlierRun: boolean;
  /** The saved results of that run were found: under the session's directory in `~/.claude/projects/`, or restored with a cloud session's history. */
  readonly resultsFound: boolean;
  /** The relaunch happens in a background session. */
  readonly inBackgroundSession: boolean;
  /** The run was started from the path of a script file outside `~/.claude/projects/`, such as one in the working directory. */
  readonly scriptOutsideProjects: boolean;
}

export type Relaunch =
  | { readonly relaunch: 'replay' }
  | { readonly relaunch: 'new-run' }
  | { readonly relaunch: 'fail'; readonly error: typeof NOTHING_TO_RESUME };

/**
 * What a relaunch does. With no earlier run, Claude "starts the workflow over" as a new run. With one
 * whose saved results are gone, or a background session relaunching a script that lives outside
 * `~/.claude/projects/`, it fails with `nothing to resume`; otherwise it replays from the saved results.
 */
export const relaunch = (facts: RelaunchFacts): Relaunch => call('relaunch', facts);

/**
 * What Claude's relaunch does after the person left in `way`: in the background session it is a
 * relaunch there (so a script outside `~/.claude/projects/` cannot be found), after `Exit and stop
 * tasks` it happens in the resumed session.
 */
export const relaunchAfter = (
  way: LeaveWay,
  facts: Omit<RelaunchFacts, 'inBackgroundSession'>,
): Relaunch => call('relaunchAfter', way, facts);
