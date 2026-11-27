import type { BackgroundDelivery, BackgroundOutcome, SettledDeliveries } from '../hooks/index.ts';
import { coreModule } from '../wasm/bridge.ts';

/** Which run of which session a background delivery came from, and how that run ended. */
export interface BackgroundSource {
  readonly sessionId: string;
  /** The id of the handler that ran (`RegisteredHandler.id`). */
  readonly handlerId: string;
  readonly outcome: BackgroundOutcome;
  /** The exit code of a command that completed: 2 is the `asyncRewake` wake. */
  readonly exitCode?: number;
}

/** Gets what one background hook delivered, as it settles, and where it came from (`session.onBackground`). */
export type BackgroundListener = (delivered: SettledDeliveries, source: BackgroundSource) => void;

/** The queue's own listener: the delivery itself, which `onBackground` names with its session. */
type DeliveryListener = (delivered: SettledDeliveries, delivery: BackgroundDelivery) => void;

/**
 * What the background hooks of a session delivered since the host last asked (docs/API.md, "Host and
 * sessions", "Background hooks"): `add` takes the promises of a `HookOutcome.background`, `take`
 * hands over what settled in the order it settled and forgets it, `listen` reports each delivery as
 * it settles, so a host that is idle need not poll.
 *
 * Judgments (spec/judgments.md): the queue holds every delivery until the host takes it, so a host
 * that never takes them keeps them all; it is the host's to drain. The repeat of an async
 * `SessionStart` hook reaches it emptied (`session-start-repeats.ts`); `HookOutcome.background`
 * read directly hands over every delivery.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/background_queue.rs`, docs/RUST.md):
 * the deliveries are the core's; the host's promises and listeners stay here, at the edge.
 */
export interface BackgroundQueue {
  add(background: readonly Promise<BackgroundDelivery>[]): void;
  take(): SettledDeliveries;
  listen(listener: DeliveryListener): () => void;
}

const call = coreModule('host/background-queue');

/** The core's half: the deliveries kept until taken. */
interface Settled {
  /** Keeps one delivery; what the listeners hear of it, `null` when it carries nothing. */
  settle(delivery: BackgroundDelivery): SettledDeliveries | null;
  take(): SettledDeliveries;
}

export function createBackgroundQueue(): BackgroundQueue {
  const settled: Settled = call('createBackgroundQueue');
  const listeners = new Set<DeliveryListener>();
  const announce = (delivered: SettledDeliveries, delivery: BackgroundDelivery): void => {
    for (const listener of listeners) {
      try {
        listener(delivered, delivery);
      } catch {
        // A listener that throws neither hides the delivery nor stops the others (spec/judgments.md).
      }
    }
  };
  return {
    add(background) {
      for (const promise of background) {
        promise.then(
          (delivery) => {
            const delivered = settled.settle(delivery);
            if (delivered !== null) announce(delivered, delivery);
          },
          // A process the runner lost delivers nothing (spec/judgments.md); the queue stays usable.
          () => undefined,
        );
      }
    },
    take: () => settled.take(),
    listen(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
