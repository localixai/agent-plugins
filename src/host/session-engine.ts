import {
  ifEvaluatorFor,
  type DispatchRequest,
  type HookOutcome,
  type HookSourceKind,
} from '../hooks/index.ts';
import { coreEngine, type CoreEngine } from '../hooks/engine.ts';
import type { HookEvent } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import { currentOf, type EngineInputs, type SessionCore } from './session-core.ts';
import { environmentFor, portsOf } from './session-environment.ts';
import type { SessionOptions } from './session-types.ts';

/**
 * The one engine of a session (docs/API.md, "Hooks"): a session's dispatch is the engine's over
 * the session's own registry and `once` claims, which the core holds (`dispatchIn`), and what lasts
 * across its dispatches (the Stop block count, the `once` holds, the environment files) is the
 * session's, so an engine built again keeps it. The engine is built again only when what it is
 * built over changes: the host's state (an update), or the turn's ports, working directory, added
 * folders or trust; its edge compares those host objects by identity.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_engine.rs`, docs/RUST.md).
 */

const call = coreModule('host/session-engine');

const inputsOf = (core: SessionCore, options: SessionOptions): EngineInputs => ({
  state: currentOf(core).state,
  ports: options.ports,
  cwd: options.cwd,
  additionalDirectories: options.additionalDirectories,
  trusted: options.trusted,
});

const sameInputs = (a: EngineInputs, b: EngineInputs): boolean =>
  a.state === b.state &&
  a.ports === b.ports &&
  a.cwd === b.cwd &&
  a.additionalDirectories === b.additionalDirectories &&
  a.trusted === b.trusted;

function built(core: SessionCore, options: SessionOptions): CoreEngine {
  const { state, gates } = currentOf(core);
  const ports = portsOf(state, options);
  const environment = environmentFor(core, options);
  const windows = state.options.windows ?? false;
  return coreEngine(ports, {
    ...state.options.hookExecution,
    windows,
    stopCap: core.stopCap,
    settingsEnv: environment.settings.values,
    ...(environment.files === undefined ? {} : { envFiles: environment.files }),
    ...(state.options.processId === undefined ? {} : { processId: state.options.processId }),
    ...(state.options.hipaa === true ? { hipaa: true } : {}),
    ifEvaluator: ifEvaluatorFor({
      paths: {
        cwd: options.cwd,
        home: ports.locations.homeDir(),
        primaryWorkingDirectory: options.cwd,
        additionalDirectories: options.additionalDirectories ?? [],
        platform: windows ? 'windows' : 'posix',
      },
    }),
    userConfigOf: (id) =>
      state.active.find((plugin) => plugin.id === id)?.userConfig ?? { declared: {}, values: {} },
    httpPolicy: gates.httpPolicy,
  });
}

export function engineFor(core: SessionCore, options: SessionOptions): CoreEngine {
  const inputs = inputsOf(core, options);
  const last = core.engine.get();
  if (last !== undefined && sameInputs(last.inputs, inputs)) return last.engine;
  const engine = built(core, options);
  core.engine.set({ inputs, engine });
  return engine;
}

/** Which of the session's handlers one dispatch reaches, and whether the Stop cap counts it. */
export interface DispatchScope {
  /** Only the handlers of these sources, when given. */
  readonly sourcesOnly: readonly HookSourceKind[] | undefined;
  /**
   * False for a `Stop` that core handlers share: the session counts the combined continuation in
   * `core.stopCap` once (Agent Plugins Extended §18.1.2, J18.9).
   */
  readonly capped: boolean;
}

/** One dispatch of the session's registry through `engine`, within `scope`. */
export const dispatchIn = <E extends HookEvent>(
  engine: CoreEngine,
  core: SessionCore,
  request: DispatchRequest<E>,
  scope: DispatchScope,
): Promise<HookOutcome<E>> => call.async('dispatchIn', engine, core.handle, request, scope);
