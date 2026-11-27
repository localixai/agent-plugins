/**
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/background.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { BackgroundDelivery } from '../verdict/types.ts';

const call = coreModule('hooks/output/background');

/**
 * What a host takes from the deliveries of background hooks (hooks.md, "How async hooks execute"):
 * the `additionalContext` and `systemMessage` texts "for Claude on the next conversation turn", and
 * the text an `asyncRewake` hook that exited 2 "wakes Claude" with, surfaced apart so the host can
 * open a turn for it.
 */
export interface SettledDeliveries {
  readonly additionalContext: readonly string[];
  readonly systemMessages: readonly string[];
  /** Present when at least one hook woke Claude; several wake texts join with a newline. */
  readonly wake?: string;
}

/** The deliveries as one value, in the order given; pure. */
export const groupDeliveries = (deliveries: readonly BackgroundDelivery[]): SettledDeliveries =>
  call('groupDeliveries', deliveries);

/**
 * Waits for the promises of `HookOutcome.background` and groups what they delivered, in the order of
 * the promises. Judgment (spec/judgments.md): a promise that rejects (the runner lost the process)
 * delivers nothing and does not hide the others; the dispatch already reported the run.
 */
export const settleDeliveries = (
  background: readonly Promise<BackgroundDelivery>[],
): Promise<SettledDeliveries> => call.async('settleDeliveries', background);
