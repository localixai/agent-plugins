import type { PluginIssue } from '../format/issues.ts';
import type { InstallScope } from './install-scopes.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * `installed_plugins.json`, the record of what Claude Code has installed (plugins/loading.md,
 * "Check which stage a plugin reached": "records each install with its `scope`, `installPath`, and
 * `version`"; "Find plugins on disk"), and what it does with a file or a record it cannot use
 * (plugins/troubleshooting.md: "could not be read and was rebuilt", "records under names that no
 * version of Claude Code can use were removed", "holds a record under "<id>" that this version of
 * Claude Code cannot read"). The Agent SDK types no such file, so the shapes are declared here from
 * the docs. Pure: reading and writing the file and its dated copies is `PluginStateStore`'s.
 *
 * Judgments (spec/judgments.md): the file is `{ "version": 2, "plugins": { "<id>": [record, ...] } }`
 * (the docs name a "format (version <N>)" and `projectPath`, not the whole layout); `installedAt`,
 * `lastUpdated`, `gitCommitSha` and `projectPath` are optional text; fields the kit does not know
 * are kept and written back; a file with no numeric `version` is not a list of plugins in any
 * format and is rebuilt.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/installed_plugins.rs`,
 * docs/RUST.md).
 */

const call = coreModule('marketplace/installed-plugins');

export interface InstallRecord {
  readonly scope: InstallScope;
  /** Where the installed copy is; for a copied plugin `cache/<marketplace>/<plugin>/<version>/`. */
  readonly installPath: string;
  readonly version: string;
  readonly installedAt?: string;
  readonly lastUpdated?: string;
  /** The commit a git-sourced plugin was installed at. */
  readonly gitCommitSha?: string;
  /** The project folder of a `project` or `local` install. */
  readonly projectPath?: string;
  /** Fields this version does not know, kept so a rewrite loses nothing. */
  readonly extra?: Readonly<Record<string, unknown>>;
}

/** Plugin id (`name@marketplace`) to its installs, one per scope (and project). */
export type InstalledRecords = Readonly<Record<string, readonly InstallRecord[]>>;

export interface InstalledPluginsRead {
  /** The records this version can use. */
  readonly records: InstalledRecords;
  /** Raw values under keys that are not plugin ids: copy them aside, then drop them. */
  readonly setAside: Readonly<Record<string, unknown>>;
  /** Ids whose value this version cannot read, in file order. */
  readonly unreadable: readonly string[];
  /** The `version` of a file whose format this version does not know. */
  readonly unknownFormat?: number;
  /** A file that is not a list of plugins: its text, to keep, before the file is rebuilt empty. */
  readonly rebuilt?: { readonly text: string };
  /** Whether the file may be rewritten: never while a record or the format cannot be read. */
  readonly writable: boolean;
  readonly issues: readonly PluginIssue[];
}

/**
 * What a file's text says. A file that "isn't valid JSON, or isn't a list of plugins" is rebuilt
 * (its text kept); a key that is not a plugin id has its record set aside; a record whose fields
 * do not parse is named and keeps the file from being rewritten; a format this version does not
 * know is named and read no further. No file is no installs.
 */
export const parseInstalledPlugins = (text: string | undefined): InstalledPluginsRead =>
  call('parseInstalledPlugins', text);

/** The file's text for these records: two-space indented JSON and a final newline (a judgment). */
export const serializeInstalledPlugins = (records: InstalledRecords): string =>
  call('serializeInstalledPlugins', records);
