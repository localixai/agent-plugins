import { error } from '../issues.ts';
import type { EntrySlot } from './marketplace.ts';
import {
  OBJECT_PARSERS,
  type ObjectKind,
  type ObjectParser,
  type Parsed,
} from './plugin-source-kinds.ts';
import { invalidInput, type SourceSite } from './plugin-source-fields.ts';
import type { SourceReading } from './entries.ts';
import type { ReadMode } from './types.ts';
import { isRecord } from '../../protocol/is-record.ts';
import { isAbsolutePath, rootRelative } from '../path-text.ts';

/**
 * Reading a plugin entry's `source` (plugins/marketplace-reference.md, "Plugin sources", "Relative
 * path plugin source", "Bare names under pluginRoot", "Validation messages"): a relative path
 * string, or an object whose own `source` key names one of the other six types. It is the
 * `SourceParser` the entry reader is given (`createEntryReader(parsePluginSource)`).
 */

const MUST_START = 'Invalid string: must start with "./"';

/** The line an entry whose path does not stay inside the marketplace is listed with (`errors.md`). */
const entryPathRefused = (path: string): string =>
  `Plugin source path refused: ${path} does not stay inside its marketplace directory. Check that the marketplace entry has a plain relative path.`;

/** The tag `source: 'unsupported'` is the SDK's placeholder, not an authorable type. */
const PLACEHOLDER_MESSAGE =
  "source.source: 'unsupported' is a parse-time placeholder and cannot be authored";
const PLACEHOLDER_TAG = 'unsupported';

const isObjectKind = (tag: string): tag is ObjectKind => Object.hasOwn(OBJECT_PARSERS, tag);

const outcome = (parsed: Parsed): SourceReading =>
  'kind' in parsed ? { source: parsed, issues: [] } : { source: undefined, issues: parsed };

/**
 * `metadata.pluginRoot` "must itself be a relative path inside the marketplace": no `..`, not
 * absolute, not empty. Written with or without `./` (spec/judgments.md).
 */
function rootPath(pluginRoot: string, site: SourceSite): string | undefined {
  const path = site.platform === 'windows' ? pluginRoot.replaceAll('\\', '/') : pluginRoot;
  const bad = path === '' || path.includes('..') || path.includes('\\') || isAbsolutePath(path);
  return bad ? undefined : rootRelative(path);
}

const unresolvable = (name: string, pluginRoot: string): string =>
  `Bare source name "${name}" cannot resolve: metadata.pluginRoot ("${pluginRoot}") must be a relative path inside the marketplace (e.g. "./plugins").`;

/** A bare name: "a single directory name with no `/`", resolved under `metadata.pluginRoot`. */
function bareName(raw: string, site: SourceSite): SourceReading {
  const root = site.pluginRoot === undefined ? undefined : rootPath(site.pluginRoot, site);
  if (site.pluginRoot === undefined || raw.includes('/') || raw === '') {
    return { source: undefined, issues: [error(site.path, MUST_START)] };
  }
  // A root that climbs out or is absolute leaves the bare name without a base: oracle 2.1.330
  // reports the bare name like one without `./`, and why it cannot resolve (spec J15.6).
  if (root === undefined) {
    return {
      source: undefined,
      issues: [error(site.path, MUST_START), error(site.path, unresolvable(raw, site.pluginRoot))],
    };
  }
  const path = root === '.' ? raw : `${root}/${raw}`;
  return { source: { kind: 'relative', path, bare: true }, issues: [] };
}

function relative(raw: string, site: SourceSite): SourceReading {
  if (raw.includes('..')) {
    return { source: undefined, issues: [error(site.path, `Path contains "..": ${raw}`)] };
  }
  if (raw === '.' || raw.startsWith('./')) return withSeparators(raw, site);
  return bareName(raw, site);
}

/**
 * On macOS and Linux a backslash after the leading `./` is refused by the client at install and at
 * load, and "validation does not report it" (spec §15.6.1 items 2 and 4): validating keeps the path
 * as written; loading refuses the entry with the load line of `errors.md`, "Marketplace entry path
 * does not stay inside the marketplace directory". On Windows a backslash is a separator.
 */
const REFUSES_BACKSLASH: { readonly [M in ReadMode]: boolean } = { load: true, validate: false };

function withSeparators(raw: string, site: SourceSite): SourceReading {
  const rest = raw === '.' ? '' : raw.slice(2);
  if (site.platform === 'posix' && rest.includes('\\') && REFUSES_BACKSLASH[site.mode ?? 'load']) {
    return { source: undefined, issues: [error(site.path, entryPathRefused(raw))] };
  }
  const path = rootRelative(rest, site.platform);
  return { source: { kind: 'relative', path, bare: false }, issues: [] };
}

function fromObject(raw: Readonly<Record<string, unknown>>, site: SourceSite): SourceReading {
  const tag = raw.source;
  if (tag === PLACEHOLDER_TAG) {
    return { source: undefined, issues: [error(site.path, PLACEHOLDER_MESSAGE)] };
  }
  if (typeof tag !== 'string' || !isObjectKind(tag)) {
    return { source: undefined, issues: invalidInput(site) };
  }
  const parser: ObjectParser = OBJECT_PARSERS[tag];
  return outcome(parser(raw, site));
}

/** The plugin source of `plugins[i]`: the source, or `undefined` with the issues that say why. */
export function parsePluginSource(raw: unknown, slot: EntrySlot): SourceReading {
  const site: SourceSite = {
    path: `plugins[${slot.index}].source`,
    platform: slot.context.platform ?? 'posix',
    mode: slot.mode ?? 'load',
    ...(slot.pluginRoot === undefined ? {} : { pluginRoot: slot.pluginRoot }),
  };
  if (typeof raw === 'string') return relative(raw, site);
  if (isRecord(raw)) return fromObject(raw, site);
  return { source: undefined, issues: invalidInput(site) };
}
