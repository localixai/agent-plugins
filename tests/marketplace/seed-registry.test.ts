/**
 * Spec §14.8.4 (plugins/org.md § Seed containers and CI: read-only, build from the declared source,
 * opting out, policy still applies; Judgment J14.16) and plugins/mods/admin.md § Know which
 * controls still apply ("Plugin controls cover mods").
 */
import { describe, expect, test } from 'bun:test';
import {
  decideEnabledFetch,
  overriddenUserDisable,
} from '../../src/marketplace/enablement-notes.ts';
import { applySeeds } from '../../src/marketplace/seed-registry.ts';
import { github, managed } from '../support/marketplace-sources.ts';
import { spec } from '../support/spec.ts';

const entry = (repo: string, autoUpdate?: boolean) => ({
  source: github(repo),
  ...(autoUpdate === undefined ? {} : { autoUpdate }),
});

/**
 * Spec J14.16 leaves open which entry applies when a seed and a declaration of the same name differ;
 * the kit keeps the seed's, so the copy on disk is the one read (spec/judgments.md).
 */
describe('judgment J14.16: a seed marketplace and a user entry of the same name', () => {
  test("a marketplace declared in the seed overwrites the user's entry of the same name", () => {
    const registry = applySeeds({ shared: entry('user/own'), mine: entry('user/mine') }, [
      { directory: '/seed', marketplaces: { shared: entry('org/shared') } },
    ]);
    expect(registry['shared']?.entry.source).toEqual(github('org/shared'));
    expect(registry['shared']?.seedDirectory).toBe('/seed');
    expect(registry['mine']?.seedDirectory).toBeUndefined();
  });

  test('a seed marketplace the user has no entry for is registered', () => {
    const registry = applySeeds({}, [
      { directory: '/seed', marketplaces: { fresh: entry('o/f') } },
    ]);
    expect(Object.keys(registry)).toEqual(['fresh']);
  });

  test('the first seed that contains a marketplace supplies it', () => {
    const registry = applySeeds({}, [
      { directory: '/one', marketplaces: { m: entry('a/first') } },
      { directory: '/two', marketplaces: { m: entry('b/second'), n: entry('b/n') } },
    ]);
    expect(registry['m']?.seedDirectory).toBe('/one');
    expect(registry['n']?.seedDirectory).toBe('/two');
  });

  test('a seed marketplace is read-only and its autoUpdate is forced off', () => {
    const registry = applySeeds({}, [
      { directory: '/seed', marketplaces: { m: entry('a/b', true) } },
    ]);
    expect(registry['m']?.entry.autoUpdate).toBe(false);
  });
});

spec('policy.org.Seed containers and CI.Build from the declared source', () => {
  test('a seed built from the declared source and ref registers that very source, read-only', () => {
    const declared: { source: ReturnType<typeof github>; autoUpdate?: boolean } = {
      source: github('org/shared', { ref: 'v2' }),
    };
    const registry = applySeeds({ shared: declared }, [
      {
        directory: '/seed',
        marketplaces: { shared: { source: github('org/shared', { ref: 'v2' }) } },
      },
    ]);
    expect(registry['shared']?.entry.source).toEqual(declared.source);
    expect(registry['shared']?.seedDirectory).toBe('/seed');
  });
});

spec('policy.org.Seed containers and CI.Opting out', () => {
  test('a seed plugin the repository enables loads from the seed, with nothing to fetch and no error', () => {
    const decision = decideEnabledFetch({
      name: 'formatter',
      marketplace: 'shared',
      trueIn: [{ source: 'project' }],
      installed: false,
      relativePath: false,
      seeded: true,
      marketplaceOnMachine: 'remote',
      moment: 'session-start',
    });
    expect(decision).toEqual({ fetch: false, inPlace: false });
  });

  test('the user opts out by disabling the plugin: a false in .claude/settings.local.json', () => {
    expect(overriddenUserDisable({ user: false, decidedBy: 'project', on: true })?.optOut).toEqual({
      file: '.claude/settings.local.json',
      value: false,
    });
  });
});

spec('policy.org.Seed containers and CI.Policy still applies', () => {
  test("the allowlist and blocklist check a seed marketplace's recorded source", () => {
    const recorded = github('built/from-here');
    const allow = managed({ strictKnownMarketplaces: [github('other/plugins')] });
    expect(allow.checkRegistered('seeded', recorded).loads).toBe(false);
    expect(
      managed({ strictKnownMarketplaces: [recorded] }).checkRegistered('seeded', recorded).loads,
    ).toBe(true);
    const block = managed({ blockedMarketplaces: [recorded] });
    expect(block.checkRegistered('seeded', recorded).loads).toBe(false);
  });
});

spec(
  [
    'policy.mods.Know which controls still apply.Plugin controls cover mods',
    'policy.mods.Apply your plugin controls to mods',
    'policy.mods.Apply your plugin controls to mods.Offer mods your users may install',
    'policy.mods.Apply your plugin controls to mods.Set up CI and containers',
  ],
  () => {
    describe('a mod is a plugin', () => {
      test('strictKnownMarketplaces decides whether the marketplace that holds it can be used at all', () => {
        const gate = managed({ strictKnownMarketplaces: [github('acme/mods')] });
        expect(gate.check(github('acme/mods'))).toEqual({ allowed: true });
        expect(gate.check(github('stranger/mods')).allowed).toBe(false);
      });
    });
  },
);
