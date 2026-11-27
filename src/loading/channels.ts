import type { PluginIssue } from '../format/issues.ts';
import type { PluginManifest } from '../format/manifest-schema.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * A `channels` entry is "bound to one of the plugin's MCP servers": `server` is the "Key of the MCP
 * server in this plugin's `mcpServers` that the channel binds to" (plugins/manifest-reference.md,
 * "Channels"). The keys are those loading found, from `.mcp.json`, a file or the manifest.
 * Judgment: an entry bound to no server is an error issue, and the plugin still loads without that
 * channel (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/channels.rs`, docs/RUST.md).
 */
const call = coreModule('loading/channels');

export const channelBindingMessage = (server: string): string =>
  call('channelBindingMessage', server);

export const channelIssues = (
  manifest: PluginManifest | undefined,
  servers: Readonly<Record<string, unknown>>,
): PluginIssue[] => call('channelIssues', manifest, servers);
