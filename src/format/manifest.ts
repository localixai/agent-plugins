import type { z } from 'zod';
import { blocksLoading, error, warning, type PluginIssue } from './issues.ts';
import {
  MANIFEST_SCHEMAS,
  isManifestKey,
  manifestSchema,
  userConfigOptionSchema,
  type PluginManifest,
} from './manifest-schema.ts';
import { nameLoadIssues } from './names.ts';
import { checkPluginManifest, manifestPathFindings } from './validate/plugin-manifest.ts';
import { isRecord } from '../protocol/is-record.ts';
import { USER_CONFIG_KEY } from './user-config-refs.ts';
import { note, type PluginNote } from './plugin-notes.ts';
import { enginesFindings } from './engines.ts';
import { networkFindings } from './network.ts';
import { withoutUnreadServerPaths } from './server-paths.ts';

/**
 * Reading `.claude-plugin/plugin.json` as Claude Code loads it (plugins/manifest-reference.md):
 * an unrecognized top-level key is stripped with a warning; a key of the wrong type, an unknown key
 * in a strict object, a bad path or an unusable name is an error, and a manifest with an error does
 * not load ("Validation failed … Claude Code reports the same problem when it loads the plugin").
 */
export interface ManifestResult {
  /** Undefined when an error keeps the plugin from loading. */
  readonly manifest: PluginManifest | undefined;
  readonly issues: readonly PluginIssue[];
  /** What the plugin loads with that `claude plugin list` notes, such as an ignored display field. */
  readonly notes: readonly PluginNote[];
}

/**
 * The display fields: a value of the wrong type is ignored with a note, and the plugin loads
 * (spec §2.1 item 3, Judgment J2.6).
 */
const DISPLAY = new Set([
  '$schema',
  'displayName',
  'description',
  'author',
  'homepage',
  'repository',
  'license',
  'keywords',
]);

/**
 * A malformed value loading ignores without a note: `metadata` or `experimental` that is not an
 * object, or an `icon` that is not a string. Claude Code 2.1.330 loads the plugin (Judgment J2.6);
 * the validator warns about the first two and says nothing of `icon`. A wrong value inside an
 * `experimental` object still fails (§2.2).
 */
const ignoredWhole = (key: string, value: unknown): boolean =>
  key === 'icon' || ((key === 'metadata' || key === 'experimental') && !isRecord(value));

/** `"a"`, `"a" and "b"`, `"a", "b" and "c"`: the kit's joining past two (spec/judgments.md). */
function quotedList(fields: readonly string[]): string {
  const quoted = fields.map((field) => `"${field}"`);
  const last = quoted.pop() ?? '';
  return quoted.length === 0 ? last : `${quoted.join(', ')} and ${last}`;
}

/** The `manifest-field-ignored` note for the display fields loading dropped (§2.1 item 3). */
function ignoredNote(fields: readonly string[]): PluginNote[] {
  if (fields.length === 0) return [];
  const verb = fields.length === 1 ? 'is' : 'are';
  return [
    note(
      'manifest-field-ignored',
      `${quotedList(fields)} in plugin.json ${verb} malformed and ignored. Earlier versions of Claude Code refused this plugin`,
    ),
  ];
}

/** Keys moved under `experimental` that still load at the top level ("Fields"). */
const MOVED = new Set(['themes', 'monitors']);

/** Each problem with the path below the key it is at (`go: Unrecognized key: "foo"`). */
const describe = (failure: z.ZodError): string =>
  failure.issues
    .map((found) =>
      found.path.length > 0 ? `${found.path.join('.')}: ${found.message}` : found.message,
    )
    .join('; ') || 'invalid value';

/** `userConfig` reported per option, so the issue names the option at fault. */
function userConfigIssues(value: unknown): PluginIssue[] {
  if (!isRecord(value)) return [error('userConfig', 'Invalid "userConfig": expected an object')];
  return Object.entries(value).flatMap(([key, option]) => {
    const path = `userConfig.${key}`;
    if (!USER_CONFIG_KEY.test(key)) {
      return [error(path, `Invalid option key "${key}": letters, digits and underscores only`)];
    }
    const parsed = userConfigOptionSchema.safeParse(option);
    return parsed.success
      ? []
      : [error(path, `Invalid option "${key}": ${describe(parsed.error)}`)];
  });
}

