import type { Clock } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { RegisteredHandler } from './contracts.ts';
import type { RawRun } from './handlers/context.ts';
import type { RunFacts } from './verdict/types.ts';

/**
 * What the run stage tells the host about each run it finishes (hooks.md, "Debug hooks": the line
 * names the event and the matched value, the command, the plugin, how the run ended and its
 * duration). The facts are the kit's; the line is the host's (Agent Plugins Extended §7.4.12 item 2,
 * Judgment J7.67).
 *
 * Judgment (spec/judgments.md): an HTTP request that got no answer `failed-to-start`, like a program
 * that did not start; an HTTP status, an MCP answer and a model's verdict, valid or not, `finished`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/run_facts.rs`, docs/RUST.md).
 */
const call = coreModule('hooks/run-facts');

/** When the run started, by the engine's clock; without one no duration is given. */
export const startedAt = (clock: Clock | undefined): number | undefined => call('startedAt', clock);

interface RunTiming {
  readonly clock: Clock | undefined;
  readonly started: number | undefined;
  readonly timeoutMs: number | undefined;
  readonly matchedValue: string | undefined;
}

export const runFactsOf = (entry: RegisteredHandler, run: RawRun, timing: RunTiming): RunFacts =>
  call('runFactsOf', entry, run, timing);
