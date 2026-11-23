/**
 * Spec §10.13.3, "Other mods' drawings and controls" (plugins/mods/events.md of Claude Code 2.1.330;
 * `Matcher`, claude-code/index.d.ts:7505): `plugin` is a default matcher key on `ui.press`,
 * `ui.input`, `ui.select`, `ui.focus` and `ui.render`. And §10.16.3 item 1: a `tool.check` with `op`
 * reaches only a hook whose matcher names `op`, and `isCeilingUnread` is pinned.
 */
import { describe, expect, test } from 'bun:test';
import { EVENT_RULES } from '../../../src/mods/events/index.ts';
import { fn, harness, type FnHook } from '../../support/mods-chain.ts';
import { spec } from '../../support/spec.ts';

/** A hook that writes `<mod>:<label>` for each event it sees, then passes it on. */
const seeing =
  (seen: string[], label: string): FnHook =>
  (_$, e, next) => (seen.push(`${label} ${String(e.plugin)}`), next(e));

spec("mods.events.Hook other mods' drawings and controls", () => {
  test('a hook on ui.press without plugin runs for its own mod controls only; { plugin } names another mod', async () => {
    for (const event of ['ui.press', 'ui.input', 'ui.select', 'ui.focus']) {
      const h = harness();
      const seen: string[] = [];
      h.mod('mine', 'user', [
        fn(event, seeing(seen, 'own')),
        fn(event, seeing(seen, 'theirs'), { matcher: { plugin: 'other' } }),
      ]);
      await h.fire(event, { plugin: 'mine', id: 'go' });
      await h.fire(event, { plugin: 'other', id: 'go' });
      expect(seen, event).toEqual(['own mine', 'theirs other']);
    }
  });

  test('{ plugin: "*" } runs for every mod and for an event that carries no plugin', async () => {
    const h = harness();
    const seen: string[] = [];
    h.mod('mine', 'user', [fn('ui.render', seeing(seen, 'all'), { matcher: { plugin: '*' } })]);
    await h.fire('ui.render', { component: 'Pane', plugin: 'other' });
    await h.fire('ui.render', { component: 'Pane', plugin: 'mine' });
    await h.fire('ui.render', { component: 'StatusLine' });
    expect(seen).toEqual(['all other', 'all mine', 'all undefined']);
  });

  test('a ui.render hook without plugin skips another mod pane and runs at the sites the client draws itself', async () => {
    const h = harness();
    const seen: string[] = [];
    h.mod('mine', 'user', [fn('ui.render', seeing(seen, 'r'))]);
    await h.fire('ui.render', { component: 'Pane', plugin: 'other' });
    await h.fire('ui.render', { component: 'AbovePrompt', plugin: 'mine' });
    await h.fire('ui.render', { component: 'StatusLine' });
    expect(seen).toEqual(['r mine', 'r undefined']);
  });

  test('a wildcard with no matcher runs for every mod; a wildcard with a matcher needs plugin like any other', async () => {
    const h = harness();
    const seen: string[] = [];
    h.mod('mine', 'user', [
      fn('ui.*', seeing(seen, 'bare')),
      fn('*', seeing(seen, 'filtered'), { matcher: { id: 'go' } }),
    ]);
    await h.fire('ui.press', { plugin: 'other', id: 'go' });
    await h.fire('ui.press', { plugin: 'mine', id: 'go' });
    expect(seen).toEqual(['bare other', 'bare mine', 'filtered mine']);
  });

  test('a matcher on ui.render that names component keeps the hook off AbovePromptList', async () => {
    const h = harness();
    const seen: string[] = [];
    h.mod('mine', 'user', [
      fn('ui.render', seeing(seen, 'any'), { matcher: { component: /./, plugin: '*' } }),
      fn('ui.render', seeing(seen, 'list'), { matcher: { plugin: '*' } }),
    ]);
    await h.fire('ui.render', { component: 'AbovePromptList' });
    await h.fire('ui.render', { component: 'AbovePrompt', plugin: 'mine' });
    expect(seen).toEqual(['list undefined', 'any mine', 'list mine']);
  });

  // reference.md §10.21.1 item 4: a hook on AbovePromptList "hears a bare on('*') or a matcher naming it".
  test('a matcher that names AbovePromptList as its component hears the list', async () => {
    const h = harness();
    const seen: string[] = [];
    h.mod('mine', 'user', [
      fn('ui.render', seeing(seen, 'named'), { matcher: { component: 'AbovePromptList' } }),
    ]);
    await h.fire('ui.render', { component: 'AbovePromptList' });
    expect(seen).toEqual(['named undefined']);
  });

  test('on other events plugin is an ordinary field: a hook without it runs whatever the event carries', async () => {
    const h = harness();
    const seen: string[] = [];
    h.mod('mine', 'user', [fn('tool.call', seeing(seen, 'call'))]);
    await h.fire('tool.call', { tool: 'Bash', plugin: 'other' });
    expect(seen).toEqual(['call other']);
  });
});

describe('tool.check with op (spec §10.16.3 item 1)', () => {
  spec('mods.events.Approve or refuse a tool call before the user is asked.Arguments', () => {
    test('a tool.check that stands for a mod own $ call (op) reaches only the hooks whose matcher names op', async () => {
      const h = harness([EVENT_RULES]);
      const seen: string[] = [];
      h.mod('policy', 'prepend', [
        fn('tool.check', (_$, e, next) => (seen.push(`any ${String(e.tool)}`), next(e))),
        fn('tool.check', () => (seen.push('op'), { decision: 'deny', reason: 'no' }), {
          matcher: { op: /./ },
        }),
      ]);
      const op = { tool: 'Read', input: { file_path: '.env' }, op: 'fs.read', ceiling: 'deny' };
      expect(await h.fire('tool.check', op)).toEqual({ decision: 'deny', reason: 'no' });
      expect(seen).toEqual(['op']);
    });

    test('isCeilingUnread and op are pinned: a hook that changes either fails, and the event goes on as fired', async () => {
      const h = harness([EVENT_RULES]);
      const errors: string[] = [];
      h.mod('policy', 'prepend', [
        fn('tool.check', (_$, e, next) => next({ ...e, isCeilingUnread: undefined }), {
          handler: (_$, e, next) => (errors.push(next.error.message ?? ''), next(e)),
        }),
      ]);
      const check = { tool: 'Bash', input: { command: 'ls' }, isCeilingUnread: true };
      await h.fire('tool.check', check, { core: () => Promise.resolve({ decision: 'allow' }) });
      expect(errors).toEqual([
        'next argument: tool.check: "isCeilingUnread" is pinned; a hook cannot change it',
      ]);
    });
  });
});

test('tool.describe mcpServer is pinned: a rewrite that changes it is refused (spec §10.16.4)', async () => {
  const h = harness([EVENT_RULES]);
  const errors: string[] = [];
  h.mod('mine', 'user', [
    fn('tool.describe', (_$, e, next) => next({ ...e, mcpServer: { type: 'http' } }), {
      handler: (_$, e, next) => (errors.push(next.error.message ?? ''), next(e)),
    }),
  ]);
  await h.fire('tool.describe', {
    tool: 'mcp__a__b',
    description: 'd',
    mcpServer: { type: 'stdio' },
  });
  expect(errors).toEqual([
    'next argument: tool.describe: "mcpServer" is pinned; a hook cannot change it',
  ]);
});
