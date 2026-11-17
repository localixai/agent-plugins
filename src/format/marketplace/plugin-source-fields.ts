import { error, type PluginIssue } from '../issues.ts';
import type { ReadMode } from './types.ts';

/**
 * The field checks every object plugin source shares (plugins/marketplace-reference.md, "Plugin
 * sources" and "Invalid input on a source"): a known type "with a required field missing or of the
 * wrong type" is `Invalid input` on the entry's `source`.
 */

/** Where a source is being read: the path its issues name, and what bare names and `\` depend on. */
export interface SourceSite {
  readonly path: string;
  readonly pluginRoot?: string;
  readonly platform: 'posix' | 'windows';
  /** `validate` leaves out what only installing or loading refuses (a backslash path). */
  readonly mode?: ReadMode;
}

export const invalidInput = (site: SourceSite): PluginIssue[] => [
  error(site.path, 'Invalid input'),
];

/** A full 40-character lowercase commit SHA ("Plugin sources": `sha`). */
const COMMIT_SHA = /^[0-9a-f]{40}$/;
const SHA_LENGTH = 40;
/** The archive's digest: 64 hex characters, uppercase or lowercase ("archive plugin source"). */
const ARCHIVE_DIGEST = /^[0-9A-Fa-f]{64}$/;

export type RawSource = Readonly<Record<string, unknown>>;

export const isText = (value: unknown): value is string =>
  typeof value === 'string' && value !== '';

const isString = (value: unknown): value is string => typeof value === 'string';

/** An optional field: `value` is `undefined` when the key is absent. A present value of the wrong type is no value at all. */
export interface Optional {
  readonly value: string | undefined;
}

/**
 * A source of the right shape, or the issues of its fields. A field present, of the right type and
 * breaking its rule is reported on that field (spec §15.6.7: "a field present with an invalid value
 * names that field"); the wording is oracle 2.1.325 (spec/judgments.md).
 */
export interface Checked<T> {
  readonly value: T;
  readonly issues: readonly PluginIssue[];
}

/** `issues` when there are any, else the source. */
export const refined = <S>(source: S, issues: readonly PluginIssue[]): S | readonly PluginIssue[] =>
  issues.length > 0 ? issues : source;

export const fieldError = (site: SourceSite, key: string, message: string): PluginIssue =>
  error(`${site.path}.${key}`, message);

/** A required text field: the text, or `undefined` when the key is missing, empty or not a string. */
export function required(raw: RawSource, key: string): string | undefined {
  const value = raw[key];
  return isText(value) ? value : undefined;
}

function optional(
  raw: RawSource,
  key: string,
  accepts: (value: unknown) => value is string,
): Optional | undefined {
  const value = raw[key];
  if (value === undefined) return { value: undefined };
  return accepts(value) ? { value } : undefined;
}

export const optionalText = (raw: RawSource, key: string): Optional | undefined =>
  optional(raw, key, isText);

/** `sha256`: a string, else `undefined`; one that is not 64 hex characters is an issue on the field. */
export function optionalDigest(
  raw: RawSource,
  site: SourceSite,
): Checked<string | undefined> | undefined {
  const digest = optional(raw, 'sha256', isString);
  if (digest === undefined) return undefined;
  const broken = digest.value !== undefined && !ARCHIVE_DIGEST.test(digest.value);
  return {
    value: digest.value,
    issues: broken ? [fieldError(site, 'sha256', 'Must be a 64-character hex SHA-256 digest')] : [],
  };
}

/** A `sha` of the wrong length is also reported as too short or too long, before the SHA message. */
function shaIssues(sha: string, site: SourceSite): PluginIssue[] {
  if (COMMIT_SHA.test(sha)) return [];
  const length =
    sha.length === SHA_LENGTH
      ? []
      : [
          fieldError(
            site,
            'sha',
            `Too ${sha.length < SHA_LENGTH ? 'small' : 'big'}: expected string to have exactly ${SHA_LENGTH} characters`,
          ),
        ];
  return [
    ...length,
    fieldError(site, 'sha', 'Must be a full 40-character lowercase git commit SHA'),
  ];
}

export interface GitPin {
  readonly ref: string | undefined;
  readonly sha: string | undefined;
}

/** The `ref` and `sha` the git sources share, or `undefined` when either is present and of the wrong type. */
export function readGitPin(raw: RawSource, site: SourceSite): Checked<GitPin> | undefined {
  const ref = optionalText(raw, 'ref');
  const sha = optional(raw, 'sha', isString);
  if (ref === undefined || sha === undefined) return undefined;
  return {
    value: { ref: ref.value, sha: sha.value },
    issues: sha.value === undefined ? [] : shaIssues(sha.value, site),
  };
}
