import type { ValidationIssue } from '../../format/validate/finding.ts';
import type { ManifestOptions } from '../../format/validate/plugin-manifest.ts';
import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * Validating a plugin folder: its manifest `.claude-plugin/plugin.json`, whether the paths it
 * claims exist, then the folder's contents.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/plugin_target.rs`,
 * docs/RUST.md).
 */
const call = coreModule('loading/validate/plugin-target');

export const MANIFEST_FILE: string = call('MANIFEST_FILE');

/** Validates the plugin whose manifest is `.claude-plugin/plugin.json`: the manifest, then its contents. */
export const validatePluginFolder = (
  files: PluginFiles,
  options: ManifestOptions = {},
  signal?: AbortSignal,
): Promise<ValidationIssue[]> => call.async('validatePluginFolder', files, options, signal);
