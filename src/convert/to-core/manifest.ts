/**
 * The root `plugin.json` of a Claude Code plugin (Agent Plugins Extended §18.4.3 item 1): the name
 * must satisfy core §5.5 or the conversion stops (§18.4.1 item 2: never renamed); the canonical
 * `$schema` and the nine shared fields are written; every other manifest field is reported. The
 * component keys `skills`, `hooks` and `mcpServers` are converted through the components they load
 * and reported there.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_core/manifest.rs`, docs/RUST.md).
 */
import type { Plugin } from '../../loading/plugin-model.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ConversionNote } from '../types.ts';

const call = coreModule('convert/to-core/manifest');

export const CORE_MANIFEST: string = call('CORE_MANIFEST');

export interface CoreManifestPart {
  readonly value: Readonly<Record<string, unknown>>;
  readonly notes: readonly ConversionNote[];
}

export const coreManifestOf = (plugin: Plugin): CoreManifestPart => call('coreManifestOf', plugin);
