/**
 * The members of `$` the 2.1.330 release adds. Agent Plugins Extended reference.md §10.7.1, "Mods
 * API methods" (`$.agent.call`, `$.agent.limits`, `$.session.info`, `$.session.machine`,
 * `$.process.hookEnv`, `$.wasm.compile`, `$.vcs`), api.md §10.35 ("Reach files, processes, and the
 * network", `$.wasm`; "Paths"), events.md §10.19.7 (`mcp.connect` carries `provider`; the client's
 * own connects with `next.origin` `engine`) and §10.19.8 (version control, J10.23). Declarations
 * 2.1.330: `$.agent.call` (claude-code/index.d.ts:3922), `AgentLimits` (:364), `SessionInfo`
 * (:14833), `SessionMachine` (:14888), `$.process.hookEnv` (:4361), `$.wasm.compile` (:4605),
 * `$.vcs` (:4440), `McpConnectInput` (:8740).
 */
import { describe, expect, test } from 'bun:test';
import { ENGINE_ORIGIN } from '../../../src/protocol/mods-events.ts';
import { MODS_API } from '../../../src/protocol/mods-api-names.ts';
import { OPTIONAL_PORT_DEFAULTS } from '../../../src/mods/api/index.ts';
import type { AgentToolCall, WasmSource } from '../../../src/mods/api/index.ts';
import { fn } from '../../support/mods-chain.ts';
import { apiHarness, expectMethods } from '../../support/mods-api.ts';
import { rejection } from '../../support/rejection.ts';
import { spec } from '../../support/spec.ts';

const LIMITS = { concurrent: 4, depth: 2, isBackgroundEnabled: true };

function agentCalls() {
  const calls: { request: AgentToolCall; aborted: boolean }[] = [];
  return {
    calls,
    port: {
      call: (request: AgentToolCall, signal: AbortSignal) => {
        calls.push({ request, aborted: signal.aborted });
        return Promise.resolve({ result: 'read', isReadOnly: true });
      },
      limits: () => Promise.resolve(LIMITS),
    },
  };
}

spec('mods.api.$.agent', () => {
  test('$.agent has register, spawn, call, list and limits', async () => {
    const h = await apiHarness();
    expectMethods(h.api.agent, MODS_API.agent);
  });

  test("call runs a tool as that agent's own call through agent.call and resolves to its result", async () => {
    const { calls, port } = agentCalls();
    const h = await apiHarness({ ports: { agentCalls: port } });
    const seen: unknown[] = [];
    await h.mod('watch', 'prepend', [fn('agent.call', (_$, e, next) => (seen.push(e), next(e)))]);
    const request = { tool: 'Read', input: { file_path: '/work/a.md' }, agentId: 'agent-1' };
    expect(await h.api.agent.call(request)).toEqual({ result: 'read', isReadOnly: true });
    expect(seen).toEqual([request]);
    expect(calls.map((one) => one.request)).toEqual([request]);
  });

  test("call rejects on an agent.call hook's deny, and the tool does not run", async () => {
    const { calls, port } = agentCalls();
    const h = await apiHarness({ ports: { agentCalls: port } });
    await h.mod('guard', 'prepend', [fn('agent.call', () => ({ deny: 'not now' }))]);
    expect(await rejection(h.api.agent.call({ tool: 'Bash', input: { command: 'ls' } }))).toBe(
      'my-mod: $.agent.call: not now',
    );
    expect(calls).toEqual([]);
  });

  test('call refuses Workflow, and takes { signal }', async () => {
    const { calls, port } = agentCalls();
    const h = await apiHarness({ ports: { agentCalls: port } });
    expect(await rejection(h.api.agent.call({ tool: 'Workflow', input: {} }))).toContain(
      'Workflow',
    );
    const stop = new AbortController();
    stop.abort();
    await h.api.agent.call({ tool: 'Read', input: {} }, { signal: stop.signal });
    expect(calls).toEqual([{ request: { tool: 'Read', input: {} }, aborted: true }]);
  });

  test('limits resolves { concurrent, depth, isBackgroundEnabled }', async () => {
    const h = await apiHarness({ ports: { agentCalls: agentCalls().port } });
    expect(await h.api.agent.limits()).toEqual(LIMITS);
  });
});

