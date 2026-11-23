import { selectsTelemetry } from '../../chain/wildcard.ts';
import type { PluginFiles } from '../../../ports/index.ts';
import { type ModScan } from '../scan/index.ts';
import { declaredNamespaces, declaredState, readTypes } from '../types-decl.ts';
import { callLabel, hookLabel, type ScannedHook } from '../scan/uses.ts';

/** A sorted list, or `nothing` (claude plugin validate 2.1.321). */
const listed = (items: readonly string[]): string =>
  items.length === 0 ? 'nothing' : items.toSorted().join(', ');

/** A `command.run` hook whose matcher names a command answers that command itself. */
const answersCommand = (hook: ScannedHook): boolean =>
  hook.event === 'command.run' && /(?:^|,)command=/.test(hook.matcher ?? '');

/** The stream a telemetry hook stands on: the `to` its matcher names, else every stream, `anthropic` among them. */
const streamOf = (hook: ScannedHook): string =>
  /(?:^|,)to=([^,]+)/.exec(hook.matcher ?? '')?.[1] ?? 'anthropic';

/** `plugin.key` values of plugins other than `plugin`: listed, not checked (§10.3.2 item 3). */
const othersOf = (values: readonly string[], plugin: string | undefined): string[] =>
  plugin === undefined
    ? []
    : values.filter((value) => value.includes('.') && !value.startsWith(`${plugin}.`));

/** oracle 2.1.325: a withheld namespace in a mod `validate` reads as the user's. */
const USER_TIER_WITHHOLD = ' (in the user tier, the fold refuses this and unloads the module)';

/** `adds to $:` and `withholds from $:` (Agent Plugins Extended §10.4.3), once an `engine.create` hook is in the form. */
function foldLines(written: string, { engine }: ModScan): string[] {
  if (engine === undefined) return [];
  const withholds =
    engine.withholds.length === 0 ? 'nothing' : `${listed(engine.withholds)}${USER_TIER_WITHHOLD}`;
  return [
    `  ❯ ${written} adds to $: ${listed(engine.adds)}`,
    `  ❯ ${written} withholds from $: ${withholds}`,
  ];
}

/**
 * The default matcher keys of the events on another mod's controls and drawings (§10.13.3, "Other
 * mods' drawings and controls"): `claude plugin validate` 2.1.330 follows a hook whose matcher leaves
 * one out with ` (default matcher key: <keys>)` (§10.4.3; oracle 2.1.330). Judgment: `plugin` and
 * `component` for each, as the two quoted lines give them; a wildcard pattern gets no note.
 */
const DEFAULT_KEYS: readonly string[] = ['plugin', 'component'];
const DEFAULT_KEYED: ReadonlySet<string> = new Set([
  'ui.press',
  'ui.input',
  'ui.select',
  'ui.focus',
  'ui.render',
]);

/** The render sites where the client sets `e.plugin`: a mod's `Pane` and its row of the band. */
const MOD_SITES: ReadonlySet<string> = new Set(['Pane', 'AbovePrompt']);

/** A hook's entry on the `hooks:` line, with the default matcher keys its matcher leaves out. */
function hooksEntry(hook: ScannedHook): string {
  const label = hookLabel(hook);
  if (!DEFAULT_KEYED.has(hook.event)) return label;
  const pairs = (hook.matcher ?? '').split(',').map((pair) => pair.split('='));
  const component = pairs.find(([key]) => key === 'component')?.[1];
  // A site the client draws itself carries no `plugin` (§10.13.3 item 1): only a mod's pane and band row do.
  const isClientSite = hook.event === 'ui.render' && component !== undefined;
  if (isClientSite && !MOD_SITES.has(component)) return label;
  const named = pairs.map(([key]) => key);
  const missing = DEFAULT_KEYS.filter((key) => !named.includes(key));
  return missing.length === 0 ? label : `${label} (default matcher key: ${missing.join(', ')})`;
}

const OTHER_STATE =
  'state of other plugins, not checked (run validate in a session with them enabled)';

/**
 * Lines of one scanned module: `❯ ./register.js hooks: ...`, `answers its own command:`, `calls:`
 * (`nothing on $` when it calls none), the `env` lines, `hooks telemetry for:`, the `state` lines
 * (writes, then reads, the other side `nothing`, once the module uses either) and the values of
 * other plugins, then each gating hook. The order, the sorting and the `nothing` words are oracle
 * 2.1.321 (`claude plugin validate`, black box; spec/judgments.md); the line names Claude Code
 * 2.1.325's (Agent Plugins Extended §10.4.3, Judgment J10.2).
 */
export function scanLines(written: string, scan: ModScan, plugin?: string): string[] {
  const lines: string[] = [];
  const add = (label: string, items: readonly string[]): void => {
    if (items.length > 0) lines.push(`  ❯ ${written} ${label}: ${items.join(', ')}`);
  };
  const pair = (noun: string, writes: readonly string[], reads: readonly string[]): void => {
    if (writes.length + reads.length === 0) return;
    lines.push(`  ❯ ${written} ${noun} writes: ${listed(writes)}`);
    lines.push(`  ❯ ${written} ${noun} reads: ${listed(reads)}`);
  };
  add('hooks', scan.hooks.map(hooksEntry));
  add('answers its own command', scan.hooks.filter(answersCommand).map(hookLabel));
  const calls = scan.calls.map(callLabel);
  lines.push(`  ❯ ${written} calls: ${calls.length === 0 ? 'nothing on $' : listed(calls)}`);
  pair('env', scan.envWrites, scan.envReads);
  const streams = scan.hooks.filter((hook) => selectsTelemetry(hook.event)).map(streamOf);
  add('hooks telemetry for', [...new Set(streams)].toSorted());
  pair('state', scan.stateWrites, scan.stateReads);
  const others = othersOf([...new Set([...scan.stateReads, ...scan.stateWrites])], plugin);
  if (others.length > 0) lines.push(`  ❯ ${written} ${OTHER_STATE}: ${listed(others)}`);
  lines.push(...foldLines(written, scan));
  add('surface modules', scan.surfaceModules);
  for (const hook of scan.hooks.filter((found) => found.isGating)) {
    lines.push(
      `  ❯ ${written} gating hook ${hook.hasCatch ? 'with' : 'without'} .catch: ${hook.event}`,
    );
  }
  return lines;
}

/**
 * What the types file declares, as the manifest's notes: `types ./types/index.d.ts declares on $:
 * $.topo` and `... declares state: probe.count` (Claude Code 2.1.325; Agent Plugins Extended §10.4.3).
 */
export async function typesNotes(
  files: PluginFiles,
  typesPath: string | undefined,
  signal?: AbortSignal,
): Promise<string[]> {
  const text = await readTypes(files, typesPath, signal);
  if (text === undefined || typesPath === undefined) return [];
  const label = `types ./${typesPath}`;
  const namespaces = declaredNamespaces(text).map((name) => `$.${name}`);
  const state = [...declaredState(text)];
  const onDollar =
    namespaces.length > 0 ? listed(namespaces) : 'nothing (no EngineInterface member)';
  return [
    `${label} declares on $: ${onDollar}`,
    ...(state.length > 0 ? [`${label} declares state: ${listed(state)}`] : []),
  ];
}
