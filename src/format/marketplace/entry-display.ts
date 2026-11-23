import type { PluginIssue } from '../issues.ts';
import { enginesFindings, readEngines, type EnginesRead } from '../engines.ts';
import type { MarketplaceEntry, ReadMode } from './types.ts';

/**
 * The two entry fields whose problems do not drop an entry at load (spec §15.4, §15.5.3; Judgment
 * J15.15): a display field of the wrong type, or a `homepage` that is not a URL, keeps the entry
 * listed but makes it invalid to install; a malformed `engines` is ignored and the entry installs.
 * A validator reports both as `claude plugin validate` does.
 */

/** The display fields of `plugin.json` (§2.1 item 3) as an entry carries them. */
const DISPLAY_KEYS = [
  '$schema',
  'displayName',
  'description',
  'author',
  'homepage',
  'repository',
  'license',
  'keywords',
] as const;

/** The display field an issue at `path` is about, when it is one. */
function displayKeyOf(path: string, base: string): string | undefined {
  return DISPLAY_KEYS.find((key) => {
    const at = `${base}.${key}`;
    return path === at || path.startsWith(`${at}.`) || path.startsWith(`${at}[`);
  });
}

export interface DisplaySplit {
  /** The issues an entry is read with. */
  readonly issues: readonly PluginIssue[];
  /** At load, each display problem as `<field>: <problem>`; the entry is kept and refuses to install. */
  readonly invalid: readonly string[];
}

/** At load, the display fields' errors leave the issues and become the entry's `invalid` list. */
export function splitDisplayIssues(
  issues: readonly PluginIssue[],
  base: string,
  mode: ReadMode,
): DisplaySplit {
  if (mode === 'validate') return { issues, invalid: [] };
  const kept: PluginIssue[] = [];
  const invalid: string[] = [];
  for (const found of issues) {
    const key = displayKeyOf(found.path, base);
    if (key === undefined) kept.push(found);
    else invalid.push(`${found.path.slice(base.length + 1)}: ${found.message}`);
  }
  return { issues: kept, invalid };
}

/** `engines`: the validator's findings in `validate` mode; what loading takes from it either way. */
export function readEntryEngines(
  value: unknown,
  base: string,
  mode: ReadMode,
): { readonly engines: EnginesRead | undefined; readonly issues: readonly PluginIssue[] } {
  if (value === undefined) return { engines: undefined, issues: [] };
  const issues = mode === 'validate' ? enginesFindings(value, `${base}.engines`) : [];
  const read = readEngines(value);
  const engines = read.range === undefined && read.invalid === undefined ? undefined : read;
  return { engines, issues };
}

/**
 * Why installing this entry fails (§15.5.3): `This plugin's marketplace entry is invalid:` and its
 * problems joined by `, `; `undefined` for an entry that installs.
 */
export function invalidEntryMessage(entry: Pick<MarketplaceEntry, 'invalid'>): string | undefined {
  const problems = entry.invalid ?? [];
  return problems.length === 0
    ? undefined
    : `This plugin's marketplace entry is invalid: ${problems.join(', ')}`;
}
