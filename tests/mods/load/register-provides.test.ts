/**
 * Agent Plugins Extended §10.19.5 (`plugin.register`'s event, 2.1.330: `e.provides`, `e.unread`,
 * `e.hosts`, `e.requiredByOrganization`, `e.uses.hosts`), §10.8.1 (a refusal keeps the whole plugin
 * out from 2.1.326; `hooks module left out by <mod>`; the event fires for a plugin with no hooks
 * module) and §14.9.8 (`e.provides`).
 */
import { expect, test } from 'bun:test';
import type { RegisterFunction } from '../../../src/testing/mods/index.ts';
import {
  installed,
  managed,
  modPlugin,
  organizations,
  registerOf,
  session,
} from '../../support/mods-load.ts';
import { memoryFiles } from '../../support/ports.ts';
import { spec } from '../../support/spec.ts';

type Event = Record<string, unknown> & { name: string; provides: Record<string, unknown[]> };
type Next = (e: unknown) => Promise<unknown>;

const reader = registerOf((on) => {
  on('tool.call', async () => ({ result: 'reader answered' }));
});

/** A policy mod whose `plugin.register` hook passes on what `change` makes of the event. */
const policyMod = (change: (e: Event) => unknown, seen: Event[] = []): RegisterFunction =>
  registerOf((on) => {
    on('plugin.register', async (_$: unknown, e: Event, next: Next) => {
      seen.push(e);
      return e.name === 'acme-guard' ? next(e) : next(change(e));
    });
  });

const PARTS = { skills: ['review', 'deploy'], themes: ['dusk'], mcpServers: [{ name: 'db' }] };

async function loadUnder(
  guard: RegisterFunction,
  subject = modPlugin(installed('reader'), reader),
) {
  const s = session(
    { 'acme-guard': guard, reader },
    { policy: managed({ prependPlugins: ['acme-guard@acme-tools'] }) },
  );
  const report = await s.host.load([
    modPlugin(organizations('acme-guard'), guard),
    { ...subject, register: { provides: PARTS } },
  ]);
  return { ...s, report };
}

spec('policy.mods.Enforce a policy with a mod of your own.e.provides', () => {
  test('e.provides lists every kind, the host`s parts and the hooks module', async () => {
    const seen: Event[] = [];
    await loadUnder(policyMod((e) => e, seen));
    const subject = seen.find((e) => e.name === 'reader');
    expect(Object.keys(subject?.provides ?? {}).toSorted()).toEqual([
      'agents',
      'bin',
      'commands',
      'hooks',
      'lspServers',
      'mcpServers',
      'modules',
      'monitors',
      'outputStyles',
      'skills',
      'themes',
      'workflows',
    ]);
    expect(subject?.provides).toMatchObject({
      ...PARTS,
      modules: [{ path: 'hooks/register.js' }],
    });
  });

  test('passing next the event with modules emptied loads every part but the hooks module: the line says `hooks module left out by <mod>`', async () => {
    const guard = policyMod((e) => ({ ...e, provides: { ...e.provides, modules: [] } }));
    const { report, debug } = await loadUnder(guard);
    expect(report.loaded.map((mod) => mod.name)).toEqual(['acme-guard']);
    expect(report.lines).toEqual(['reader: hooks module left out by acme-guard']);
    expect(debug).toContain('reader: hooks module left out by acme-guard');
    expect(report.keptOut).toEqual([
      {
        plugin: 'reader@market',
        verdict: {
          loads: true,
          leftOut: { modules: [{ path: 'hooks/register.js' }] },
          by: 'acme-guard',
        },
      },
    ]);
  });

  test('entries removed from another kind stay out while the rest, the hooks module included, load', async () => {
    const guard = policyMod((e) => ({ ...e, provides: { ...e.provides, skills: ['review'] } }));
    const { report } = await loadUnder(guard);
    expect(report.loaded.map((mod) => mod.name)).toContain('reader');
    expect(report.keptOut).toEqual([
      { plugin: 'reader@market', verdict: { loads: true, leftOut: { skills: ['deploy'] } } },
    ]);
  });

  test('a list that is not a subset of the one received, or a narrowed themes or bin, is refused: the hook fails and the plugin loads whole', async () => {
    for (const change of [
      (e: Event) => ({ ...e, provides: { ...e.provides, skills: ['review', 'deploy', 'extra'] } }),
      (e: Event) => ({ ...e, provides: { ...e.provides, mcpServers: [{ name: 'other' }] } }),
      (e: Event) => ({ ...e, provides: { ...e.provides, themes: [] } }),
    ]) {
      const { report, reports } = await loadUnder(policyMod(change));
      expect(report.loaded.map((mod) => mod.name)).toContain('reader');
      expect(report.keptOut).toEqual([]);
      expect(reports.map((one) => one.line).join('\n')).toContain('e.provides');
    }
  });

  test('a provides left out is restored: everything loads', async () => {
    const guard = policyMod(({ provides: _provides, ...rest }) => rest);
    const { report } = await loadUnder(guard);
    expect(report.loaded.map((mod) => mod.name)).toContain('reader');
    expect(report.keptOut).toEqual([]);
  });

  test('what loads is what every mod`s next in the dispatch left in', async () => {
    const first = policyMod((e) => ({ ...e, provides: { ...e.provides, skills: ['review'] } }));
    const second = policyMod((e) => ({ ...e, provides: { ...e.provides, mcpServers: [] } }));
    const s = session(
      { 'acme-guard': first, 'acme-two': second, reader },
      { policy: managed({ prependPlugins: ['acme-guard@acme-tools', 'acme-two@acme-tools'] }) },
    );
    const report = await s.host.load([
      modPlugin(organizations('acme-guard'), first),
      modPlugin(organizations('acme-two'), second),
      { ...modPlugin(installed('reader'), reader), register: { provides: PARTS } },
    ]);
    expect(report.keptOut).toEqual([
      {
        plugin: 'reader@market',
        verdict: { loads: true, leftOut: { mcpServers: [{ name: 'db' }], skills: ['deploy'] } },
      },
    ]);
  });
});

