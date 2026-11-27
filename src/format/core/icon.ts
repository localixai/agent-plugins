import type { DirectoryEntry } from '../../ports/files.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * Plugin icon candidates (core §4.3.1): a regular file directly in the root `assets/` directory whose
 * whole name is lowercase, `icon` and one image-format extension the client recognises. PNG is the
 * one format every displaying client supports, so it is always recognised. The kit only finds the
 * candidates: reading, decoding and rendering them, and making SVG inert, are the host's, and an
 * unusable candidate never affects the package (core §4.3.1).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/icon.rs`, docs/RUST.md).
 */

const call = coreModule('format/core/icon');

/**
 * The candidates among the entries of `assets/`, as root-relative paths: PNG first, then the other
 * recognised formats in the order `formats` gives them (judgment: a client may pick any).
 */
export const iconCandidates = (
  entries: readonly DirectoryEntry[],
  formats: readonly string[] = [],
): readonly string[] => call('iconCandidates', entries, formats);
