/**
 * A core package's `mcp.json` (Agent Plugins 1.1.0, `spec/1.1.0.md`): core §7.2.1 (closed outer
 * object; `$schema`; three closed server variants; the `command` token; `cwd` forms; `url` and
 * `headers` rules; server `extensions`), §7.2.2 (failure boundaries: the file, then one server, then
 * an unsupported transport), §9.2 (no `PLUGIN_ROOT`/`PLUGIN_DATA` in `env`), §10.1 (the version
 * matches `plugin.json`). Judgment-style tests outside `spec(...)` (docs/SPEC.md, "How the conventions map").
 */
import { describe, expect, test } from 'bun:test';
import { commandProblem, readCoreMcp } from '../../../src/entry/core.ts';

const SCHEMA = 'https://agent-plugins.org/schemas/1.1.0/mcp.schema.json';
const mcp = (servers: Record<string, unknown>, extra: Record<string, unknown> = {}) =>
  readCoreMcp({ $schema: SCHEMA, mcpServers: servers, ...extra }, '1.1.0');
const one = (server: unknown) => mcp({ s: server });
const skipped = (server: unknown): boolean => one(server).servers.s === undefined;

describe('core §7.2.1, §7.2.2 item 2: the outer object disables MCP when it fails', () => {
  test('the example of the specification loads all three servers', () => {
    const result = mcp({
      'local-validator': {
        type: 'stdio',
        command: './bin/validator',
        args: ['--data', '${PLUGIN_DATA}/validator'],
        env: { CONFIG: '${PLUGIN_ROOT}/config.json' },
        cwd: '${PLUGIN_ROOT}',
      },
      'deployment-api': {
        type: 'streamable-http',
        url: 'https://deploy.example.com/mcp',
        headers: { 'X-Tenant': 'public-tenant' },
      },
      'legacy-events': { type: 'sse', url: 'https://legacy.example.com/sse' },
    });
    expect(Object.keys(result.servers)).toEqual([
      'local-validator',
      'deployment-api',
      'legacy-events',
    ]);
    expect(result.issues).toEqual([]);
    expect(result.servers['local-validator']).toEqual({
      type: 'stdio',
      command: './bin/validator',
      args: ['--data', '${PLUGIN_DATA}/validator'],
      env: { CONFIG: '${PLUGIN_ROOT}/config.json' },
      cwd: '${PLUGIN_ROOT}',
    });
  });

  test('an empty mcpServers is valid', () => {
    expect(mcp({})).toEqual({ servers: {}, disabled: false, issues: [] });
  });

  test('another top-level field, a missing mcpServers or a bad $schema disables MCP', () => {
    expect(mcp({}, { servers: {} }).disabled).toBe(true);
    expect(readCoreMcp({ $schema: SCHEMA }, '1.1.0').disabled).toBe(true);
    expect(readCoreMcp({ mcpServers: {} }, '1.1.0').disabled).toBe(true);
    expect(readCoreMcp('nope', '1.1.0').disabled).toBe(true);
  });

  test('core §10.1: an unsupported version disables MCP and says so', () => {
    const result = readCoreMcp(
      { $schema: 'https://agent-plugins.org/schemas/1.0.0/mcp.schema.json', mcpServers: {} },
      '1.1.0',
    );
    expect(result.disabled).toBe(true);
    expect(result.issues[0]?.message).toBe(
      'MCP is disabled: Unsupported Agent Plugins version 1.0.0: this client implements 1.1.0',
    );
  });
});

describe('core §7.2.1, §7.2.2 item 3: one server fails alone', () => {
  test('an invalid server is skipped and reported, the others load', () => {
    const result = mcp({ bad: { type: 'stdio' }, good: { type: 'stdio', command: 'node' } });
    expect(Object.keys(result.servers)).toEqual(['good']);
    expect(result.issues).toEqual([
      {
        severity: 'error',
        path: 'mcpServers.bad',
        message: 'Server "bad" is skipped: "command" is required for "stdio"',
      },
    ]);
  });

  test('type is required and closed', () => {
    expect(skipped({ command: 'node' })).toBe(true);
    expect(skipped({ type: 'http', url: 'https://a.example' })).toBe(true);
    expect(skipped({ type: 'ws', url: 'wss://a.example' })).toBe(true);
  });

  test('an unknown field, or a field of another variant, makes the entry invalid', () => {
    expect(skipped({ type: 'stdio', command: 'node', timeout: 5 })).toBe(true);
    expect(skipped({ type: 'stdio', command: 'node', url: 'https://a.example' })).toBe(true);
    expect(one({ type: 'sse', url: 'https://a.example', command: 'x' }).issues[0]?.message).toBe(
      'Server "s" is skipped: "command" belongs to another transport than "sse"',
    );
  });

  test('args, env and headers are typed', () => {
    expect(skipped({ type: 'stdio', command: 'node', args: ['a', 1] })).toBe(true);
    expect(skipped({ type: 'stdio', command: 'node', env: { A: 1 } })).toBe(true);
    expect(skipped({ type: 'sse', url: 'https://a.example', headers: { A: 1 } })).toBe(true);
  });

  test('core §9.2: env must not set PLUGIN_ROOT or PLUGIN_DATA', () => {
    expect(skipped({ type: 'stdio', command: 'node', env: { PLUGIN_ROOT: '/x' } })).toBe(true);
    expect(skipped({ type: 'stdio', command: 'node', env: { PLUGIN_DATA: '/x' } })).toBe(true);
    expect(skipped({ type: 'stdio', command: 'node', env: { plugin_root: '/x' } })).toBe(false);
  });

  test('core §8.1: a non-object extensions skips the server; unimplemented namespaces are dropped', () => {
    expect(skipped({ type: 'stdio', command: 'node', extensions: [] })).toBe(true);
    expect(skipped({ type: 'stdio', command: 'node', extensions: { 'a.b': 1 } })).toBe(true);
    const kept = one({ type: 'stdio', command: 'node', extensions: { 'a.b': { x: 1 } } });
    expect(kept.servers.s).toEqual({ type: 'stdio', command: 'node' });
  });
});

