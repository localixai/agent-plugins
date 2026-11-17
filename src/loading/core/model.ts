import type { CoreHooks } from '../../format/core/hooks.ts';
import type { CoreManifest } from '../../format/core/manifest.ts';
import type { CoreMcpServer, CoreTransport } from '../../format/core/mcp-server.ts';
import type { CoreVersion } from '../../format/core/versions.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { PluginFiles } from '../../ports/index.ts';
import type { PluginTextFile } from '../plugin-model.ts';

/**
 * A core package as the kit loads it (Agent Plugins 1.1.0, core §4–§10): the manifest, the three
 * portable component types from their fixed locations, the icon candidates and the extension
 * directories of the namespaces the host implements. Every path is relative to the plugin root.
 */
export interface CorePackage {
  readonly name: string;
  /** The Agent Plugins version the package targets (core §10.1). */
  readonly version: CoreVersion;
  readonly manifest: CoreManifest;
  /** One per `skills/<directory>/SKILL.md` that conforms to Agent Skills; `name` is the directory (core §7.1). */
  readonly skills: readonly PluginTextFile[];
  readonly mcpServers: Readonly<Record<string, CoreMcpServer>>;
  readonly hooks: CoreHooks;
  /** Icon candidates under `assets/`, PNG first (core §4.3.1). */
  readonly icons: readonly string[];
  /** The top-level directories named for an implemented extension namespace (core §8.2). */
  readonly extensionDirectories: readonly string[];
}

export interface LoadedCorePackage {
  /** `undefined` when the package is rejected: its `plugin.json` is missing, outside the root or invalid (core §4.1, §5). */
  readonly package: CorePackage | undefined;
  readonly issues: readonly PluginIssue[];
}

export interface CoreLoadOptions {
  /** The extension namespaces the host implements (core §8): their data is kept, their directories found. */
  readonly extensions?: readonly string[];
  /** The MCP transports the host supports; all three by default. */
  readonly transports?: readonly CoreTransport[];
  /** Icon formats the host recognises besides PNG, as extensions (`svg`, `webp`). */
  readonly iconFormats?: readonly string[];
}

export interface LoadCorePackageInput extends CoreLoadOptions {
  /** Read access to the package: its root is the plugin root. */
  readonly files: PluginFiles;
  readonly signal?: AbortSignal;
}
