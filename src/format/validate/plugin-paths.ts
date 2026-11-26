import { coreModule } from '../../wasm/bridge.ts';
import type { Finding } from './finding.ts';

/**
 * The shape rules of the manifest keys that name component paths (oracle 2.1.319): every path
 * starts with `./`; a key that names one kind of file also fixes its extension. A value of the
 * wrong shape is a bare "Invalid input" at the key; a path that breaks a rule is reported once per
 * rule, at the key (a single string) or at the entry (`skills.0`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/plugin_paths.rs`,
 * docs/RUST.md). The table of shapes, `PATH_SHAPES`, holds functions and so stays in the core:
 * `pathShapeFindings` takes the key.
 */
const call = coreModule('format/validate/plugin-paths');

export type PathKey =
  | 'skills'
  | 'commands'
  | 'agents'
  | 'hooks'
  | 'mcpServers'
  | 'lspServers'
  | 'outputStyles'
  | 'workflows'
  | 'themes'
  | 'monitors'
  | 'types';
export const isPathKey = (key: string): key is PathKey => call('isPathKey', key);

/** The shape findings of one path-valued key; `path` is what the oracle prints for it. */
export const pathShapeFindings = (path: string, key: PathKey, value: unknown): Finding[] =>
  call('pathShapeFindings', path, key, value);
