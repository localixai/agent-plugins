import type { Clock } from '../ports/index.ts';
import type { RegisteredHandler } from './contracts.ts';
import type { RawRun } from './handlers/context.ts';
import { isPluginSource, pluginLabel } from './source-kind.ts';
import { handlerLabel } from './verdict/notices.ts';
import type { RunEnding, RunFacts } from './verdict/types.ts';

/**
 * What the run stage tells the host about each run it finishes (hooks.md, "Debug hooks": the line
 * names the event and the matched value, the command, the plugin, how the run ended and its
 * duration). The facts are the kit's; the line is the host's (Agent Plugins Extended §7.4.12 item 2,
 * Judgment J7.67).
 *
 * Judgment (spec/judgments.md): an HTTP request that got no answer `failed-to-start`, like a program
 * that did not start; an HTTP status, an MCP answer and a model's verdict, valid or not, `finished`.
 */
const ENDINGS: { readonly [K in RawRun['kind']]: RunEnding } = {
  exited: 'finished',
  'timed-out': 'timed-out',
  cancelled: 'cancelled',
  'not-started': 'failed-to-start',
  skipped: 'skipped',
  background: 'background',
  http: 'finished',
  'http-failed': 'failed-to-start',
  mcp: 'finished',
  model: 'finished',
  'model-failed': 'finished',
};

/** Only a run that reached its limit names the limit. */
const GIVES_TIMEOUT: { readonly [E in RunEnding]: boolean } = {
  finished: false,
  'timed-out': true,
  cancelled: false,
  background: false,
  'failed-to-start': false,
  skipped: false,
};

/** When the run started, by the engine's clock; without one no duration is given. */
export const startedAt = (clock: Clock | undefined): number | undefined => clock?.now();

interface RunTiming {
  readonly clock: Clock | undefined;
  readonly started: number | undefined;
  readonly timeoutMs: number | undefined;
  readonly matchedValue: string | undefined;
}

/** How long the run took, when the engine has a clock. */
function durationOf(timing: RunTiming): { durationMs?: number } {
  const { started, clock } = timing;
  const ended = started === undefined ? undefined : clock?.now();
  return ended === undefined || started === undefined ? {} : { durationMs: ended - started };
}

export function runFactsOf(entry: RegisteredHandler, run: RawRun, timing: RunTiming): RunFacts {
  const ending = ENDINGS[run.kind];
  const plugin = isPluginSource(entry.source) ? pluginLabel(entry) : undefined;
  return {
    ...(timing.matchedValue === undefined ? {} : { matchedValue: timing.matchedValue }),
    handler: handlerLabel(entry.handler),
    ...(plugin === undefined ? {} : { plugin }),
    ending,
    ...('exitCode' in run ? { exitCode: run.exitCode } : {}),
    ...(GIVES_TIMEOUT[ending] && timing.timeoutMs !== undefined
      ? { timeoutMs: timing.timeoutMs }
      : {}),
    ...durationOf(timing),
  };
}
