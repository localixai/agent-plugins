/**
 * The report lines of `claude plugin validate` that Claude Code 2.1.330 adds (Agent Plugins Extended
 * §10.4.3, Judgment J10.2, oracle 2.1.330): the `(default matcher key: …)` note after a hook whose
 * matcher leaves out a default matcher key (§10.13.3), and `tool.run` and `tool.judge` as gating
 * hooks. Judgments, so outside `spec(...)`.
 */
import { expect, test } from 'bun:test';
import { validateMod } from '../../../src/mods/load/commands/validate.ts';
import { memoryFiles } from '../../support/ports.ts';

const run = async (body: string) => {
  const files = memoryFiles({
    '.claude-plugin/plugin.json': JSON.stringify({ name: 'probe', version: '0.1.0' }),
    'hooks/hooks.json': JSON.stringify({ modules: ['./register.js'] }),
    'hooks/register.js': `export function register(on) {\n${body}\n}`,
  });
  return validateMod({ files, target: '.' });
};

test('a hook on another mod`s controls or drawings whose matcher leaves out a default key is noted on the hooks line', async () => {
  const report = await run(`
  on('ui.press', async ($, e, next) => next(e))
  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => next(e))
  on('ui.select', { plugin: '*', component: 'Pick' }, async ($, e, next) => next(e))
  on('ui.*', async ($, e, next) => next(e))
  on('tool.call', async ($, e, next) => next(e))`);
  expect(report.lines).toContain(
    '  ❯ ./register.js hooks: ui.press (default matcher key: plugin, component), ui.render{component=AbovePrompt} (default matcher key: plugin), ui.select{plugin=*,component=Pick}, ui.*, tool.call',
  );
});

test('tool.run and tool.judge are gating hooks', async () => {
  const report = await run(`
  on('tool.run', async ($, e, next) => next(e))
  on('tool.judge', async ($, e, next) => next(e)).catch(async () => ({ decision: 'deny' }))`);
  expect(report.lines).toContain('  ❯ ./register.js gating hook without .catch: tool.run');
  expect(report.lines).toContain('  ❯ ./register.js gating hook with .catch: tool.judge');
});
