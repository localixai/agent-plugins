import { coreNameProblems } from '../../format/core/name.ts';
import { coreSchemaId } from '../../format/core/versions.ts';
import type { Plugin } from '../../loading/plugin-model.ts';
import { notConverted, stop } from '../notes.ts';
import { CLAUDE_MANIFEST, SHARED_FIELDS } from '../to-claude-code/manifest.ts';
import type { ConversionNote } from '../types.ts';

/**
 * The root `plugin.json` of a Claude Code plugin (Agent Plugins Extended §18.4.3 item 1): the name
 * must satisfy core §5.5 or the conversion stops (§18.4.1 item 2: never renamed); the canonical
 * `$schema` and the nine shared fields are written; every other manifest field is reported. The
 * component keys `skills`, `hooks` and `mcpServers` are converted through the components they load
 * and reported there.
 */

export const CORE_MANIFEST = 'plugin.json';

const SHARED: ReadonlySet<string> = new Set(SHARED_FIELDS);
const THROUGH_COMPONENTS: ReadonlySet<string> = new Set(['skills', 'hooks', 'mcpServers']);

export interface CoreManifestPart {
  readonly value: Readonly<Record<string, unknown>>;
  readonly notes: readonly ConversionNote[];
}

export function coreManifestOf(plugin: Plugin): CoreManifestPart {
  const manifest = plugin.manifest ?? { name: plugin.name };
  const problems = coreNameProblems(plugin.name);
  const naming = problems.map((problem) =>
    stop(
      CLAUDE_MANIFEST,
      'name',
      `the name "${plugin.name}" is not a core package name (${problem}), and a converter never renames`,
    ),
  );
  const others = Object.keys(manifest)
    .filter((key) => !SHARED.has(key) && !THROUGH_COMPONENTS.has(key))
    .map((key) => notConverted(CLAUDE_MANIFEST, key, `"${key}" has no core counterpart`));
  const value = {
    $schema: coreSchemaId('1.1.0', 'plugin'),
    name: plugin.name,
    ...Object.fromEntries(
      SHARED_FIELDS.filter((field) => field !== 'name' && manifest[field] !== undefined).map(
        (field) => [field, manifest[field]],
      ),
    ),
  };
  return { value, notes: [...naming, ...others] };
}
