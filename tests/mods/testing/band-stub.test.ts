/**
 * A `ui.render` stub at the band. Agent Plugins Extended §10.12 ("a `ui.render` stub chooses what
 * `next(e)` returns, and for the band from 2.1.323 the test's hook needs `{ plugin: '<mod>' }` for
 * the stub to answer it"); test.md § Follow the test kit's rules; `plugin` is a default matcher key,
 * `{ plugin: '*' }` naming every mod (§10.13.3; `Matcher`, claude-code/index.d.ts:7505 of 2.1.330).
 * With no stub that answers, `next(e)` gives Claude Code's own drawing.
 */
import { expect, test } from 'bun:test';
import { registerOf } from '../../support/mods-load.ts';
import { runPluginTests, type TestFile } from '../../support/mods-testing.ts';
import { spec } from '../../support/spec.ts';

type Handle = {
  find(q: { type?: string; text?: string }): Promise<unknown>;
  keepsBeneath(): Promise<boolean>;
};
const mounter = (claude: unknown) =>
  (claude as { ui: { mount: (spec: unknown) => Promise<Handle> } }).ui.mount;

// The mod draws nothing of its own in the band: its row is what `next(e)` answers.
const notes = registerOf((on) => {
  on(
    'ui.render',
    { component: 'AbovePrompt' },
    (_$: unknown, e: unknown, next: (e: unknown) => unknown) => next(e),
  );
});

const BAND = {
  plugin: 'notes',
  component: 'AbovePrompt',
  surface: 'terminal',
  props: { hasSurvey: false, isCollapsed: false, isWorking: false, maxRows: 4, bodyColumns: 60 },
} as const;

const DRAWN = { type: 'Text', props: {}, children: ['from the stub'] };

const run = (file: TestFile) =>
  runPluginTests({ name: 'notes', register: notes, tests: { 'tests/band.test.ts': file } });

spec('mods.command.claude plugin test [directory]', () => {
  test('a band stub answers only with { plugin } naming the mod, or *; without it next(e) is Claude Code`s own drawing', async () => {
    const result = await run(({ test, expect }) => {
      test('no plugin in the matcher', async (claude, on) => {
        on('ui.render', () => DRAWN);
        const ui = await mounter(claude)(BAND);
        expect(await ui.find({ type: 'Text' })).toBeUndefined();
        expect(await ui.keepsBeneath()).toBe(true);
      });
      test('the mod named', async (claude, on) => {
        on('ui.render', { plugin: 'notes' }, () => DRAWN);
        const ui = await mounter(claude)(BAND);
        expect(await ui.find({ type: 'Text', text: 'from the stub' })).toBeDefined();
      });
      test('every mod', async (claude, on) => {
        on('ui.render', { plugin: '*' }, () => DRAWN);
        const ui = await mounter(claude)(BAND);
        expect(await ui.find({ type: 'Text', text: 'from the stub' })).toBeDefined();
      });
      test('another mod named', async (claude, on) => {
        on('ui.render', { plugin: 'other' }, () => DRAWN);
        const ui = await mounter(claude)(BAND);
        expect(await ui.find({ type: 'Text' })).toBeUndefined();
      });
    });
    expect(result.lines.join('\n')).not.toContain('(fail)');
    expect(result.passed).toBe(4);
  });

  test('a stub matcher is tested against the event: another field it names keeps it off', async () => {
    const result = await run(({ test, expect }) => {
      test('a component the event is not', async (claude, on) => {
        on('ui.render', { plugin: 'notes', component: 'Pane' }, () => DRAWN);
        const ui = await mounter(claude)(BAND);
        expect(await ui.find({ type: 'Text' })).toBeUndefined();
      });
    });
    expect(result.passed).toBe(1);
  });
});