describe('core §7.2.1: the command is one executable token', () => {
  test('a bare name and a ./ path are valid', () => {
    expect(commandProblem('node')).toBeUndefined();
    expect(commandProblem('./bin/server')).toBeUndefined();
    expect(commandProblem('server.cmd')).toBeUndefined();
  });

  test('a shell string, a path not beginning with ./ and a placeholder are refused', () => {
    expect(commandProblem('npx tsx server.ts')).toBe(
      'must be one executable token, not a shell command',
    );
    expect(commandProblem('bin/server')).toBe(
      'must be a bare executable name or a path that begins with "./"',
    );
    expect(commandProblem('/usr/bin/node')).toBeDefined();
    expect(commandProblem('../bin/server')).toBeDefined();
    expect(commandProblem('./../bin/server')).toBe(
      'is a plugin-relative path that leaves the plugin root',
    );
    expect(commandProblem('${PLUGIN_ROOT}/bin/server')).toBeDefined();
    expect(commandProblem('')).toBe('must be a non-empty string');
    expect(commandProblem('./')).toBe('is a plugin-relative path that names no file');
  });
});

describe('core §7.2.1: cwd forms', () => {
  test('./, ${PLUGIN_ROOT} and ${PLUGIN_DATA} rooted values are valid', () => {
    for (const cwd of [
      './data',
      '${PLUGIN_ROOT}',
      '${PLUGIN_ROOT}/x',
      '${PLUGIN_DATA}',
      '${PLUGIN_DATA}/cache',
    ]) {
      expect(skipped({ type: 'stdio', command: 'node', cwd })).toBe(false);
    }
  });

  test('another form, or a value that climbs out of its base after resolution, is invalid', () => {
    for (const cwd of [
      'data',
      '/tmp',
      '${PLUGIN_ROOT}x',
      '${HOME}',
      './a/../../x',
      '${PLUGIN_DATA}/../x',
    ]) {
      expect(skipped({ type: 'stdio', command: 'node', cwd })).toBe(true);
    }
  });
});

const remote = (url: string, headers?: Record<string, string>) =>
  skipped({ type: 'streamable-http', url, ...(headers && { headers }) });

describe('core §7.2.1: url and headers', () => {
  test('an absolute https URL is valid; http only on localhost or a loopback literal', () => {
    expect(remote('https://mcp.example.com/mcp')).toBe(false);
    expect(remote('http://localhost:3000/mcp')).toBe(false);
    expect(remote('http://127.0.0.1:3000/mcp')).toBe(false);
    expect(remote('http://[::1]:3000/mcp')).toBe(false);
    expect(remote('http://mcp.example.com/mcp')).toBe(true);
    expect(remote('http://localhost.example.com/mcp')).toBe(true);
  });

  test('a relative URL, another scheme, user information or a fragment is invalid', () => {
    expect(remote('/mcp')).toBe(true);
    expect(remote('ftp://mcp.example.com')).toBe(true);
    expect(remote('https://user:secret@mcp.example.com')).toBe(true);
    expect(remote('https://@mcp.example.com')).toBe(true);
    expect(remote('https://mcp.example.com/mcp#x')).toBe(true);
  });

  test('a placeholder in url or headers is literal text, not expanded', () => {
    const result = one({
      type: 'sse',
      url: 'https://a.example/${PLUGIN_ROOT}',
      headers: { A: '${X}' },
    });
    expect(result.servers.s).toEqual({
      type: 'sse',
      url: 'https://a.example/${PLUGIN_ROOT}',
      headers: { A: '${X}' },
    });
  });

  test('headers are valid HTTP fields, and one name twice under different casing is invalid', () => {
    expect(remote('https://a.example', { 'X-Tenant': 'public' })).toBe(false);
    expect(remote('https://a.example', { 'Bad Name': 'v' })).toBe(true);
    expect(remote('https://a.example', { A: 'line\nbreak' })).toBe(true);
    expect(remote('https://a.example', { A: ' padded' })).toBe(true);
    expect(remote('https://a.example', { 'X-A': '1', 'x-a': '2' })).toBe(true);
  });
});

describe('core §7.2.2 item 4: an unsupported transport', () => {
  test('a valid server of a transport the host does not support is skipped with a warning', () => {
    const result = readCoreMcp(
      { $schema: SCHEMA, mcpServers: { old: { type: 'sse', url: 'https://a.example' } } },
      '1.1.0',
      { transports: ['stdio', 'streamable-http'] },
    );
    expect(result.servers).toEqual({});
    expect(result.issues).toEqual([
      {
        severity: 'warning',
        path: 'mcpServers.old',
        message: 'Server "old" is skipped: this client does not support "sse"',
      },
    ]);
  });
});