spec('mods.event.plugin.register', () => {
  test('{ refuse } keeps the whole plugin out (2.1.326): the report says so for the host', async () => {
    const guard = registerOf((on) => {
      on('plugin.register', async (_$: unknown, e: Event, next: Next) =>
        e.name === 'reader' ? { refuse: 'not on the list' } : next(e),
      );
    });
    const { report } = await loadUnder(guard);
    expect(report.lines).toEqual(['reader: refused by acme-guard: not on the list']);
    expect(report.keptOut).toEqual([
      {
        plugin: 'reader@market',
        verdict: { loads: false, by: 'acme-guard', reason: 'not on the list' },
      },
    ]);
  });

  test('it fires for a plugin with no hooks module, with empty uses and no modules; a refusal keeps it out', async () => {
    const seen: Event[] = [];
    const guard = registerOf((on) => {
      on('plugin.register', async (_$: unknown, e: Event, next: Next) => {
        seen.push(e);
        return e.name === 'skills-only' ? { refuse: 'no' } : next(e);
      });
    });
    const files = memoryFiles({
      '.claude-plugin/plugin.json': JSON.stringify({ name: 'skills-only', version: '2.0.0' }),
      'hooks/hooks.json': JSON.stringify({ hooks: {} }),
    });
    const { report } = await loadUnder(guard, { candidate: installed('skills-only'), files });
    const event = seen.find((e) => e.name === 'skills-only');
    expect(event).toMatchObject({ version: '2.0.0', uses: { events: [], calls: [] } });
    expect(event?.provides).toMatchObject({ modules: [], skills: ['review', 'deploy'] });
    expect(report.keptOut.map((one) => one.plugin)).toEqual(['skills-only@market']);
  });

  test('e.unread, e.hosts and e.requiredByOrganization come from the host; e.uses.hosts from the scan', async () => {
    const seen: Event[] = [];
    const guard = policyMod((e) => e, seen);
    const fetcher = registerOf((on) => {
      on(
        'session.start',
        async ($: { http: { fetch(url: string): Promise<unknown> } }, e: unknown, next: Next) => {
          await $.http.fetch('https://api.example.com/v1');
          return next(e);
        },
      );
    });
    const s = session(
      { 'acme-guard': guard, fetcher },
      { policy: managed({ prependPlugins: ['acme-guard@acme-tools'] }) },
    );
    await s.host.load([
      modPlugin(organizations('acme-guard'), guard),
      {
        ...modPlugin(installed('fetcher'), fetcher),
        register: {
          unread: ['skills', 'modules'],
          hosts: { named: [{ host: 'api.example.com' }], unread: [] },
          requiredByOrganization: true,
        },
      },
    ]);
    expect(seen.find((e) => e.name === 'fetcher')).toMatchObject({
      unread: ['skills'],
      hosts: { named: [{ host: 'api.example.com' }], unread: [] },
      requiredByOrganization: true,
      uses: { hosts: { named: ['api.example.com'], unread: [] } },
    });
  });
});
