/**
 * `mcp.json` and `hooks.json` of a core package, read from their fixed locations and then held to
 * the filesystem: a `./` command must resolve to a regular file inside the plugin root, and a `cwd`
 * rooted in the plugin root must stay inside it through any link (core §4.1, boundaries 4 and 5).
 * A server or handler that fails is skipped and reported; the rest loads.
 *
 * Judgment (spec/judgments.md): a `./` command that names no file fails command resolution, so the
 * entry is invalid at load (core §7.3.2 item 4), not at launch; a `cwd` that does not exist yet is
 * no load problem, since only its containment is a configuration rule.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/config.rs`, docs/RUST.md).
 */
import type { CoreHooks } from '../../format/core/hooks.ts';
import type { CoreMcpOptions } from '../../format/core/mcp.ts';
import type { CoreMcpServer } from '../../format/core/mcp-server.ts';
import type { CoreVersion } from '../../format/core/versions.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { LocateContext } from '../../format/locate.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/core/config');

export interface CoreMcpRead {
  readonly servers: Readonly<Record<string, CoreMcpServer>>;
  readonly issues: readonly PluginIssue[];
}

/** The servers of `mcp.json` that are valid and contained. */
export const readCoreMcpFile = (
  context: LocateContext,
  version: CoreVersion,
  options: CoreMcpOptions,
): Promise<CoreMcpRead> => call.async('readCoreMcpFile', context, version, options);

export interface CoreHooksRead {
  readonly hooks: CoreHooks;
  readonly issues: readonly PluginIssue[];
}

/** The handlers of `hooks.json` that are valid and whose `./` command resolves inside the root. */
export const readCoreHooksFile = (
  context: LocateContext,
  version: CoreVersion,
  implemented: readonly string[],
): Promise<CoreHooksRead> => call.async('readCoreHooksFile', context, version, implemented);
