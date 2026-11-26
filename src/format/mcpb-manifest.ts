import { coreModule } from '../wasm/bridge.ts';
import type { UserConfigOption } from './manifest-schema.ts';
/**
 * What an MCPB bundle's own `manifest.json` says (plugins/components.md, "Include a packaged MCPB
 * server": "The server takes its name from the `name` in the bundle's manifest" and "A bundle's own
 * manifest can declare settings the server needs from the user in a `user_config` block"). The format
 * itself lives at github.com/modelcontextprotocol/mcpb, not in the Claude Code docs, so every rule here is
 * a judgment (spec/judgments.md). Pure data: extracting or downloading the archive is the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcpb_manifest.rs`, docs/RUST.md).
 */
const call = coreModule('format/mcpb-manifest');

export interface McpbManifest {
  /** The server's name: the manifest's `name`. */
  readonly name: string;
  /** The `server.mcp_config` (or the one an `entry_point` stands for) for this platform; `${...}` unresolved. */
  readonly server: Readonly<Record<string, unknown>>;
  /**
   * The `user_config` block by key, in the plugin option shape: a bundle setting is the same concept
   * as a plugin's `userConfig` option (plugins/components.md: "Claude Code prompts for them alongside
   * the plugin's own options"). `required` is the bundle's own mark: a setting with a default of its
   * own still starts the server (`requiredBundleSettings`).
   */
  readonly options: Readonly<Record<string, UserConfigOption>>;
}

export type McpbRead = { readonly manifest: McpbManifest } | { readonly problem: string };

/**
 * A bundle's `user_config` block in the plugin option shape. Judgments (spec/judgments.md): an entry
 * that is not an object is skipped, a `type` the kit does not know is `string`, a setting with no
 * `title` is titled by its key, and a `default` that is not text, a number or a Boolean is none.
 * Reading a block already read gives it back.
 */
export const readMcpbOptions = (declared: unknown): Readonly<Record<string, UserConfigOption>> =>
  call('readMcpbOptions', declared);

/**
 * The name, server and settings of a bundle manifest for `platform` (`darwin`, `linux`, `win32`).
 * Judgment: a `type` other than `node` or `python` with an `entry_point` runs that file as a binary.
 */
export const readMcpbManifest = (manifest: unknown, platform: string): McpbRead =>
  call('readMcpbManifest', manifest, platform);

/** The key a bundled server's setting has among the plugin's settings: `<server>.<key>`. */
export const bundleSettingKey = (server: string, key: string): string =>
  call('bundleSettingKey', server, key);

/**
 * The settings of a bundle the server cannot start without (plugins/components.md: "A bundled server
 * with a required setting that has no saved value doesn't start"), as `<server>.<key>`. Judgment: a
 * required setting with a default of its own is not one.
 */
export const requiredBundleSettings = (
  manifest: Pick<McpbManifest, 'name' | 'options'>,
): string[] => call('requiredBundleSettings', manifest);

/**
 * The settings among `required` that `saved` has no value for: the one rule both the host's dialog and
 * the kit's load-time check use. Judgment (spec/judgments.md): an empty text is no value, as a
 * required plugin option's dialog "doesn't accept an empty value" (plugins/manifest-reference.md).
 */
export const missingBundleSettings = (
  required: readonly string[],
  saved: Readonly<Record<string, unknown>>,
): string[] => call('missingBundleSettings', required, saved);

export interface McpbVariables {
  /** Where the bundle was extracted: `${__dirname}`. */
  readonly directory: string;
  readonly home: string;
  /** `/`, or `\` on Windows: `${/}` and `${pathSeparator}`. */
  readonly separator: string;
}

/**
 * The manifest's `${...}` variables in every string of the server: `${__dirname}`, `${HOME}`, `${/}` and
 * `${pathSeparator}`. `${user_config.<key>}` stays as written: the kit fills it from the bundle's own
 * saved settings when it resolves the server, in its one substitution pass; any other variable is left
 * as written.
 */
export const bundleServerOf = (
  manifest: McpbManifest,
  variables: McpbVariables,
): Readonly<Record<string, unknown>> => call('bundleServerOf', manifest, variables);