/**
 * `engines` and `network` never fail a plugin (spec §2.4, J2.10): what is wrong is a warning, the
 * value is dropped when it is not an object, and an invalid range becomes a note (`engineNotes`).
 */
const LENIENT: Readonly<Record<string, (value: unknown) => PluginIssue[]>> = {
  engines: (value) => (isRecord(value) ? [] : enginesFindings(value)),
  network: networkFindings,
};

function keyIssues(key: string, value: unknown): PluginIssue[] {
  if (!isManifestKey(key)) return [warning(key, `Unknown field "${key}"`)];
  const lenient = LENIENT[key];
  if (lenient !== undefined) return lenient(value);
  const parsed = MANIFEST_SCHEMAS[key].safeParse(value);
  if (!parsed.success) {
    return key === 'userConfig'
      ? userConfigIssues(value)
      : [error(key, `Invalid "${key}": ${describe(parsed.error)}`)];
  }
  return MOVED.has(key) ? [warning(key, `"${key}" belongs under "experimental"`)] : [];
}

/** One key's issues as loading keeps them: a display or ignored key's error is not one. */
function loadIssues(
  key: string,
  value: unknown,
  found: readonly PluginIssue[],
): readonly PluginIssue[] {
  if (DISPLAY.has(key) && blocksLoading(found)) return [];
  if (ignoredWhole(key, value) && blocksLoading(found)) {
    return found.map((issue) => ({ ...issue, severity: 'warning' }));
  }
  return found;
}

interface KeptKeys {
  readonly kept: Readonly<Record<string, unknown>>;
  readonly issues: readonly PluginIssue[];
  readonly dropped: readonly string[];
}

function keepKeys(raw: Readonly<Record<string, unknown>>): KeptKeys {
  const issues: PluginIssue[] = [];
  const kept: Record<string, unknown> = {};
  const dropped: string[] = [];
  for (const [key, value] of Object.entries(raw)) {
    const found = keyIssues(key, value);
    if (DISPLAY.has(key) && blocksLoading(found)) dropped.push(key);
    issues.push(...loadIssues(key, value, found));
    const droppedLenient = Object.hasOwn(LENIENT, key) && !isRecord(value);
    if (isManifestKey(key) && !blocksLoading(found) && !droppedLenient) kept[key] = value;
  }
  return { kept, issues, dropped };
}

export function readManifest(raw: unknown): ManifestResult {
  if (!isRecord(raw)) {
    return {
      manifest: undefined,
      issues: [error('plugin.json', 'The manifest is not a JSON object')],
      notes: [],
    };
  }
  const unread = withoutUnreadServerPaths(raw);
  const { kept, dropped, ...found } = keepKeys(unread.manifest);
  const issues = [...unread.issues, ...found.issues];
  const notes = ignoredNote(dropped);
  if (typeof kept.name !== 'string') {
    if (!issues.some((issue) => issue.path === 'name')) {
      issues.push(error('name', 'The manifest has no "name"'));
    }
    return { manifest: undefined, issues, notes };
  }
  issues.push(...nameLoadIssues(kept.name));
  // Every kept key passed its own schema, so the whole parses; this types it without a cast.
  const parsed = manifestSchema.safeParse(kept);
  if (blocksLoading(issues) || !parsed.success) return { manifest: undefined, issues, notes };
  issues.push(...manifestPathFindings(kept));
  return { manifest: blocksLoading(issues) ? undefined : parsed.data, issues, notes };
}

/**
 * `claude plugin validate` on a parsed `plugin.json`: the validator's findings and what the inline
 * `hooks` and `mcpServers` hold (`checkPluginManifest`, the one rule set).
 */
export function validateManifest(raw: unknown): PluginIssue[] {
  const { findings, inline } = checkPluginManifest(raw);
  return [...findings, ...inline];
}
