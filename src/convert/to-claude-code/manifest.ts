import type { CoreManifest } from '../../format/core/manifest.ts';
import { nameValidationIssues } from '../../format/names.ts';
import { notConverted, reported, stop } from '../notes.ts';
import type { ConversionNote } from '../types.ts';

/**
 * `.claude-plugin/plugin.json` from a core manifest (Agent Plugins Extended §18.4.2 item 1): the
 * nine shared fields copied, `$schema` dropped, `extensions` reported. An `author` without `name` and
 * a `homepage` that is no URL fail in Claude Code (§2.2): each is left out and reported, never
 * invented. The name is never changed (§18.4.1 item 2): one `claude plugin validate` refuses (a
 * reserved name) stops the conversion, since its output could not pass the validator (item 3;
 * judgment, spec/judgments.md), and its warnings (not kebab-case, J18.19) are reported.
 */

export const CLAUDE_MANIFEST = '.claude-plugin/plugin.json';
const SOURCE = 'plugin.json';

const NOTES_BY_SEVERITY = { error: stop, warning: reported } as const;

function nameNotes(name: string): ConversionNote[] {
  return nameValidationIssues(name).map((issue) =>
    NOTES_BY_SEVERITY[issue.severity](SOURCE, 'name', issue.message),
  );
}

/** The nine fields both manifests define, in the order the converter writes them (§18.3.1). */
export const SHARED_FIELDS = [
  'name',
  'displayName',
  'version',
  'description',
  'author',
  'homepage',
  'repository',
  'license',
  'keywords',
] as const;
type SharedField = (typeof SHARED_FIELDS)[number];

export interface ManifestPart {
  readonly value: Readonly<Record<string, unknown>>;
  readonly notes: readonly ConversionNote[];
}

/** The fields Claude Code refuses as the core lets them be, each with why it is left out. */
function refusedFields(manifest: CoreManifest): ReadonlyMap<SharedField, string> {
  const refused = new Map<SharedField, string>();
  if (manifest.author !== undefined && manifest.author.name === undefined) {
    refused.set('author', 'author has no "name", which Claude Code requires: left out');
  }
  const { homepage } = manifest;
  if (homepage !== undefined && !URL.canParse(homepage)) {
    refused.set(
      'homepage',
      `homepage "${homepage}" is not a URL, which Claude Code requires: left out`,
    );
  }
  return refused;
}

export function claudeCodeManifest(manifest: CoreManifest): ManifestPart {
  const refused = refusedFields(manifest);
  const notes = [
    ...nameNotes(manifest.name),
    ...[...refused].map(([field, message]) => notConverted(SOURCE, field, message)),
    ...Object.keys(manifest.extensions ?? {}).map((namespace) =>
      notConverted(
        SOURCE,
        `extensions.${namespace}`,
        'client extension data has no Claude Code counterpart',
      ),
    ),
  ];
  const value = Object.fromEntries(
    SHARED_FIELDS.filter((field) => manifest[field] !== undefined && !refused.has(field)).map(
      (field) => [field, manifest[field]],
    ),
  );
  return { value, notes };
}
