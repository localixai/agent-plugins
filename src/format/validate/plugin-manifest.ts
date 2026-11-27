import type { Finding } from './finding.ts';
import type { NameOptions } from './plugin-name.ts';
import type { PathClaim } from './path-claims.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * `claude plugin validate` on a parsed `plugin.json` (oracle 2.1.319): what is wrong with each
 * key's type, the name, the keys it does not know, and what is missing. Whether the paths it
 * names exist is for the caller, which has the files: `claims` lists them.
 *
 * The validator parses the manifest against its schema first. A manifest that fails it is still
 * checked for unknown keys and paths, but not for what is missing or how its name reads, and the
 * hooks and MCP servers written inline are not read at all: `inline` is empty then.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/plugin_manifest.rs`,
 * docs/RUST.md).
 */
const call = coreModule('format/validate/plugin-manifest');

export interface ManifestChecks {
  readonly findings: readonly Finding[];
  /** What the inline `hooks` and `mcpServers` hold; for the plugin's own file, not for a marketplace entry. */
  readonly inline: readonly Finding[];
  readonly claims: readonly PathClaim[];
}

export interface ManifestOptions extends NameOptions {
  /** The name of the marketplace entry this manifest belongs to: a name it already reported is not reported twice. */
  readonly entryName?: string;
}

export const checkPluginManifest = (raw: unknown, options: ManifestOptions = {}): ManifestChecks =>
  call('checkPluginManifest', raw, options);
