import type { SettingSource } from '../protocol/official.ts';

/**
 * What a directory passed with `--add-dir` contributes to plugins (spec §16.4.6; plugins/loading.md,
 * "Enabled in an --add-dir directory but not loaded"): only the `enabledPlugins` and
 * `extraKnownMarketplaces` keys of its two settings files, each file only when its setting sources
 * load (`--setting-sources`, all three by default), and only once the user has saved workspace trust
 * for the directory. Answering the prompt `--add-dir` shows does not save that trust. Pure: the host
 * knows the trust, the sources and the terminal, and reads the files.
 */

export type AddDirFile = '.claude/settings.json' | '.claude/settings.local.json';

export interface AddDirFacts {
  readonly file: AddDirFile;
  /** The setting sources this session loads. */
  readonly settingSources: readonly SettingSource[];
  /** The user saved workspace trust for the directory (a session started there, or the trust key). */
  readonly trustSaved: boolean;
  /** An interactive terminal session: there an untrusted directory's `true` entries are skipped too. */
  readonly interactive: boolean;
}

export interface AddDirReads {
  /** Whether the file's `enabledPlugins` entries set to `true` are read. */
  readonly enabledPlugins: boolean;
  readonly extraKnownMarketplaces: boolean;
}

/** The setting sources each file of the added directory needs (§16.4.6 item 2). */
const NEEDS: { readonly [F in AddDirFile]: readonly SettingSource[] } = {
  '.claude/settings.json': ['project'],
  '.claude/settings.local.json': ['project', 'local'],
};

/** Which of the two keys of one file of an added directory are read. */
export function addDirReads(facts: AddDirFacts): AddDirReads {
  const loaded = NEEDS[facts.file].every((source) => facts.settingSources.includes(source));
  if (!loaded) return { enabledPlugins: false, extraKnownMarketplaces: false };
  return {
    enabledPlugins: facts.trustSaved || !facts.interactive,
    extraKnownMarketplaces: facts.trustSaved,
  };
}
