import type { FlagName, GivenFlags } from './flags/types.ts';

/**
 * What moves into a background session and what an agent view applies (agent-view.md, "What
 * carries over when you background", "Settings, plugins, and MCP servers"). Pure over the flags of
 * the launch and the work the host says is running: starting the background session, stopping the
 * work and showing the `Background this session?` dialog are the host's. `--fallback-model` and
 * `--allow-dangerously-skip-permissions` also carry over; they are not flags the kit plans, so the
 * host carries them itself.
 */

/** "Configuration flags from the original launch carry through to the backgrounded session". */
const CARRIED: readonly FlagName[] = [
  '--mcp-config',
  '--strict-mcp-config',
  '--settings',
  '--setting-sources',
  '--add-dir',
  '--plugin-dir',
];

/** "Agent view applies `--settings`, `--setting-sources`, and `--plugin-dir` to itself". */
const AGENT_VIEW_OWN: readonly FlagName[] = ['--settings', '--setting-sources', '--plugin-dir'];

/** The flags of agent view's table, every one passed through to the sessions it dispatches. */
const AGENT_VIEW_PASSED = CARRIED;

function pick(given: GivenFlags, names: readonly FlagName[]): GivenFlags {
  const kept: Record<string, unknown> = {};
  for (const name of names) if (given[name] !== undefined) kept[name] = given[name];
  // The keys copied are flag names and their values are the ones `given` holds for them.
  return kept as GivenFlags;
}

/** The flags of the launch the background session starts with, so a plugin loaded by flag stays loaded. */
export const backgroundSessionFlags = (given: GivenFlags): GivenFlags => pick(given, CARRIED);

export interface AgentViewFlags {
  /** What agent view itself loads with. */
  readonly own: GivenFlags;
  /** What every session dispatched from it gets. */
  readonly dispatched: GivenFlags;
}

export const agentViewFlags = (given: GivenFlags): AgentViewFlags => ({
  own: pick(given, AGENT_VIEW_OWN),
  dispatched: pick(given, AGENT_VIEW_PASSED),
});

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

const FATES: { readonly [K in BackgroundWorkKind]: BackgroundWorkFate } = {
  shell: 'carries',
  subagent: 'carries',
  loop: 'carries',
  'artifact-replies': 'carries',
  workflow: 'relaunch',
  monitor: 'stops',
};

const STOPS: { readonly [F in BackgroundWorkFate]: boolean } = {
  carries: false,
  relaunch: false,
  stops: true,
};

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
export function planBackgrounding(work: readonly RunningWork[]): BackgroundingPlan {
  const fates: Record<string, BackgroundWorkFate> = {};
  for (const item of work) fates[item.id] = FATES[item.kind];
  let changed = true;
  while (changed) {
    changed = false;
    for (const item of work) {
      const owner = item.startedBy;
      const ownerFate = owner === undefined ? undefined : fates[owner];
      if (owner === undefined || ownerFate === undefined) continue;
      if (STOPS[fates[item.id] ?? 'carries'] && !STOPS[ownerFate]) {
        fates[owner] = 'stops';
        changed = true;
      }
    }
  }
  return { fates, confirm: Object.values(fates).some((fate) => STOPS[fate]) };
}
