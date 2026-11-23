import type { Clock } from '../../ports/index.ts';
import { ENGINE_ORIGIN } from '../../protocol/mods-events.ts';
import type { HookReport, ModChain } from '../types.ts';
import { ENGINE_TYPE } from './elements/node.ts';
import { faultReason } from './elements/client.ts';
import type { ModInterface } from './ports.ts';
import { createPaneRegistry, type PaneRegistry } from './panes/registry.ts';
import { paneRequest } from './panes/pane-request.ts';
import { createPlacedRows, type PlacedRows } from './placed/rows.ts';
import { createInvalidator, type Invalidator } from './redraw/invalidate.ts';
import { trackStateReads, type StateReads } from './redraw/subscribe.ts';
import type { StateStore } from '../api/state.ts';
import { foldRender, type Drawn, type FoldOptions } from './sites/fold.ts';
import { renderEvent, type RenderRequest } from './sites/registry.ts';
import {
  drawBand,
  type BandDrawn,
  type BandRequest,
  type SettleRender,
} from './sites/band-rows.ts';
import { createElementTables, resolveTables, type ElementTables } from './tables.ts';
import { createToastBoard, type ToastBoard } from './toast.ts';
import { uiApi, type UiParts } from './ui-api.ts';
import type { Deps } from '../api/call.ts';
import type { ModsNamespaceApi } from '../types.ts';

export interface ModsUiOptions {
  readonly chain: ModChain;
  readonly clock: Clock;
  readonly iface: ModInterface;
  /** `$.state`: a write of a value a site read redraws the site. */
  readonly state: StateStore;
  /** Aborts when the host shuts down. */
  readonly signal: AbortSignal;
  readonly report: (report: HookReport) => void;
  readonly debug: (line: string) => void;
}

/** What `ui.fault` carries: where the `Client` failed and the error message (an error with none is `NO_MESSAGE`). */
export interface FaultReport {
  readonly phase: 'load' | 'render' | 'run';
  readonly reason: string;
}

export interface ModsUi extends UiParts {
  readonly panes: PaneRegistry;
  /** The rows of `$.ui.place`: the host draws each at the `Placed` site and says when rows go away. */
  readonly placed: PlacedRows;
  readonly toasts: ToastBoard;
  readonly tables: ElementTables;
  readonly invalidator: Invalidator;
  /** `ModsApiOptions.ui`: builds the `$.ui` of one mod. */
  readonly api: (deps: Deps) => ModsNamespaceApi<'ui'>;
  /** Fires `ui.resolve` for a mod that loaded, once for each app and render site. */
  loadMod(plugin: string): Promise<void>;
  /**
   * Draws the band (2.1.323 or later): one `AbovePrompt` row for each of `mods` (the loaded mods, in
   * the order they run in) that hooks `ui.render`, stacked through `AbovePromptList`.
   */
  drawBand(request: BandRequest, mods: readonly string[]): Promise<BandDrawn>;
  /** Draws one render site: fires `ui.render` and folds what the chain settled on. */
  draw(
    request: RenderRequest,
    options?: FoldOptions,
  ): Promise<Drawn | { readonly problem: string }>;
  /** A `Client` of `plugin` failed: fires `ui.fault` for that mod's hooks only, then draws the sites once more. */
  fault(plugin: string, report: FaultReport): Promise<void>;
  /** A `Client` of `plugin` posted `data` to its mod: fires `ui.message` for that mod's hooks only. */
  message(plugin: string, data: unknown): Promise<unknown>;
  /**
   * The focused control or the scroll position is about to change: fires `ui.focus` or `ui.scroll`
   * with `change`, then `apply` runs with the change as the hooks left it. A hook that answers
   * without calling `next` keeps the change from happening.
   */
  change(
    name: 'ui.focus' | 'ui.scroll',
    change: Readonly<Record<string, unknown>>,
    apply: (change: unknown) => unknown,
  ): Promise<unknown>;
}

/** Fires `ui.render` for one request; the sites a hook read `$.state` at redraw when it changes. */
function settler(options: ModsUiOptions, reads: StateReads): SettleRender {
  const { chain, signal, report } = options;
  return async (request) => {
    const built = renderEvent(request);
    if ('problem' in built) return undefined;
    const key = `${request.site}:${built.event.requestId}:${request.plugin ?? ''}`;
    return reads.track(key, () =>
      chain.dispatch('ui.render', built.event, {
        origin: ENGINE_ORIGIN,
        signal,
        report,
        debug: options.debug,
        // Claude Code's own drawing, with the props the last `next(e)` gave it.
        core: (e) => Promise.resolve({ type: ENGINE_TYPE, ref: e }),
      }),
    );
  };
}

/**
 * The interface part of mods: what `$.ui` needs, and what the host asks of it (draw a site, report
 * a failed `Client`, deliver a posted message). The host passes `api` as `ModsApiOptions.ui` and
 * `uiRules()` to the chain.
 */
export function createModsUi(options: ModsUiOptions): ModsUi {
  const { chain, iface, signal, report } = options;
  const tables = createElementTables();
  const toasts = createToastBoard({
    clock: options.clock,
    renderer: () => iface.renderer(),
    isHeld: () => panes.holdsToasts(),
    debug: options.debug,
  });
  const panes = createPaneRegistry({
    chain,
    signal,
    report,
    onHoldChange: () => toasts.syncHold(),
  });
  const invalidator = createInvalidator({
    clock: options.clock,
    redraw: () => iface.redraw(),
    isTerminalVisible: () => iface.isTerminalVisible(),
  });
  const parts: UiParts = { iface, panes, placed: createPlacedRows(), invalidator, toasts, tables };
  const reads = trackStateReads(options.state, () => invalidator.invalidate());
  const fire = (
    name: 'ui.fault' | 'ui.message',
    plugin: string,
    input: Readonly<Record<string, unknown>>,
  ) =>
    chain.dispatch(name, input, {
      origin: ENGINE_ORIGIN,
      only: plugin,
      signal,
      report,
      core: (e) => Promise.resolve(e),
    });
  const settle = settler(options, reads);
  return {
    ...parts,
    api: (deps) => uiApi(deps, parts),
    drawBand: (request, mods) =>
      drawBand(
        settle,
        request,
        mods.filter((plugin) => chain.hears('ui.render', plugin)),
      ),
    loadMod: (plugin) => resolveTables({ chain, tables, signal, report }, plugin),
    async draw(given, fold) {
      const request = paneRequest(panes, given);
      const built = renderEvent(request);
      if ('problem' in built) return built;
      const settled = await settle(request);
      const drawn = foldRender(request.site, request.surface, settled, fold);
      // troubleshoot.md, "A `ui.render` line says refused": the debug log's line beside the transcript's.
      if ('refused' in drawn) {
        options.debug(
          `ui.render (${request.site}): the chain settled on a tree that does not validate`,
        );
      }
      return drawn;
    },
    async fault(plugin, fault) {
      await fire('ui.fault', plugin, { ...fault, reason: faultReason(fault.reason) });
      iface.redraw();
    },
    message: (plugin, data) => fire('ui.message', plugin, { data }),
    change: (name, change, apply) =>
      chain.dispatch(name, change, {
        origin: ENGINE_ORIGIN,
        signal,
        report,
        core: async (e) => apply(e),
      }),
  };
}
