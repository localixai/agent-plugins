/**
 * What moves into a background session and what an agent view applies (agent-view.md, "What
 * carries over when you background", "Settings, plugins, and MCP servers"). Pure over the flags of
 * the launch and the work the host says is running: starting the background session, stopping the
 * work and showing the `Background this session?` dialog are the host's. `--fallback-model` and
 * `--allow-dangerously-skip-permissions` also carry over; they are not flags the kit plans, so the
 * host carries them itself.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/background_session.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { GivenFlags } from './flags/types.ts';

const call = coreModule('cli/background-session');

/** The flags of the launch the background session starts with, so a plugin loaded by flag stays loaded. */
export const backgroundSessionFlags = (given: GivenFlags): GivenFlags =>
  call('backgroundSessionFlags', given);

export interface AgentViewFlags {
  /** What agent view itself loads with. */
  readonly own: GivenFlags;
  /** What every session dispatched from it gets. */
  readonly dispatched: GivenFlags;
}

export const agentViewFlags = (given: GivenFlags): AgentViewFlags => call('agentViewFlags', given);

/** The kinds of in-flight work the page names. */
export type BackgroundWorkKind =
  | 'shell'
  | 'subagent'
  | 'loop'
  | 'artifact-replies'
  | 'workflow'
  | 'monitor';

/** "Carries over", "continues only when Claude relaunches it", or "stops". */
export type BackgroundWorkFate = 'carries' | 'relaunch' | 'stops';

export interface RunningWork {
  readonly id: string;
  readonly kind: BackgroundWorkKind;
  /** The subagent that started it, by id. */
  readonly startedBy?: string;
}

export interface BackgroundingPlan {
  readonly fates: Readonly<Record<string, BackgroundWorkFate>>;
  /** Some work stops: the user confirms the `Background this session?` dialog first. */
  readonly confirm: boolean;
}

/**
 * "A subagent moves together with everything it started, so it carries over only when all of that
 * work can move too": a subagent stops when anything it started, at any depth, stops.
 */
export const planBackgrounding = (work: readonly RunningWork[]): BackgroundingPlan =>
  call('planBackgrounding', work);
