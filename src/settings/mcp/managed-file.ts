/**
 * Where Claude Code looks for `managed-mcp.json` (managed-mcp.md, "Deploy managed-mcp.json"): "Linux
 * and WSL" share a row, so the platform union names both and the registry gives them one path.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/managed_file.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { PluginMcpServer } from '../../format/mcp-config.ts';
import type { TextFileReader } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/managed-file');

export const MANAGED_MCP_PLATFORMS: readonly ['macos', 'linux', 'wsl', 'windows'] =
  call('MANAGED_MCP_PLATFORMS');
export type ManagedMcpPlatform = (typeof MANAGED_MCP_PLATFORMS)[number];

export const managedMcpFilePath = (platform: ManagedMcpPlatform): string =>
  call('managedMcpFilePath', platform);

/** The servers a deployed `managed-mcp.json` defines, already read like a project `.mcp.json`. */
export interface ManagedMcpFile {
  readonly servers: Readonly<Record<string, PluginMcpServer>>;
}

export interface ManagedMcpFileRead {
  /** Undefined when there is no file, or Claude Code "can't read or parse" it: no exclusive control then. */
  readonly file: ManagedMcpFile | undefined;
  readonly issues: readonly PluginIssue[];
}

/**
 * The file's text: "the same format as a project `.mcp.json` file" (a `mcpServers` wrapper; the
 * reader of `.mcp.json` also takes the bare map). Only a file Claude Code "can read and parse" takes
 * exclusive control, so text that is no JSON object is an issue and no file. An entry Claude Code
 * would drop is an issue and the others stay (spec/judgments.md for the wording and the empty file).
 */
export const readManagedMcpFile = (text: string | undefined): ManagedMcpFileRead =>
  call('readManagedMcpFile', text);

/** Reads the file of `platform` through the host's reader; a machine without it has no file. */
export const loadManagedMcpFile = (
  reader: TextFileReader,
  platform: ManagedMcpPlatform,
  options: { readonly signal?: AbortSignal | undefined } = {},
): Promise<ManagedMcpFileRead> => call.async('loadManagedMcpFile', reader, platform, options);
