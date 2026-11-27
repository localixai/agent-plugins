import type {
  Clock,
  CommandRunner,
  Environment,
  HttpClient,
  Locations,
  McpCaller,
  ModelRunner,
  ScratchFiles,
  ShellKind,
} from '../ports/index.ts';
import type { HookEvent } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { HookRegistry, ModelCatalog, RegisteredHandler } from './contracts.ts';
import type { EnvFiles } from './env-file/index.ts';
import type { RunContext } from './handlers/context.ts';
import type { DispatchRequest, HookOutcome } from './outcome.ts';
import type { IfEvaluator } from './select/if-guard.ts';
import type { StopBlockCap } from './verdict/stop-block-cap.ts';
import type { HookObserver, OutputStage } from './verdict/types.ts';

/**
 * The engine over the real stages: select, build, run through the ports, read, merge. One handle
 * of the core per engine, made with the host's ports, so a dispatch is one call; the registry
 * crosses by serial, each registered handler numbered here by identity and sent only when the
 * engine does not hold it from the dispatch before (docs/RUST.md, "The host session").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/engine.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/engine');

/** The ports the engine runs handlers through: each handler type needs its own (docs/API.md, "Ports"). */
export interface HookPorts {
  readonly commands: CommandRunner;
  readonly locations: Locations;
  readonly environment: Environment;
  /** `http` hooks. */
  readonly http?: HttpClient;
  /** `mcp_tool` hooks. */
  readonly mcp?: McpCaller;
  /** `prompt` and `agent` hooks. */
  readonly models?: ModelRunner;
  /** Where a hook's text over the 10,000-character cap is saved; without it such text is cut. */
  readonly scratch?: ScratchFiles;
  readonly clock?: Clock;
  readonly observer?: HookObserver;
  readonly modelCatalog?: ModelCatalog;
}

/** What the engine cannot read off a port: the machine, and the output stage. */
export interface HookEngineOptions {
  /** Reads the JSON output of each hook and merges them; by default the real one (`createOutputStage`). */
  readonly output?: OutputStage;
  /** Reads a handler's `if`; without one a handler with `if` runs and a warning says why. */
  readonly ifEvaluator?: IfEvaluator;
  readonly windows?: boolean;
  /** What a command hook with no `shell` field runs in: `bash`, or `powershell` on Windows without Git Bash. */
  readonly defaultShell?: ShellKind;
  readonly startDirectory?: string;
  readonly tempDirectory?: string;
  readonly userConfigOf?: RunContext['userConfigOf'];
  readonly httpPolicy?: RunContext['httpPolicy'];
  /** The host process's id, exported as `CLAUDE_PID` to command hooks. */
  readonly processId?: number;
  /** The HIPAA configuration applies to the session (hipaa-setup.md): hooks get no Anthropic credentials. */
  readonly hipaa?: boolean;
  /** The `env` blocks of the settings files, resolved (`resolveSettingsEnv`). */
  readonly settingsEnv?: RunContext['settingsEnv'];
  /** `CLAUDE_ENV_FILE` for the events that have one (`createEnvFiles`). */
  readonly envFiles?: EnvFiles;
  /**
   * The counter of consecutive Stop blocks (`createStopBlockCap`). By default each engine has its
   * own, which is gone with it; a host that builds an engine per run passes one that outlives the
   * runs, owns its lifetime and calls its `reset` when the session ends.
   */
  readonly stopCap?: StopBlockCap;
  /**
   * Plugin command hooks first check that their plugin's directory exists, so a plugin that is gone
   * from disk fails with the documented notice. On by default; a host that never re-reads its
   * plugins mid-session and wants no lookup per hook run sets `false`.
   */
  readonly checkPluginDirectories?: boolean;
}

export interface HookEngine {
  dispatch<E extends HookEvent>(
    registry: HookRegistry,
    request: DispatchRequest<E>,
  ): Promise<HookOutcome<E>>;
}

/** A registry as the core reads it: its serials in order, and the entries the engine lacks. */
export interface SentRegistry {
  readonly serials: readonly number[];
  readonly fresh: readonly (readonly [number, RegisteredHandler])[];
}

/** The core's engine, which a session's dispatch is given as itself (`dispatchIn`). */
export interface CoreEngine {
  dispatch<E extends HookEvent>(
    registry: SentRegistry,
    request: DispatchRequest<E>,
  ): Promise<HookOutcome<E>>;
}

/** The engine of the core; its two functions cross on their own, so the options stay data. */
export function coreEngine(ports: HookPorts, options: HookEngineOptions = {}): CoreEngine {
  const { userConfigOf, ifEvaluator, ...data } = options;
  return call('createHookEngine', ports, data, userConfigOf, ifEvaluator);
}

const SERIALS = new WeakMap<RegisteredHandler, number>();
let lastSerial = 0;

/** The serial `entry` crosses under: one per registered handler object, never reused. */
function serialOf(entry: RegisteredHandler): number {
  const known = SERIALS.get(entry);
  if (known !== undefined) return known;
  lastSerial += 1;
  SERIALS.set(entry, lastSerial);
  return lastSerial;
}

/** The engine over the real stages: select, build, run through the ports, read, merge. */
export function createHookEngine(ports: HookPorts, options: HookEngineOptions = {}): HookEngine {
  const core = coreEngine(ports, options);
  let held = new Set<number>();
  return {
    dispatch(registry, request) {
      const serials = registry.entries.map(serialOf);
      const fresh = registry.entries.flatMap((entry, index) => {
        const serial = serials[index] ?? serialOf(entry);
        return held.has(serial) ? [] : [[serial, entry] as const];
      });
      held = new Set(serials);
      return core.dispatch({ serials, fresh }, request);
    },
  };
}
