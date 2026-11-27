import { coreModule } from '../../wasm/bridge.ts';
import type { Finding } from './finding.ts';
import type { PathKey } from './plugin-paths.ts';

/**
 * The keys whose inline form is a strict object the validator reads entry by entry (spec §9.1.3,
 * §9.2.2; oracle 2.1.325): the `lspServers` map and the monitors array, at the top level and under
 * `experimental`. The loader reads the same schemas through the manifest's (manifest-schema.ts).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/inline_sources.rs`,
 * docs/RUST.md).
 */
const call = coreModule('format/validate/inline-sources');

/**
 * The findings of one path-valued key: an inline LSP map or monitors array that breaks its schema
 * is reported at the path of each problem (`lspServers.go: Unrecognized key: "foo"`,
 * `experimental.monitors: Monitor names must be unique within a plugin`), or as `Invalid input` at
 * the key when no form fits; otherwise the path rules of `plugin-paths.ts`.
 */
export const sourceFindings = (key: PathKey, path: string, value: unknown): Finding[] =>
  call('sourceFindings', key, path, value);
