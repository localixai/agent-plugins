import type {
  BackgroundDelivery,
  HookOutcome,
  HookSession,
  HookSource,
  HookSourceKind,
  RegisteredHandler,
  SettledDeliveries,
} from '../hooks/index.ts';
import type { HooksSettings } from '../protocol/hook-config.ts';
import type { HookEvent } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { BackgroundListener } from './background-queue.ts';
import { currentOf, type SessionCore } from './session-core.ts';
import { dropSpent } from './session-registry.ts';

/**
 * The scoped-hook part of a session: `addHooks`, `removeHooks`, `runScoped`'s turn scope and the
 * reads over the registry, each one call of the session's core.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_hook_scope.rs`,
 * docs/RUST.md); the host's listeners and promises stay here.
 */

const call = coreModule('host/session-hook-scope');

/** Removes one registration; it does nothing once the prefix was registered again or removed. */
export interface HookHandle {
  remove(): void;
}

/** How long hooks added with `addHooks` live: the session, or until the `runScoped` turn ends. */
export type HookLifetime = 'session' | 'turn';

/** Where scoped hooks come from: an agent's frontmatter, a skill, or the host's own run. */
export type ScopedHookSource = Omit<HookSource, 'kind'> & {
  readonly kind: Extract<HookSourceKind, 'agent' | 'session' | 'skill'>;
};

export interface AddHooksOptions {
  /** Default `session`. `turn` needs the view `runScoped` gives, which drops the hooks when it ends. */
  readonly lifetime?: HookLifetime;
  /** The full `name@marketplace` id of the plugin a skill or agent hook belongs to. */
  readonly pluginId?: string;
}

/** The turn-lifetime registrations of one `runScoped`, dropped together when it ends. */
export interface TurnScope {
  /** Removes every registration it holds; a view kept past the callback registers nothing more. */
  end(): void;
}

export const createTurnScope = (): TurnScope => call('createTurnScope');

/**
 * What a dispatch leaves in the core: `watchPaths` of a SessionStart, CwdChanged or FileChanged
 * output replaces the dynamic watch list (no other event's output carries the field), the `once`
 * registrations its runs spent go (a background run's when its process ends), and the background
 * promises join the queue `takeBackground` reads, an async `SessionStart` hook's repeat emptied
 * (`source` is the `SessionStart` input's).
 */
export function recordOutcome(
  core: SessionCore,
  outcome: HookOutcome<HookEvent>,
  source?: unknown,
): void {
  dropSpent(core);
  const watched: readonly Promise<BackgroundDelivery>[] = call(
    'recordOutcome',
    core.handle,
    outcome,
    source,
  );
  core.background.add(
    watched.map((delivery) =>
      delivery.finally(() => {
        dropSpent(core);
      }),
    ),
  );
}

export interface HookScope {
  /**
   * Adds the hooks of a run, an agent or a skill under `prefix` (the ids of its entries begin with
   * it; two registrations of one session never share one). Registering a prefix again replaces what
   * it registered. The hooks pass the same gates as any: the source decides them.
   */
  addHooks(
    prefix: string,
    hooks: HooksSettings,
    source: ScopedHookSource,
    options?: AddHooksOptions,
  ): HookHandle;
  /** Removes what `prefix` registered; an unknown prefix does nothing. */
  removeHooks(prefix: string): void;
  /** Whether a handler that passes this turn's gates would run for the event (matchers are not read). */
  hasHooks(event: HookEvent): boolean;
  /**
   * The handlers that would run in this turn, in registry order (Judgment J7.2: the settings files,
   * highest precedence first; then the plugins in load order; then frontmatter and session hooks as
   * they registered): each with its event, matcher, official handler object, id and `source` (kind, label,
   * scope, plugin name), for a read-only `/hooks` view (hooks.md, "The `/hooks` menu"). A handler the
   * gates silence (`disableAllHooks`, trust, a disabled plugin, a managed lock) or of a type its
   * event does not support is not listed; a `once` handler that was consumed is gone.
   */
  hookEntries(): readonly RegisteredHandler[];
  /** The literal names of the FileChanged matchers, then the dynamic `watchPaths`, each once. */
  fileChangedWatchList(): readonly string[];
  /** The background deliveries that settled since the last call (docs/API.md, "Background hooks"). */
  takeBackground(): SettledDeliveries;
  /**
   * Calls `listener` with what each background hook delivers as it settles (its `wake`, when it woke
   * Claude) and the session and run it came from, so an idle host learns of a wake without polling. It does not take the delivery:
   * `takeBackground` still returns it. Returns the function that stops the listening.
   */
  onBackground(listener: BackgroundListener): () => void;
}

/**
 * The scoped-hook part of one turn's view over the session's core. `session` is read at each use,
 * so the gates it carries are the current settings'.
 */
export function createHookScope(
  core: SessionCore,
  session: () => HookSession,
  scope: TurnScope | undefined,
): HookScope {
  return {
    addHooks(prefix, hooks, source, options = {}) {
      currentOf(core);
      return call('addHooks', core.handle, prefix, hooks, source, options, scope);
    },
    removeHooks: (prefix) => {
      currentOf(core);
      call('removeHooks', core.handle, prefix);
    },
    hasHooks: (event) => call('hasHooks', core.handle, session(), event),
    hookEntries: () => call('hookEntries', core.handle, session()),
    fileChangedWatchList: () => call('fileChangedWatchList', core.handle, session()),
    takeBackground: () => core.background.take(),
    onBackground: (listener) =>
      core.background.listen((delivered, delivery) =>
        listener(delivered, {
          sessionId: session().sessionId,
          handlerId: delivery.handlerId,
          outcome: delivery.outcome,
          ...(delivery.exitCode === undefined ? {} : { exitCode: delivery.exitCode }),
        }),
      ),
  };
}
