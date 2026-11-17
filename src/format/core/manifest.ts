import { isRecord } from '../../protocol/is-record.ts';
import { defined } from '../defined.ts';
import { blocksLoading, error, warning, type PluginIssue } from '../issues.ts';
import { extensionsProblem, implementedExtensions, type CoreExtensions } from './extensions.ts';
import { coreNameProblems } from './name.ts';
import {
  CORE_VERSIONS,
  coreSchemaVersion,
  supportedCoreVersion,
  type CoreVersion,
} from './versions.ts';

/**
 * A core manifest, root `plugin.json` (core §5): a closed object of eleven fields. An unknown
 * top-level field is reported and ignored (core §5.2); every other violation (a missing or bad
 * `$schema` or `name`, a field of the wrong type, an `author` with another member) rejects the
 * package before any component is discovered (core §5.2, §5.3, §11.3 item 2). Metadata is checked by
 * JSON type alone: a `version` that is not SemVer or a `homepage` that is no URL is accepted (core
 * §5.4).
 */

export interface CoreAuthor {
  readonly name?: string;
  readonly email?: string;
  readonly url?: string;
}

export interface CoreManifest {
  readonly $schema: string;
  readonly name: string;
  readonly displayName?: string;
  readonly version?: string;
  readonly description?: string;
  readonly author?: CoreAuthor;
  readonly homepage?: string;
  readonly repository?: string;
  readonly license?: string;
  readonly keywords?: readonly string[];
  /** Only the namespaces the host implements (core §8.1); absent when none is there. */
  readonly extensions?: CoreExtensions;
}

export interface CoreManifestResult {
  /** `undefined` when the manifest rejects the package. */
  readonly manifest: CoreManifest | undefined;
  /** The Agent Plugins version the manifest targets, when the kit implements it. */
  readonly version: CoreVersion | undefined;
  readonly issues: readonly PluginIssue[];
}

type Field = Exclude<keyof CoreManifest, '$schema' | 'name'>;

const string = (value: unknown): string | undefined =>
  typeof value === 'string' ? undefined : 'must be a string';

const AUTHOR_FIELDS = new Set(['name', 'email', 'url']);

function authorProblem(value: unknown): string | undefined {
  if (!isRecord(value)) return 'must be an object';
  const [key] = Object.keys(value).filter((one) => !AUTHOR_FIELDS.has(one));
  if (key !== undefined) return `may hold only "name", "email" and "url", not "${key}"`;
  const bad = Object.entries(value).find(([, member]) => typeof member !== 'string');
  return bad === undefined ? undefined : `"${bad[0]}" must be a string`;
}

const keywordsProblem = (value: unknown): string | undefined =>
  Array.isArray(value) && value.every((one) => typeof one === 'string')
    ? undefined
    : 'must be an array of strings';

/** The type check of each optional field (core §5.4, §5.6): a registry over the closed field set. */
const FIELD_PROBLEMS: { readonly [K in Field]: (value: unknown) => string | undefined } = {
  displayName: string,
  version: string,
  description: string,
  author: authorProblem,
  homepage: string,
  repository: string,
  license: string,
  keywords: keywordsProblem,
  extensions: extensionsProblem,
};

const isField = (key: string): key is Field => Object.hasOwn(FIELD_PROBLEMS, key);

/** The `$schema` step: the version it targets, or why the package is rejected (core §5.2). */
function schemaStep(value: unknown): {
  readonly version?: CoreVersion;
  readonly issue?: PluginIssue;
} {
  if (value === undefined) return { issue: error('$schema', 'The core manifest has no "$schema"') };
  const named = coreSchemaVersion(value, 'plugin');
  if (named === undefined) {
    return {
      issue: error('$schema', 'Invalid "$schema": not a canonical core manifest identifier'),
    };
  }
  const version = supportedCoreVersion(named);
  return version === undefined
    ? {
        issue: error(
          '$schema',
          `Unsupported Agent Plugins version ${named}: this client implements ${CORE_VERSIONS.join(', ')}`,
        ),
      }
    : { version };
}

function nameIssue(value: unknown): PluginIssue | undefined {
  if (value === undefined) return error('name', 'The core manifest has no "name"');
  if (typeof value !== 'string') return error('name', 'Invalid "name": must be a string');
  const problems = coreNameProblems(value);
  return problems.length === 0
    ? undefined
    : error('name', `Invalid "name" "${value}": ${problems.join('; ')}`);
}

/** Each other top-level member: a warning for an unknown one, an error for a bad value. */
function memberIssues(raw: Readonly<Record<string, unknown>>): PluginIssue[] {
  return Object.entries(raw).flatMap(([key, value]) => {
    if (key === '$schema' || key === 'name') return [];
    if (!isField(key)) return [warning(key, `Unknown field "${key}": ignored (core §5.2)`)];
    const problem = FIELD_PROBLEMS[key](value);
    return problem === undefined ? [] : [error(key, `Invalid "${key}": ${problem}`)];
  });
}

const text = (value: unknown): string | undefined =>
  typeof value === 'string' ? value : undefined;

function authorOf(value: unknown): CoreAuthor | undefined {
  if (!isRecord(value)) return undefined;
  return defined({ name: text(value.name), email: text(value.email), url: text(value.url) });
}

const keywordsOf = (value: unknown): readonly string[] | undefined =>
  Array.isArray(value) ? value.filter((one): one is string => typeof one === 'string') : undefined;

/** The manifest's known fields, once they passed their checks; extensions narrowed to what the host implements. */
function kept(
  raw: Readonly<Record<string, unknown>>,
  implemented: readonly string[],
): CoreManifest {
  return {
    $schema: String(raw.$schema),
    name: String(raw.name),
    ...defined({
      displayName: text(raw.displayName),
      version: text(raw.version),
      description: text(raw.description),
      author: authorOf(raw.author),
      homepage: text(raw.homepage),
      repository: text(raw.repository),
      license: text(raw.license),
      keywords: keywordsOf(raw.keywords),
      extensions: implementedExtensions(raw.extensions, implemented),
    }),
  };
}

/**
 * Reads a parsed root `plugin.json` by core §5. `implemented` names the extension namespaces the
 * host implements; the others are dropped without being looked at (core §8.1).
 */
export function readCoreManifest(
  raw: unknown,
  implemented: readonly string[] = [],
): CoreManifestResult {
  if (!isRecord(raw)) {
    return {
      manifest: undefined,
      version: undefined,
      issues: [error('plugin.json', 'The core manifest is not a JSON object')],
    };
  }
  const schema = schemaStep(raw.$schema);
  const name = nameIssue(raw.name);
  const issues = [schema.issue, name, ...memberIssues(raw)].filter(
    (found): found is PluginIssue => found !== undefined,
  );
  const fatal = blocksLoading(issues);
  return {
    manifest: fatal ? undefined : kept(raw, implemented),
    version: schema.version,
    issues,
  };
}
