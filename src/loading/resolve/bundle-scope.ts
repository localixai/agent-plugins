/**
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/bundle_scope.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { McpBundleConfig } from '../../format/mcp-config.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { SubstitutionScope } from './scope.ts';

const call = coreModule('loading/resolve/bundle-scope');

/**
 * The scope of a bundled server, or why it does not start (plugins/components.md, "Include a packaged
 * MCPB server"). Its `${user_config.KEY}` reads its bundle's own settings, never the plugin's
 * `userConfig`: the values saved under `<server>.<key>` (`pluginConfigs.<id>.mcpServers.<server>`, the
 * host's store over it; the `--config` form), else the setting's default, answered in the same one
 * pass as every other reference, so a value holding `${...}` stays that text. The verdict is made here,
 * when the server is resolved for a session, so one loaded plugin serves sessions with different
 * values; a value that is not text, a number or a Boolean is none (judgments, spec/judgments.md).
 */
export const bundleScope = (
  scope: SubstitutionScope,
  server: string,
  bundle: McpBundleConfig,
): { readonly scope: SubstitutionScope } | { readonly issue: PluginIssue } =>
  call('bundleScope', scope, server, bundle);
