import type { JsonObject, JsonValue } from '../foundation/plain-data.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { Rendered } from './result.ts';

const call = coreModule('cli/plugin-list');

/**
 * `claude plugin list` (plugins/cli-reference.md, "plugin list" and "JSON output"): the groups of
 * the text listing, the `--json` array with the fields the page names, `--available` and
 * `--data-size`. Pure: the host reads the records, the settings and the data directories and passes
 * them in. The layout of a plugin's text lines is the kit's (spec/judgments.md); the group headings,
 * the empty line and every JSON field are the docs'.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/plugin_list.rs`, docs/RUST.md).
 */

/** One installation, as the list shows it. Optional fields "appear only when they apply". */
export interface ListedPlugin {
  /** `name@marketplace`, `name@inline`, `name@skills-dir` or `name@synced`. */
  readonly id: string;
  /** The computed version, or the manifest's, or `unknown`. */
  readonly version: string;
  /** `user`, `project`, `local`, `managed`, `session` or `synced`. */
  readonly scope: string;
  readonly enabled: boolean;
  /** The directory it loads from; absent for a plugin loaded in place, which has `readFromFolder`. */
  readonly installPath?: string;
  readonly readFromFolder?: string;
  readonly folderVersion?: string;
  readonly installedAt?: string;
  readonly lastUpdated?: string;
  readonly projectPath?: string;
  readonly mcpServers?: Readonly<Record<string, JsonValue>>;
  readonly errors?: readonly string[];
  readonly notes?: readonly string[];
  readonly errorDetails?: readonly Readonly<Record<string, JsonValue>>[];
  readonly noteDetails?: readonly Readonly<Record<string, JsonValue>>[];
  /** Only for a plugin that loaded and declares `userConfig`. */
  readonly hasUserConfig?: boolean;
  /** Marketplace installs only: the project's shared settings turn it on. */
  readonly projectEnabled?: boolean;
}

/** A plugin a marketplace offers that is not installed. */
export interface AvailablePlugin {
  readonly pluginId: string;
  readonly name: string;
  readonly marketplaceName: string;
  readonly source: string | Readonly<Record<string, JsonValue>>;
  readonly description?: string;
  readonly version?: string;
  readonly installCount?: number;
}

/** What measuring a saved data directory found; absent for a missing or empty one. */
export type DataSize = { readonly bytes: number } | 'unreadable';

export interface ListRequest {
  readonly installed: readonly ListedPlugin[];
  readonly available?: readonly AvailablePlugin[];
}

export interface ListOptions {
  readonly json: boolean;
  /** `--available`: "has no effect without `--json`". */
  readonly available?: boolean;
  /**
   * `--data-size [plugin]`: `true` for every installed plugin, or one `name@marketplace`; "has no
   * effect without `--json`". `sizes` is what the host measured, by id.
   */
  readonly dataSize?: true | string;
  readonly sizes?: Readonly<Record<string, DataSize | undefined>>;
}

/** The text listing, grouped by how each plugin loads; nothing in any group is one line. */
export const listText = (installed: readonly ListedPlugin[]): readonly string[] =>
  call('listText', installed);

/** One installed plugin as a JSON object, only the fields that apply. */
export const installedJson = (
  plugin: ListedPlugin,
  options: ListOptions = { json: true },
): JsonObject => call('installedJson', plugin, options);

/** The `available` entry. */
export const availableJson = (plugin: AvailablePlugin): JsonObject => call('availableJson', plugin);

/**
 * What `claude plugin list` prints. `--json` is an array with one object per installation, or with
 * `--available` one object holding `installed` and `available`. A `--data-size` plugin with no install
 * record prints its message and exits `1` "instead of printing the list".
 */
export const planPluginList = (request: ListRequest, options: ListOptions): Rendered =>
  call('planPluginList', request, options);
