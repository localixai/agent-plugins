import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { MarketplaceMetadata, Owner } from './types.ts';
import type { Renames } from './renames.ts';

/**
 * The top level of `marketplace.json` (plugins/marketplace-reference.md, "Top-level fields" and
 * "Validation messages"): every key Claude Code reads, the type of each, and the report for a key
 * that is missing, of the wrong type or unknown. `name` and `owner.name` are checked for their
 * grammar by the caller; `plugins` is handed on entry by entry.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/marketplace_schema.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/marketplace/marketplace-schema');

/** The JSON type of a value, as the "got <type>" and "received <type>" reports name it. */
export const typeName = (value: unknown): string => call('typeName', value);

/** `Invalid input: expected <type>, received <type>`, the one wording of a value of the wrong type. */
export const mismatch = (path: string, expected: string, value: unknown): PluginIssue =>
  call('mismatch', path, expected, value);

/** What a reader returns: the value when it is usable, and the issues it raised. */
export interface Read<T> {
  readonly value: T | undefined;
  readonly issues: readonly PluginIssue[];
}

/** `owner`: an object with a `name` (non-empty) and optional `email` and `url`. */
export const readOwner = (value: unknown): Read<Owner> => call('readOwner', value);

/**
 * The warning for a key Claude Code does not read, at `path`: the docs quote its plain form
 * (`Unknown field '<key>'`); the oracle adds where the key probably comes from (docs/SPEC.md).
 */
export const unknownField = (key: string, path: string, known: readonly string[]): PluginIssue =>
  call('unknownField', key, path, known);

/** The top-level fields of a marketplace, everything but its entries. */
export interface TopLevel {
  readonly name: string;
  readonly owner: Owner;
  readonly plugins: readonly unknown[];
  readonly description?: string;
  readonly version?: string;
  readonly metadata?: MarketplaceMetadata;
  readonly forceRemoveDeletedPlugins: boolean;
  readonly allowCrossMarketplaceDependenciesOn: readonly string[];
  readonly renames: Renames;
}

export interface TopLevelResult {
  /** Undefined when any field breaks the schema: such a marketplace does not load. */
  readonly top: TopLevel | undefined;
  readonly issues: readonly PluginIssue[];
}

/** Reads and checks every top-level key of the parsed `marketplace.json`. */
export const readTopLevel = (raw: Readonly<Record<string, unknown>>): TopLevelResult =>
  call('readTopLevel', raw);
