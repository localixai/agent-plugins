/**
 * The report lines and messages of `claude plugin validate` that Claude Code 2.1.325 gives and the
 * docs (2.1.321) do not quote: Agent Plugins Extended §10.3.1, §10.3.2, §10.4.2 and §10.4.3,
 * Judgment J10.2, each checked against `claude plugin validate` 2.1.325 run as a black box on a
 * minimal module of our own (spec/judgments.md). Judgments, so outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { readHooksJsonModules } from '../../../src/format/mod-files.ts';
import { validateMod } from '../../../src/mods/load/commands/validate.ts';
import { memoryFiles } from '../../support/ports.ts';

const manifest = (extra: Record<string, unknown> = {}) =>
  JSON.stringify({
    name: 'probe',
    version: '0.1.0',
    description: 'x',
    author: { name: 'x' },
    ...extra,
  });

const TYPES = `export type Topo = { layers: () => Promise<string[]> }
declare module 'claude-code' {
  interface EngineInterface { topo: Topo }
  interface PluginState { 'probe': { count: number } }
}`;

const run = async (module: string, extra: Record<string, string> = {}) => {
  const files = memoryFiles({
    '.claude-plugin/plugin.json': manifest(),
    'hooks/hooks.json': JSON.stringify({ modules: ['./register.js'] }),
    'hooks/register.js': module,
    ...extra,
  });
  const report = await validateMod({ files, target: '.' });
  return { ...report, text: report.lines.join('\n') };
};

const REMINDER =
  '$ is always spelled $.noun.event(...) at the call site, on is always on("<event>", hook), and next.to always next.to(e, "<tier>")';

const inRegister = (body: string) => `export function register(on) {\n${body}\n}`;

describe('the report lines of 2.1.325', () => {
  test('answers its own command, hooks telemetry for, and state of other plugins', async () => {
    const report = await run(
      inRegister(`
  on('command.run', { command: 'tally' }, async ($, e, next) => ({ text: 'x' }))
  on('telemetry.*', { to: 'collector' }, async ($, e, next) => next(e))
  on('session.start', async ($, e, next) => {
    await $.state.get({ plugin: 'other', key: 'x' })
    await $.state.get({ plugin: 'probe', key: 'count' })
    return next(e)
  })`),
      { 'types/index.d.ts': TYPES },
    );
    expect(report.exitCode).toBe(0);
    expect(report.lines).toContain(
      '  ❯ ./register.js answers its own command: command.run{command=tally}',
    );
    expect(report.lines).toContain('  ❯ ./register.js hooks telemetry for: collector');
    expect(report.lines).toContain('  ❯ ./register.js state reads: other.x, probe.count');
    expect(report.lines).toContain(
      '  ❯ ./register.js state of other plugins, not checked (run validate in a session with them enabled): other.x',
    );
  });

  test('what the types file declares is listed under the manifest', async () => {
    const report = await run(inRegister(''), { 'types/index.d.ts': TYPES });
    expect(report.lines).toContain('  ❯ types ./types/index.d.ts declares on $: $.topo');
    expect(report.lines).toContain('  ❯ types ./types/index.d.ts declares state: probe.count');
    expect(report.manifest?.notes).toEqual([
      'types ./types/index.d.ts declares on $: $.topo',
      'types ./types/index.d.ts declares state: probe.count',
    ]);
    const stateOnly = `declare module 'claude-code' { interface PluginState { 'probe': { count: number } } }`;
    const plain = await run(inRegister(''), { 'types/index.d.ts': stateOnly });
    expect(plain.manifest?.notes).toEqual([
      'types ./types/index.d.ts declares on $: nothing (no EngineInterface member)',
      'types ./types/index.d.ts declares state: probe.count',
    ]);
  });

  test('a telemetry event, and a glob or a negation that selects a gating event, is gating', async () => {
    const report = await run(
      inRegister(`
  on('telemetry.log', { to: 'collector' }, async ($, e, next) => next(e))
  on('tool.*', async ($, e, next) => next(e))
  on('!session.*', async ($, e, next) => next(e))
  on('session.*', async ($, e, next) => next(e))`),
    );
    const gating = report.lines.filter((line) => line.includes('gating hook'));
    expect(gating).toEqual([
      '  ❯ ./register.js gating hook without .catch: telemetry.log',
      '  ❯ ./register.js gating hook without .catch: tool.*',
      '  ❯ ./register.js gating hook without .catch: !session.*',
    ]);
  });
});

describe('the messages of 2.1.325', () => {
  test('a second modules entry', () => {
    const read = readHooksJsonModules({ modules: ['./register.js', './b.js'] });
    expect(read.issues.map((issue) => issue.message)).toEqual([
      'hooks.json `modules` names one hooks module per plugin; a second entry is refused',
    ]);
  });

  test('on twice for one event without a matcher; a matcher makes the second one another hook', async () => {
    const report = await run(
      inRegister(`  on('session.start', async ($, e, next) => next(e))
  on('session.start', async ($, e, next) => next(e))
  on('tool.call', { tool: 'Bash' }, async ($, e, next) => next(e))
  on('tool.call', { tool: 'Read' }, async ($, e, next) => next(e))`),
    );
    expect(report.exitCode).toBe(1);
    expect(report.text).toContain(
      'on("session.start") is registered twice without a matcher; the first is at ./register.js:2',
    );
    expect(report.text).not.toContain('on("tool.call") is registered twice');
  });

  test('a hook on a streaming event that is not an async generator', async () => {
    const report = await run(inRegister(`  on('turn.step', async ($, e, next) => next(e))`));
    expect(report.exitCode).toBe(1);
    expect(report.text).toContain(
      'the hook on "turn.step" is not an async generator: turn.step streams, so it takes async function* ($, e, next) { ... }, which yields the chunks and returns the result',
    );
    const generator = await run(
      inRegister(`  on('turn.step', async function* ($, e, next) { return yield* next(e) })`),
    );
    expect(generator.exitCode).toBe(0);
  });

  test('an imported file not named like code', async () => {
    const report = await run(`import data from './data.json'\n${inRegister('')}`, {
      'hooks/data.json': '{}',
    });
    expect(report.exitCode).toBe(1);
    expect(report.text).toContain(
      'cannot import "./data.json" (from hooks/register.js): hooks/data.json is not named like code and was not loaded: a hooks module and the files it imports end in .ts, .tsx, .jsx, .js, .mjs, .cjs, .mts or .cts',
    );
  });

  test('a $.state ref its declarations do not name, written to $.state.get', async () => {
    const report = await run(
      inRegister(`  on('session.start', async ($, e, next) => {
    await $.state.get({ plugin: 'probe', key: 'nope' })
    return next(e)
  })`),
      { 'types/index.d.ts': TYPES },
    );
    expect(report.exitCode).toBe(1);
    expect(report.text).toContain(
      "probe.nope is not declared: the manifest's types contract must name it in interface PluginState { probe: { nope: ... } }",
    );
  });
});

describe('the --json entries (Agent Plugins Extended J17.15, §17.13)', () => {
  test('an error is { path, message }, and a gating hook names its pattern and the hook with its matcher', async () => {
    const report = await run(
      inRegister(`  on('tool.calls', async ($, e, next) => next(e))
  on('tool.call', { tool: 'Bash' }, async ($, e, next) => next(e))`),
    );
    const [module] = report.contents;
    expect(module?.errors).toEqual([
      {
        path: 'modules../register.js',
        message: `.: ./hooks/register.js:2: "tool.calls" is not an event; ${REMINDER}`,
      },
    ]);
    expect(module?.gatingHooks).toEqual([
      {
        module: './register.js',
        pattern: 'tool.call',
        hook: 'tool.call{tool=Bash}',
        hasCatch: false,
      },
    ]);
  });
});

describe('every result carries the four arrays (Agent Plugins Extended J17.15)', () => {
  test('the manifest, a hooks.json with an issue and a module that cannot be read carry gatingHooks: []', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': manifest({ name: 7 }),
      'hooks/hooks.json': JSON.stringify({ modules: ['./register.js'], hooks: {} }),
    });
    const report = await validateMod({ files, target: '.' });
    expect(report.manifest?.gatingHooks).toEqual([]);
    expect(report.contents.length).toBeGreaterThan(0);
    for (const file of report.contents) expect(file.gatingHooks).toEqual([]);
  });
});

describe('adds to $ and withholds from $ (Agent Plugins Extended §10.4.3, J10.2)', () => {
  test('the keys returned beside the spread are added, those destructured beside the rest withheld', async () => {
    const report = await run(
      inRegister(`  on('engine.create', async ($, e, next) => {
    const { fs, ...beneath } = await next(e)
    return { ...beneath, topo: {} }
  })`),
    );
    expect(report.lines).toContain('  ❯ ./register.js adds to $: topo');
    expect(report.lines).toContain(
      '  ❯ ./register.js withholds from $: fs (in the user tier, the fold refuses this and unloads the module)',
    );
  });

  test('an arrow that spreads next(e) adds its keys and withholds nothing', async () => {
    const report = await run(
      inRegister(
        `  on('engine.create', async ($, e, next) => ({ ...(await next(e)), web: {}, topo: {} }))`,
      ),
    );
    expect(report.lines).toContain('  ❯ ./register.js adds to $: topo, web');
    expect(report.lines).toContain('  ❯ ./register.js withholds from $: nothing');
  });

  test('a hook outside the form, and a module without engine.create, list neither', async () => {
    const outside = await run(
      inRegister(`  on('engine.create', async ($, e, next) => {
    const api = await next(e)
    return { ...api, fs: undefined }
  })`),
    );
    const none = await run(inRegister(`  on('session.start', async ($, e, next) => next(e))`));
    for (const report of [outside, none]) {
      expect(report.text).not.toContain('adds to $');
      expect(report.text).not.toContain('withholds from $');
    }
  });
});

describe('the lead of a scan issue (Agent Plugins Extended §10.4.2, J10.2)', () => {
  test('<plugin>: <absolute file>:<line>: <message>; the reminder, at modules.<entry>, <plugin> the directory name', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': manifest(),
      'hooks/hooks.json': JSON.stringify({ modules: ['./register.js'] }),
      'hooks/register.js': inRegister(`  on('session.start', async ($, e, next) => {
    const x = $
    const u = $.ui
    return $['store']
  })`),
    });
    const report = await validateMod({ files, target: 'my-dir', root: '/work/my-dir' });
    const lead = 'my-dir: /work/my-dir/hooks/register.js';
    expect(report.contents[0]?.errors).toEqual([
      {
        path: 'modules../register.js',
        message: `${lead}:3: $ itself is bound to a name (bound, passed, spread, returned or read); ${REMINDER}`,
      },
      {
        path: 'modules../register.js',
        message: `${lead}:4: $.ui is used as a value (a noun of $ bound, passed or read); ${REMINDER}`,
      },
      {
        path: 'modules../register.js',
        message: `${lead}:5: a computed or optional member access on $; ${REMINDER}`,
      },
    ]);
    expect(report.lines).toContain(
      `  ✘ modules../register.js: ${lead}:3: $ itself is bound to a name (bound, passed, spread, returned or read); ${REMINDER}`,
    );
  });
});
