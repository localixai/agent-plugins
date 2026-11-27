import type { PluginIssue } from './issues.ts';
import type { PluginManifest } from './manifest-schema.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Reading `.claude-plugin/plugin.json` as Claude Code loads it (plugins/manifest-reference.md):
 * an unrecognized top-level key is stripped with a warning; a key of the wrong type, an unknown key
 * in a strict object, a bad path or an unusable name is an error, and a manifest with an error does
 * not load ("Validation failed … Claude Code reports the same problem when it loads the plugin").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/manifest.rs`, docs/RUST.md).
 */
const call = coreModule('format/manifest');

export interface ManifestResult {
  /** Undefined when an error keeps the plugin from loading. */
  readonly manifest: PluginManifest | undefined;
  readonly issues: readonly PluginIssue[];
}

export const readManifest = (raw: unknown): ManifestResult => call('readManifest', raw);

/**
 * `claude plugin validate` on a parsed `plugin.json`: the validator's findings and what the inline
 * `hooks` and `mcpServers` hold (`checkPluginManifest`, the one rule set).
 */
export const validateManifest = (raw: unknown): PluginIssue[] => call('validateManifest', raw);