spec('mods.api.$.session', () => {
  test('info and machine resolve what the host tells, each through its event', async () => {
    const info = { transcriptPath: '/t.jsonl', platform: 'macos' as const, isHosted: false };
    const machine = {
      sysname: 'Darwin',
      release: '25.0.0',
      machine: 'arm64',
      packageManagers: ['npm', 'bun'],
      runtimes: ['node', 'bun'],
    };
    const h = await apiHarness({
      ports: {
        sessionFacts: {
          info: () => Promise.resolve(info),
          machine: () => Promise.resolve(machine),
        },
      },
    });
    const seen: string[] = [];
    await h.mod('watch', 'prepend', [
      fn('session.info', (_$, e, next) => (seen.push('info'), next(e))),
      fn('session.machine', (_$, e, next) => (seen.push('machine'), next(e))),
    ]);
    expect(await h.api.session.info()).toEqual(info);
    expect(await h.api.session.machine()).toEqual(machine);
    expect(seen).toEqual(['info', 'machine']);
  });
});

spec('mods.api.$.process', () => {
  test("hookEnv resolves the variables a command hook of this plugin gets, from the host's hookEnv port", async () => {
    const asked: string[] = [];
    const env = { CLAUDECODE: '1', CLAUDE_PLUGIN_ROOT: '/plugins/my-mod' };
    const h = await apiHarness({
      ports: { hookEnv: { of: (plugin) => (asked.push(plugin), Promise.resolve(env)) } },
    });
    expectMethods(h.api.process, MODS_API.process);
    expect(await h.api.process.hookEnv()).toEqual(env);
    expect(asked).toEqual(['my-mod']);
  });
});

/** A wasm port that records what it was handed and answers a module handle. */
function wasm() {
  const compiled: WasmSource[] = [];
  return {
    compiled,
    port: {
      compile: (source: WasmSource) => (compiled.push(source), Promise.resolve({ module: 1 })),
    },
  };
}

const MAGIC = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);

spec(['mods.api.$.wasm', 'mods.apiGuide.Reach files, processes, and the network.$.wasm'], () => {
  test('compile takes the bytes and hands them on as base64 through the wasm.compile event', async () => {
    const { compiled, port } = wasm();
    const h = await apiHarness({ ports: { wasm: port } });
    const seen: unknown[] = [];
    await h.mod('watch', 'prepend', [fn('wasm.compile', (_$, e, next) => (seen.push(e), next(e)))]);
    expect(await h.api.wasm.compile(MAGIC)).toEqual({ module: 1 });
    expect(await h.api.wasm.compile(MAGIC.buffer)).toEqual({ module: 1 });
    expect(seen).toEqual([{ base64: 'AGFzbQEAAAA=' }, { base64: 'AGFzbQEAAAA=' }]);
    expect(compiled).toEqual([{ base64: 'AGFzbQEAAAA=' }, { base64: 'AGFzbQEAAAA=' }]);
  });

  test("compile of a path reads the file's bytes; a missing file, or one over 64 MiB, rejects", async () => {
    const { compiled, port } = wasm();
    const files: Record<string, { text: string; bytes: number; base64: string }> = {
      '/plugins/my-mod/engine.wasm': { text: '', bytes: 8, base64: 'AGFzbQEAAAA=' },
      '/plugins/my-mod/big.wasm': { text: '', bytes: 64 * 1024 * 1024 + 1, base64: '' },
    };
    const h = await apiHarness({
      ports: {
        wasm: port,
        fs: {
          read: (path: string) => Promise.resolve(files[path]),
          write: () => Promise.reject(new Error('unused')),
          list: () => Promise.reject(new Error('unused')),
          stat: () => Promise.reject(new Error('unused')),
          exists: () => Promise.reject(new Error('unused')),
          ancestors: () => Promise.reject(new Error('unused')),
        },
      },
    });
    await h.api.wasm.compile('engine.wasm');
    expect(compiled).toEqual([{ base64: 'AGFzbQEAAAA=', path: '/plugins/my-mod/engine.wasm' }]);
    expect(await rejection(h.api.wasm.compile('gone.wasm'))).toContain(
      '/plugins/my-mod/gone.wasm: no such file',
    );
    expect(await rejection(h.api.wasm.compile('big.wasm'))).toContain('67108864-byte limit');
    expect(await rejection(h.api.wasm.compile(42))).toStartWith('$.wasm.compile: ');
  });
});

