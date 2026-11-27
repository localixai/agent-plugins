import type { PluginIssue } from '../format/issues.ts';
import type { CorePackage } from '../loading/core/index.ts';
import type { ResolvedMcpServer } from '../loading/index.ts';
import type { Environment, Locations } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ActivePlugin } from './state.ts';

/**
 * A core package's MCP servers in the session's one tool namespace (Agent Plugins Extended §18.1.4,
 * J18.10, J18.11): registered under `plugin:<name>:<server>`, so their tools are
 * `mcp__plugin_<name>_<server>__<tool>` for the model, the permission rules, the hook matchers and
 * the policy lists. The loader validated each entry by core §7.2; here a valid one becomes a Claude
 * Code plugin server of the same transport with no optional field set: `stdio` as is, with its
 * `./` command resolved, `args`, `env` and `cwd` expanded (core §9.2) and the `cwd` contained;
 * `streamable-http` as `http`; `sse` as `sse`; `url` and `headers` never expanded. The process gets
 * `PLUGIN_ROOT`, `PLUGIN_DATA` and `CLAUDE_PROJECT_DIR` (§18.1.3, J18.12), no `CLAUDE_PLUGIN_*`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/core_servers.rs`, docs/RUST.md).
 */

export interface CorePlace {
  readonly locations: Locations;
  readonly environment: Environment;
  readonly signal?: AbortSignal;
}

const call = coreModule('host/core-servers');

/** The servers of one core package, in the order `mcp.json` names them. */
export const coreServers = (
  plugin: ActivePlugin,
  core: CorePackage,
  place: CorePlace,
): Promise<{
  readonly resolved: readonly ResolvedMcpServer[];
  readonly issues: readonly PluginIssue[];
}> => call.async('coreServers', { id: plugin.id }, { mcpServers: core.mcpServers }, place);
