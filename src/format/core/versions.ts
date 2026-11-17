import { isRecord } from '../../protocol/is-record.ts';

/**
 * The core's schema identifiers (Agent Plugins 1.1.0, core §5.2, §7.2.1, §7.3.1, §10.1): every
 * document of a release names its schema by a canonical identifier that carries the release's
 * version, `https://agent-plugins.org/schemas/<version>/<document>.schema.json`. The kit selects its
 * local rules from that identifier and never retrieves a schema.
 */

/** The three documents the core defines a schema for (core §10.1). */
export const CORE_DOCUMENTS = ['plugin', 'mcp', 'hooks'] as const;
export type CoreDocument = (typeof CORE_DOCUMENTS)[number];

/**
 * The Agent Plugins versions the kit implements. 1.0.0 is not mapped to 1.1.0: its manifest has no
 * `displayName`, its servers no `extensions` and it has no `hooks.json`, so the kit does not
 * recognise the two as compatible (core §5.2 allows a mapping only when it is explicit).
 */
export const CORE_VERSIONS = ['1.1.0'] as const;
export type CoreVersion = (typeof CORE_VERSIONS)[number];

/** A canonical identifier: the version is any SemVer, so an unsupported release is still recognised as core. */
const IDENTIFIER =
  /^https:\/\/agent-plugins\.org\/schemas\/(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)\/(plugin|mcp|hooks)\.schema\.json$/;

/** The canonical identifier of a document of a release. */
export const coreSchemaId = (version: CoreVersion, document: CoreDocument): string =>
  `https://agent-plugins.org/schemas/${version}/${document}.schema.json`;

/** The version a `$schema` value names for `document`, or `undefined` when it is no canonical identifier of it. */
export function coreSchemaVersion(value: unknown, document: CoreDocument): string | undefined {
  if (typeof value !== 'string') return undefined;
  const match = IDENTIFIER.exec(value);
  return match?.[2] === document ? match[1] : undefined;
}

/** The implemented version a version string is, or `undefined` when the kit does not implement it. */
export const supportedCoreVersion = (version: string | undefined): CoreVersion | undefined =>
  CORE_VERSIONS.find((known) => known === version);

/**
 * Whether a root `plugin.json` value makes the folder a core package (spec §1.3): a JSON object whose
 * `$schema` is a canonical core manifest identifier of any release (Judgment J1.1: anything else is
 * an ordinary file of a Claude Code plugin).
 */
export const isCoreManifestValue = (raw: unknown): boolean =>
  isRecord(raw) && coreSchemaVersion(raw.$schema, 'plugin') !== undefined;
