/**
 * Spec §16.4.6 (plugins/loading.md, "Enabled in an --add-dir directory but not loaded"): a directory
 * passed with `--add-dir` gives its `enabledPlugins` and `extraKnownMarketplaces` only once the user
 * saved workspace trust for it, and each of its files only when that file's setting sources load.
 */
import { describe, expect, test } from 'bun:test';
import { addDirReads, type AddDirFacts } from '../../src/format/add-dir-gate.ts';
import { spec } from '../support/spec.ts';

const ALL = ['user', 'project', 'local'] as const;
const facts = (extra: Partial<AddDirFacts> = {}): AddDirFacts => ({
  file: '.claude/settings.json',
  settingSources: ALL,
  trustSaved: true,
  interactive: true,
  ...extra,
});
const BOTH = { enabledPlugins: true, extraKnownMarketplaces: true };
const NEITHER = { enabledPlugins: false, extraKnownMarketplaces: false };

describe('trust (§16.4.6 item 1)', () => {
  test('a trusted directory gives both keys', () => {
    expect(addDirReads(facts())).toEqual(BOTH);
  });

  test('until trust is saved its marketplaces are skipped, and in an interactive terminal its true entries too', () => {
    expect(addDirReads(facts({ trustSaved: false }))).toEqual(NEITHER);
    expect(addDirReads(facts({ trustSaved: false, interactive: false }))).toEqual({
      enabledPlugins: true,
      extraKnownMarketplaces: false,
    });
  });
});

spec(
  'install.loading.Enabled in an --add-dir directory but not loaded..claude/settings.json',
  () => {
    test('.claude/settings.json is read only when the project source loads', () => {
      expect(addDirReads(facts({ settingSources: ['project'] }))).toEqual(BOTH);
      expect(addDirReads(facts({ settingSources: ['user', 'local'] }))).toEqual(NEITHER);
    });
  },
);

spec(
  'install.loading.Enabled in an --add-dir directory but not loaded..claude/settings.local.json',
  () => {
    test('.claude/settings.local.json needs the project and local sources', () => {
      const local = { file: '.claude/settings.local.json' } as const;
      expect(addDirReads(facts({ ...local, settingSources: ['project', 'local'] }))).toEqual(BOTH);
      expect(addDirReads(facts({ ...local, settingSources: ['project'] }))).toEqual(NEITHER);
      expect(addDirReads(facts({ ...local, settingSources: ['local'] }))).toEqual(NEITHER);
    });
  },
);

spec('install.loading.Enabled in an --add-dir directory but not loaded.settings.json', () => {
  test('--setting-sources user reads neither file; adding project reads settings.json', () => {
    expect(addDirReads(facts({ settingSources: ['user'] }))).toEqual(NEITHER);
    expect(addDirReads(facts({ settingSources: ['user', 'project'] }))).toEqual(BOTH);
  });
});

spec('install.loading.Enabled in an --add-dir directory but not loaded.settings.local.json', () => {
  test('a true in settings.local.json needs user,project,local', () => {
    const local = { file: '.claude/settings.local.json' } as const;
    expect(addDirReads(facts({ ...local, settingSources: ['user', 'project'] }))).toEqual(NEITHER);
    expect(addDirReads(facts({ ...local, settingSources: ALL }))).toEqual(BOTH);
  });
});
