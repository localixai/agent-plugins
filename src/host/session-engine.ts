import {
  createHookEngine,
  ifEvaluatorFor,
  type HookEngine,
  type StopBlockCap,
} from '../hooks/index.ts';
import { currentOf, type EngineInputs, type SessionCore } from './session-core.ts';
import { environmentFor, portsOf } from './session-environment.ts';
import type { SessionOptions } from './session-types.ts';

/**
 * The one engine of a session (docs/API.md, "Hooks"): a session's dispatch is `createHookEngine`'s
 * dispatch, and what lasts across its dispatches (the Stop block count, the `once` holds, the
 * environment files) is the core's, so an engine built again keeps it. The engine is built again
 * only when what it is built over changes: the host's state (an update), or the turn's ports,
 * working directory, added folders or trust.
 */

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

/** A cap that counts nothing: the session applies its own to an outcome it combined first. */
const UNCOUNTED: StopBlockCap = { apply: (outcome) => outcome, reset: () => undefined };

function built(core: SessionCore, options: SessionOptions, stopCap = core.stopCap): HookEngine {
  const { state, gates } = currentOf(core);
  const ports = portsOf(state, options);
  const environment = environmentFor(core, options);
  const windows = state.options.windows ?? false;
  return createHookEngine(ports, {
    ...state.options.hookExecution,
    windows,
    stopCap,
    onceClaims: core.onceClaims,
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

export function engineFor(core: SessionCore, options: SessionOptions): HookEngine {
  const inputs = inputsOf(core, options);
  const last = core.engine.get();
  if (last !== undefined && sameInputs(last.inputs, inputs)) return last.engine;
  const engine = built(core, options);
  core.engine.set({ inputs, engine });
  return engine;
}

/**
 * The session's engine without the Stop cap, for a `Stop` that core handlers share: the session
 * counts the combined continuation in `core.stopCap` once (Agent Plugins Extended §18.1.2, J18.9).
 * Built for that dispatch alone; the `once` claims are the session's as always.
 */
export const uncappedEngineFor = (core: SessionCore, options: SessionOptions): HookEngine =>
  built(core, options, UNCOUNTED);
