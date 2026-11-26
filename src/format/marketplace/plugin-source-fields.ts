/**
 * The field checks every object plugin source shares (plugins/marketplace-reference.md, "Plugin
 * sources" and "Invalid input on a source"): a known type "with a required field missing or of the
 * wrong type" is `Invalid input` on the entry's `source`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/plugin_source_fields.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { ReadMode } from './types.ts';

const call = coreModule('format/marketplace/plugin-source-fields');

/** Where a source is being read: the path its issues name, and what bare names and `\` depend on. */
export interface SourceSite {
  readonly path: string;
  readonly pluginRoot?: string;
  readonly platform: 'posix' | 'windows';
  /** `validate` leaves out what only installing or loading refuses (a backslash path). */
  readonly mode?: ReadMode;
}

export const invalidInput = (site: SourceSite): PluginIssue[] => call('invalidInput', site);

export type RawSource = Readonly<Record<string, unknown>>;

export const isText = (value: unknown): value is string => call('isText', value);

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
  call('refined', source, issues);

export const fieldError = (site: SourceSite, key: string, message: string): PluginIssue =>
  call('fieldError', site, key, message);

/** A required text field: the text, or `undefined` when the key is missing, empty or not a string. */
export const required = (raw: RawSource, key: string): string | undefined =>
  call('required', raw, key);

export const optionalText = (raw: RawSource, key: string): Optional | undefined =>
  call('optionalText', raw, key);

/** `sha256`: a string, else `undefined`; one that is not 64 hex characters is an issue on the field. */
export const optionalDigest = (
  raw: RawSource,
  site: SourceSite,
): Checked<string | undefined> | undefined => call('optionalDigest', raw, site);

export interface GitPin {
  readonly ref: string | undefined;
  readonly sha: string | undefined;
}

/** The `ref` and `sha` the git sources share, or `undefined` when either is present and of the wrong type. */
export const readGitPin = (raw: RawSource, site: SourceSite): Checked<GitPin> | undefined =>
  call('readGitPin', raw, site);
