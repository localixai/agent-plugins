import { addDirReads, type AddDirFacts } from '../format/add-dir-gate.ts';
import type { PluginIssue } from '../format/issues.ts';
import type { Settings } from '../protocol/official.ts';
import type { PolicyLayer } from './layers.ts';
import { readSettingsLayer } from './read-layer.ts';

/** The facts of one `--add-dir` file, as `addDirReads` takes them. */
export type AddDirLayerFacts = AddDirFacts;

const KEYS = ['enabledPlugins', 'extraKnownMarketplaces'] as const;

/**
 * One settings file of a directory passed with `--add-dir`, read as an `addDir` layer (spec
 * §16.4.6): of its two keys, only those `addDirReads` admits for this file stay, so a file whose
 * setting sources do not load gives nothing, and in an interactive terminal an untrusted directory
 * gives neither key; outside one its `true` entries are read and its marketplaces still wait for
 * saved trust. `enabledPlugin` and the marketplace registry read the layer as it is kept.
 */
export function readAddDirLayer(
  file: AddDirFacts['file'],
  raw: unknown,
  facts: Omit<AddDirFacts, 'file'>,
  source?: string,
): { readonly layer: PolicyLayer; readonly issues: readonly PluginIssue[] } {
  const read = readSettingsLayer('addDir', raw, source);
  const admitted = addDirReads({ ...facts, file });
  const settings: Settings = Object.fromEntries(
    KEYS.filter((key) => admitted[key] && read.layer.settings[key] !== undefined).map((key) => [
      key,
      read.layer.settings[key],
    ]),
  );
  return { layer: { ...read.layer, settings }, issues: read.issues };
}
