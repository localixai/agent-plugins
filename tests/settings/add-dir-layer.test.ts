/**
 * Spec §16.4.6 (plugins/loading.md, "Enabled in an --add-dir directory but not loaded"), read
 * through the settings: a `--add-dir` directory's file gives `enabledPlugins` and
 * `extraKnownMarketplaces` only when that file's setting sources load, and an untrusted directory's
 * `true` entries are skipped in an interactive terminal; its marketplaces need saved trust.
 * `readAddDirLayer` is the layer a host builds for each file; `enabledPlugin` and the marketplace
 * registry read what it kept.
 */
import { expect, test } from 'bun:test';
import {
  createPolicyView,
  readAddDirLayer,
  type AddDirLayerFacts,
} from '../../src/settings/index.ts';
import { enabledPlugin } from '../../src/settings/plugins/index.ts';
import { createMarketplaceRegistry } from '../../src/settings/marketplaces/index.ts';
import { github } from '../support/marketplace-sources.ts';
import { spec } from '../support/spec.ts';

const ID = 'formatter@team';
const RAW = {
  enabledPlugins: { [ID]: true },
  extraKnownMarketplaces: { team: { source: github('acme/team') } },
  model: 'opus',
};
const ALL = ['user', 'project', 'local'] as const;
const facts = (extra: Partial<Omit<AddDirLayerFacts, 'file'>> = {}) => ({
  settingSources: ALL,
  trustSaved: true,
  interactive: true,
  ...extra,
});

function read(file: AddDirLayerFacts['file'], extra: Partial<Omit<AddDirLayerFacts, 'file'>> = {}) {
  const layer = readAddDirLayer(file, RAW, facts(extra)).layer;
  const view = createPolicyView([layer]);
  const names = createMarketplaceRegistry(view, {
    isFolderTrusted: () => true,
    officialMarketplaces: [],
    check: () => ({ allowed: true }),
  })
    .registrations()
    .filter((each) => each.status === 'register')
    .map((each) => each.name);
  return { on: enabledPlugin(view, ID, false).on, marketplaces: names };
}

spec(
  [
    'install.loading.Enabled in an --add-dir directory but not loaded..claude/settings.json',
    'install.loading.Enabled in an --add-dir directory but not loaded.settings.json',
  ],
  () => {
    test('with saved trust and the project source loaded, both keys are read; nothing else is', () => {
      expect(read('.claude/settings.json')).toEqual({ on: true, marketplaces: ['team'] });
      const { layer } = readAddDirLayer('.claude/settings.json', RAW, facts());
      expect(Object.keys(layer.settings).toSorted()).toEqual([
        'enabledPlugins',
        'extraKnownMarketplaces',
      ]);
    });

    test('--setting-sources without project leaves the file unread', () => {
      expect(read('.claude/settings.json', { settingSources: ['user', 'local'] })).toEqual({
        on: false,
        marketplaces: [],
      });
    });

    test('in an interactive terminal an untrusted directory gives neither key', () => {
      expect(read('.claude/settings.json', { trustSaved: false })).toEqual({
        on: false,
        marketplaces: [],
      });
    });

    test('outside an interactive terminal its true entries are read, its marketplaces still need trust', () => {
      expect(read('.claude/settings.json', { trustSaved: false, interactive: false })).toEqual({
        on: true,
        marketplaces: [],
      });
    });
  },
);

spec(
  [
    'install.loading.Enabled in an --add-dir directory but not loaded..claude/settings.local.json',
    'install.loading.Enabled in an --add-dir directory but not loaded.settings.local.json',
  ],
  () => {
    test('the local file needs both the project and the local source', () => {
      expect(read('.claude/settings.local.json')).toEqual({ on: true, marketplaces: ['team'] });
      for (const sources of [
        ['user', 'project'],
        ['user', 'local'],
      ] as const) {
        expect(read('.claude/settings.local.json', { settingSources: sources })).toEqual({
          on: false,
          marketplaces: [],
        });
      }
    });
  },
);
