import type { InitPluginFields, SDKPluginInstallMessage } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What a non-interactive run reports about plugins in its stream output (headless.md, "Fail CI when
 * a plugin or MCP server doesn't load", "Track plugin installs"; the SDK's `SDKSystemMessage` and
 * `SDKPluginInstallMessage`). The kit builds the fields from what the host loaded and installed;
 * writing the events to the stream, and when, is the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/headless_report.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/headless-report');

/** A load-time error: a category from an open set (`path-not-found`, `dependency-unsatisfied`, ...) and display text. */
export interface PluginLoadError {
  readonly type: string;
  readonly message: string;
}

/** A plugin the session loaded; `errors` are the components it loaded without. */
export interface LoadedPluginRow {
  /** `<name>@<marketplace>`. */
  readonly id: string;
  readonly name: string;
  readonly path: string;
  /** The manifest's `version`, verbatim; absent when it declares none. */
  readonly version?: string;
  readonly errors?: readonly PluginLoadError[];
}

/** A plugin that did not load, such as one whose dependency's version is not satisfied. */
export interface PluginNotLoaded {
  readonly id: string;
  readonly error: PluginLoadError;
}

/**
 * A `--plugin-dir`, SDK `plugins` or synced directory entry that failed before it had a name: it is
 * reported by its position (`inline[N]`, `synced[N]`) and the path it resolved to, absolute.
 */
export interface FailedPluginDirectory {
  readonly origin: 'inline' | 'synced';
  readonly index: number;
  /** Absent for a `--plugin-url` entry. */
  readonly path?: string;
  readonly error: PluginLoadError;
}

export interface InitPluginFacts {
  readonly loaded: readonly LoadedPluginRow[];
  readonly unloaded?: readonly PluginNotLoaded[];
  readonly failedDirectories?: readonly FailedPluginDirectory[];
  /** The session's frames are persisted server-side (a Remote Control worker): `plugin_errors` is always omitted. */
  readonly persistedRemotely?: boolean;
}

/**
 * `plugins` lists what loaded, each with `name` and `path`; `plugin_errors` the load-time errors,
 * each with `plugin`, `type` and `message`, and the key "is omitted when there are no errors". A
 * plugin that did not load is absent from `plugins`; one that loaded without a component keeps its
 * row and gets an entry too.
 */
export const initPluginFields = (facts: InitPluginFacts): InitPluginFields =>
  call('initPluginFields', facts);

/** How one marketplace's install ended: `error` when it failed. */
export interface MarketplaceInstallOutcome {
  readonly name: string;
  readonly error?: string;
}

export interface PluginInstallStamp {
  readonly sessionId: string;
  /** A fresh event identifier per call: the kit has no source of randomness (L3). */
  readonly uuid: () => SDKPluginInstallMessage['uuid'];
}

/**
 * The `system/plugin_install` events of a synchronous install (`CLAUDE_CODE_SYNC_PLUGIN_INSTALL`):
 * `started` and `completed` bracket the install, `installed` and `failed` report one marketplace
 * each, with `name` and, on `failed`, `error`. Whether the install waits is `syncPluginInstall`'s.
 */
export const pluginInstallEvents = (
  outcomes: readonly MarketplaceInstallOutcome[],
  stamp: PluginInstallStamp,
): readonly SDKPluginInstallMessage[] =>
  call('pluginInstallEvents', outcomes, stamp.sessionId, stamp.uuid);
