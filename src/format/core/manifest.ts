/**
 * A core manifest, root `plugin.json` (core §5): a closed object of eleven fields. An unknown
 * top-level field is reported and ignored (core §5.2); every other violation (a missing or bad
 * `$schema` or `name`, a field of the wrong type, an `author` with another member) rejects the
 * package before any component is discovered (core §5.2, §5.3, §11.3 item 2). Metadata is checked by
 * JSON type alone: a `version` that is not SemVer or a `homepage` that is no URL is accepted (core
 * §5.4).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/manifest.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { CoreExtensions } from './extensions.ts';
import type { CoreVersion } from './versions.ts';

const call = coreModule('format/core/manifest');

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

/**
 * Reads a parsed root `plugin.json` by core §5. `implemented` names the extension namespaces the
 * host implements; the others are dropped without being looked at (core §8.1).
 */
export const readCoreManifest = (
  raw: unknown,
  implemented: readonly string[] = [],
): CoreManifestResult => call('readCoreManifest', raw, implemented);