spec('mods.api.$.mcp', () => {
  test("a mod's connect carries provider, { plugin, tier } of whoever configured the server", async () => {
    const h = await apiHarness({
      ports: {
        mcp: {
          ensureConnected: () => Promise.resolve('connected'),
          callTool: () => Promise.reject(new Error('unused')),
        },
      },
      options: { ownMcpServers: () => ['mine'] },
    });
    const seen: unknown[] = [];
    await h.mod('watch', 'prepend', [fn('mcp.connect', (_$, e, next) => (seen.push(e), next(e)))]);
    await h.api.mcp.connect('mine');
    expect(seen).toEqual([{ server: 'mine', provider: { plugin: 'my-mod', tier: 'user' } }]);
  });

  test("the client's own connect fires mcp.connect from engine; a hook that answers without next holds the server back", async () => {
    const h = await apiHarness();
    const origins: unknown[] = [];
    await h.mod('policy', 'prepend', [
      fn('mcp.connect', (_$, _e, next) => {
        origins.push(next.origin.plugin);
        return { deny: 'held by policy' };
      }),
    ]);
    let connected = 0;
    const answer = await h.chain.dispatch(
      'mcp.connect',
      { server: 'docs', provider: { plugin: 'docs-plugin', tier: 'user' } },
      {
        origin: ENGINE_ORIGIN,
        signal: new AbortController().signal,
        report: () => undefined,
        core: () => ((connected += 1), Promise.resolve({ value: { isConnected: true } })),
      },
    );
    expect(origins).toEqual(['engine']);
    expect(connected).toBe(0);
    expect(answer).toEqual({ deny: 'held by policy' });
  });
});

describe('§10.19.8 version control, J10.23 (declarations only: outside spec(...))', () => {
  test('$.vcs has detect, diff, isolate, list, release and status', async () => {
    const h = await apiHarness();
    expectMethods(h.api.vcs, MODS_API.vcs);
  });

  test("each call raises its event and resolves to that event's result; a hook answers in the client's place", async () => {
    const h = await apiHarness({ ports: { vcs: OPTIONAL_PORT_DEFAULTS.vcs } });
    await h.mod('jj', 'prepend', [
      fn('vcs.detect', (_$, e) => ({
        vcs: 'jj',
        root: Reflect.get(Object(e), 'path'),
        mainRoot: null,
        revision: 'kx',
        branch: null,
        defaultBranch: null,
        remote: null,
      })),
    ]);
    expect(await h.api.vcs.detect({ path: '/work' })).toMatchObject({ vcs: 'jj', root: '/work' });
  });

  test('without a host port core answers as where no working copy is found', async () => {
    const h = await apiHarness({ ports: { vcs: OPTIONAL_PORT_DEFAULTS.vcs } });
    expect(await h.api.vcs.detect({ path: '/work' })).toBeNull();
    expect(await h.api.vcs.list({ path: '/work' })).toEqual([]);
    expect(await h.api.vcs.diff({ path: '/w', vcs: 'git', base: 'head' })).toHaveProperty(
      'unknown',
    );
    expect(
      await h.api.vcs.isolate({ for: 'agent', from: '/w', name: 'fix', base: 'head' }),
    ).toHaveProperty('deny');
    expect(await rejection(h.api.vcs.diff({ path: '/w', base: 'tip' }))).toStartWith(
      '$.vcs.diff: ',
    );
  });
});
