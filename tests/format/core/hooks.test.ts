/**
 * A core package's `hooks.json` (Agent Plugins 1.1.0, `spec/1.1.0.md`): core §7.3.1 (closed outer
 * object, `$schema`, the five events, closed handlers, the command token, handler `extensions`),
 * §7.3.2 (failure isolation: the file, then an event member, then one handler), §10.1 (the version
 * matches `plugin.json`). Running handlers is a later wave (Agent Plugins Extended §18.1).
 * Judgment-style tests outside `spec(...)` (docs/SPEC.md, "How the conventions map").
 */
import { describe, expect, test } from 'bun:test';
import { CORE_HOOK_EVENTS, readCoreHooks } from '../../../src/entry/core.ts';

const SCHEMA = 'https://agent-plugins.org/schemas/1.1.0/hooks.schema.json';
const hooks = (members: Record<string, unknown>) =>
  readCoreHooks({ $schema: SCHEMA, hooks: members }, '1.1.0');

describe('core §7.3.1: the configuration', () => {
  test('the five events of the specification', () => {
    expect(CORE_HOOK_EVENTS).toEqual([
      'SessionStart',
      'UserPromptSubmit',
      'PreToolUse',
      'PostToolUse',
      'Stop',
    ]);
  });

  test('the example of the specification loads every handler', () => {
    const result = hooks({
      SessionStart: [{ command: './hooks/load-session-context' }],
      UserPromptSubmit: [{ command: './hooks/check-request' }],
      PreToolUse: [
        {
          command: './hooks/check-tool-policy',
          args: ['--policy', '${PLUGIN_ROOT}/config/tool-policy.json'],
        },
      ],
      PostToolUse: [{ command: './hooks/review-tool-result' }],
      Stop: [{ command: './hooks/check-completion' }],
    });
    expect(result.issues).toEqual([]);
    expect(Object.keys(result.hooks)).toEqual([...CORE_HOOK_EVENTS]);
    expect(result.hooks.PreToolUse?.[0]?.args).toEqual([
      '--policy',
      '${PLUGIN_ROOT}/config/tool-policy.json',
    ]);
  });

  test('an empty hooks object is valid', () => {
    expect(hooks({})).toEqual({ hooks: {}, positions: {}, disabled: false, issues: [] });
  });
});

describe('core §7.3.2 item 2: the outer object disables the hooks component', () => {
  test('another top-level field, a missing hooks member or a non-object disables hooks', () => {
    expect(readCoreHooks({ $schema: SCHEMA, hooks: {}, x: 1 }, '1.1.0').disabled).toBe(true);
    expect(readCoreHooks({ $schema: SCHEMA }, '1.1.0').disabled).toBe(true);
    expect(readCoreHooks({ hooks: {} }, '1.1.0').disabled).toBe(true);
    expect(readCoreHooks(null, '1.1.0').disabled).toBe(true);
  });

  test('core §10.1: an unsupported version disables hooks', () => {
    const result = readCoreHooks(
      { $schema: 'https://agent-plugins.org/schemas/2.0.0/hooks.schema.json', hooks: {} },
      '1.1.0',
    );
    expect(result.disabled).toBe(true);
    expect(result.issues[0]?.path).toBe('hooks.json');
  });
});

describe('core §7.3.2 item 3: an event member fails alone', () => {
  test('an event the core does not define is skipped and reported, the others load', () => {
    const result = hooks({
      PreCompact: [{ command: 'node' }],
      Stop: [{ command: 'node' }],
    });
    expect(Object.keys(result.hooks)).toEqual(['Stop']);
    expect(result.issues).toEqual([
      {
        severity: 'error',
        path: 'hooks.PreCompact',
        message: 'Event "PreCompact" is skipped: the core defines no such event',
      },
    ]);
  });

  test('a member that is not a nonempty array is skipped', () => {
    const result = hooks({ Stop: [], SessionStart: { command: 'node' } });
    expect(result.hooks).toEqual({});
    expect(result.issues.map((one) => one.path)).toEqual(['hooks.Stop', 'hooks.SessionStart']);
  });
});

describe('core §7.3.2 item 4: a handler fails alone', () => {
  test('an invalid handler is skipped and reported, its siblings load', () => {
    const result = hooks({
      Stop: [{ type: 'command', command: 'node' }, { command: 'node' }, { command: 'npx tsx x' }],
    });
    expect(result.hooks.Stop).toEqual([{ command: 'node' }]);
    expect(result.positions.Stop).toEqual([1]);
    expect(result.issues.map((one) => [one.path, one.message])).toEqual([
      ['hooks.Stop[0]', 'Handler is skipped: unknown field "type"'],
      [
        'hooks.Stop[2]',
        'Handler is skipped: "command" must be one executable token, not a shell command',
      ],
    ]);
  });

  test('command is required and args are strings', () => {
    expect(hooks({ Stop: [{ args: [] }] }).hooks).toEqual({});
    expect(hooks({ Stop: [{ command: 'node', args: [1] }] }).hooks).toEqual({});
  });

  test('core §8.1: a non-object extensions skips the handler; unimplemented namespaces are ignored', () => {
    expect(hooks({ Stop: [{ command: 'node', extensions: 'x' }] }).hooks).toEqual({});
    expect(hooks({ Stop: [{ command: 'node', extensions: { 'a.b': 1 } }] }).hooks).toEqual({});
    const implemented = readCoreHooks(
      {
        $schema: SCHEMA,
        hooks: { Stop: [{ command: 'node', extensions: { 'a.b': { x: 1 }, 'c.d': { y: 2 } } }] },
      },
      '1.1.0',
      ['a.b'],
    );
    expect(implemented.hooks.Stop).toEqual([{ command: 'node', extensions: { 'a.b': { x: 1 } } }]);
  });
});
