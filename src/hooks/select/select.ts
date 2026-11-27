/**
 * The handlers an event runs, out of a registry.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/select.rs`, docs/RUST.md).
 * The entries are the host's own objects, told apart by identity: the core answers with the
 * places of the selected entries, and this hands back the entries at those places.
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type {
  EventFactsOf,
  HookRegistry,
  HookSession,
  ModelCatalog,
  MonitorTreatment,
  RegisteredHandler,
  Selection,
} from '../contracts.ts';
import type { IfEvaluator } from './if-guard.ts';

const call = coreModule('hooks/select/select');

export interface SelectRequest<E extends HookEvent> {
  readonly event: E;
  readonly facts: EventFactsOf<E>;
  readonly session: HookSession;
  /** Reads a handler's `if`; without one a handler with `if` runs and a warning says why. */
  readonly ifEvaluator?: IfEvaluator;
  /** Names the canonical model of a model switch; without one every model-switch hook runs. */
  readonly modelCatalog?: ModelCatalog;
}

/** What the core answers: the selected entries by their place in the registry. */
interface Picked {
  readonly selected: readonly number[];
  readonly issues: readonly PluginIssue[];
  readonly monitor: readonly (readonly [number, MonitorTreatment])[];
}

/**
 * The handlers an event runs (hooks.md, "How a hook resolves"): the registered handlers of the
 * event that nothing gates off (`disableAllHooks`, workspace trust, a disabled plugin), with a
 * settings handler defined twice kept once, whose type the event supports, whose matcher selects
 * the occurrence and whose `if` lets them run. Tool calls an event skips select nothing.
 */
export function selectHandlers<E extends HookEvent>(
  registry: Pick<HookRegistry, 'entries'>,
  request: SelectRequest<E>,
): Selection {
  const { entries } = registry;
  const { event, facts, session, ifEvaluator, modelCatalog } = request;
  const picked: Picked = call(
    'selectHandlers',
    entries,
    event,
    facts,
    session,
    ifEvaluator,
    modelCatalog,
  );
  const at = (index: number): readonly RegisteredHandler[] => {
    const entry = entries[index];
    return entry === undefined ? [] : [entry];
  };
  return {
    selected: picked.selected.flatMap(at),
    issues: picked.issues,
    monitor: new Map(
      picked.monitor.flatMap(([index, treatment]) =>
        at(index).map((entry) => [entry, treatment] as const),
      ),
    ),
  };
}
