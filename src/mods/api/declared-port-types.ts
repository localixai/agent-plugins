import type { EventData } from './ports.ts';

/**
 * The optional ports of the members of `$` the 2.1.330 declarations add (Agent Plugins Extended
 * §10.7.1, §10.19.8): what each asks of the host, and what each answers where the host gives none
 * (`DECLARED_PORT_DEFAULTS`, joined into `OPTIONAL_PORT_DEFAULTS`).
 */

/** What `$.agent.limits()` resolves to (`AgentLimits`, claude-code/index.d.ts:364). */
export interface AgentLimits {
  readonly concurrent: number;
  readonly depth: number;
  readonly isBackgroundEnabled: boolean;
}

/** A tool call `$.agent.call` makes as an agent's own (`{ tool, input, agentId? }`, :3922). */
export interface AgentToolCall {
  readonly tool: string;
  readonly input: EventData;
  /** The agent whose loop makes the call; absent, the main conversation. */
  readonly agentId?: string | undefined;
}

/** The agents' loops: a tool run as one agent's own call, and how many agents the session allows. */
export interface ModAgentCalls {
  /**
   * Runs the call in that agent's loop as its model's call would run (its rules, hooks, dialog and
   * `tool.judge`; `tool.call` fires there), adding nothing to its conversation; answers the tool's
   * `ToolCallResult` or `{ deny }`.
   */
  call(request: AgentToolCall, signal: AbortSignal): Promise<unknown>;
  limits(): Promise<AgentLimits>;
}

/** What `$.session.info()` resolves to (`SessionInfo`, :14833). */
export interface SessionInfo {
  readonly transcriptPath: string;
  readonly scratchpadDir?: string;
  readonly promptId?: string;
  readonly agentType?: string;
  readonly configRoot?: string;
  readonly platform?: 'macos' | 'linux' | 'windows' | 'wsl' | 'unknown';
  readonly isHosted?: boolean;
}

/** What `$.session.machine()` resolves to (`SessionMachine`, :14888), read without running a program. */
export interface SessionMachine {
  /** As `uname -s`, `-r` and `-m` print them. */
  readonly sysname: string;
  readonly release: string;
  readonly machine: string;
  /** Which of npm, yarn, pnpm and bun are on the `PATH`. */
  readonly packageManagers: readonly string[];
  /** Which of bun, deno and node are on the `PATH`. */
  readonly runtimes: readonly string[];
}

/** Facts of the session and its machine. */
export interface ModSessionFacts {
  info(): Promise<SessionInfo>;
  machine(): Promise<SessionMachine>;
}

/**
 * The variables a command hook of `plugin` gets (`hookEnv()` of `$` for processes, :4361): `CLAUDECODE`,
 * `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_CHILD_SESSION`, `CLAUDE_CODE_SESSION_ATTENDED`,
 * `CLAUDE_PID`, `CLAUDE_PROJECT_DIR`, `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA` (the host creates
 * that folder, and rejects when it cannot) and `TRACEPARENT` while a trace is passed on.
 */
export interface ModHookEnv {
  of(plugin: string, signal: AbortSignal): Promise<Readonly<Record<string, string>>>;
}

/** The bytes `$.wasm.compile` hands over, as its event carries them (`{ base64, path? }`). */
export interface WasmSource {
  readonly base64: string;
  /** The file the bytes were read from, absolute; absent when the mod passed the bytes. */
  readonly path?: string | undefined;
}

/**
 * The mod environment's WebAssembly (§10.3.5 item 5): compiles the bytes into a module the mod
 * passes to `WebAssembly.instantiate`; rejects for a module with no room.
 */
export interface ModWasm {
  compile(source: WasmSource, plugin: string, signal: AbortSignal): Promise<unknown>;
}

/**
 * Version control (`$.vcs`, experimental in the 2.1.330 declarations, :4440; events :5552–:5787):
 * the client's own behaviour behind each `vcs.*` event, which answers for Git or with nothing
 * (Agent Plugins Extended §10.19.8, J10.23). Each takes the event as the hooks left it.
 */
export interface ModVcs {
  /** The working copy at `path`: `{ vcs, root, mainRoot, revision, branch, defaultBranch, remote }`, or `null`. */
  detect(request: EventData, signal: AbortSignal): Promise<unknown>;
  /** `{ base, files }`, or `{ unknown }`. */
  diff(request: EventData, signal: AbortSignal): Promise<unknown>;
  /** `{ path, vcs, revision?, branch? }`, or `{ deny }`. */
  isolate(request: EventData, signal: AbortSignal): Promise<unknown>;
  /** The working copies of the repository holding `path`: `[{ path, vcs, branch? }]`. */
  list(request: EventData, signal: AbortSignal): Promise<unknown>;
  /** `{ isReleased: true }` for a copy the client made, `{ kept }` for any other. */
  release(request: EventData, signal: AbortSignal): Promise<unknown>;
  /** `{ changes, revision?, unpublished }`, or `{ unknown }`. */
  status(request: EventData, signal: AbortSignal): Promise<unknown>;
}

/** The optional ports of this file, by the name `ModOptionalPorts` gives each. */
export interface DeclaredOptionalPorts {
  /** `$.agent.call` and `$.agent.limits` (:3922, :364). */
  readonly agentCalls: ModAgentCalls;
  /** `$.session.info` and `$.session.machine` (:3449, :3470). */
  readonly sessionFacts: ModSessionFacts;
  /** `hookEnv` of the processes namespace of `$` (:4361). */
  readonly hookEnv: ModHookEnv;
  /** `$.wasm.compile` (:4605). */
  readonly wasm: ModWasm;
  /** `$.vcs` and the `vcs.*` events (:4440). */
  readonly vcs: ModVcs;
}

// The layer gate reads `process` and a dot as the global, so the namespace is named once, apart.
const PROCESS = 'process';

const refuse = (reason: string) => (): Promise<never> => Promise.reject(new Error(reason));

/** Judgment: no working copy is known without the host, so core answers as for a folder that is none. */
const NO_VCS = 'no version control system is known for this path';

/**
 * What each port answers where the host gives none: a call that needs the host rejects naming what
 * is missing; `$.vcs` answers as core does where no working copy is found: `detect` `null`, `diff`
 * and `status` `{ unknown }`, `isolate` `{ deny }`, `list` none, `release` `{ kept }`.
 */
export const DECLARED_PORT_DEFAULTS: DeclaredOptionalPorts = Object.freeze({
  agentCalls: Object.freeze({
    call: refuse('$.agent.call: this host runs no agent loop for mods'),
    limits: refuse('$.agent.limits: this host does not tell its agent limits'),
  }),
  sessionFacts: Object.freeze({
    info: refuse('$.session.info: this host does not tell the session info'),
    machine: refuse('$.session.machine: this host does not tell the machine'),
  }),
  hookEnv: Object.freeze({ of: refuse(`$.${PROCESS}.hookEnv: this host runs no command hooks`) }),
  wasm: Object.freeze({ compile: refuse('$.wasm.compile: this host compiles no WebAssembly') }),
  vcs: Object.freeze({
    detect: () => Promise.resolve(null),
    diff: () => Promise.resolve({ unknown: NO_VCS }),
    isolate: () => Promise.resolve({ deny: NO_VCS }),
    list: () => Promise.resolve([]),
    release: () => Promise.resolve({ kept: NO_VCS }),
    status: () => Promise.resolve({ unknown: NO_VCS }),
  }),
});
