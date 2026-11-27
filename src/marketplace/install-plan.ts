import type { PluginIssue } from '../format/issues.ts';
import type { PluginSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { InstallDecision } from './install-mode.ts';
import type { InstallScope } from './install-scopes.ts';
import type { InstalledRecords, InstallRecord } from './installed-plugins.ts';
import type { PluginVersion } from './version-resolution.ts';
import type { PathJoin, StoreLayout } from './store-layout.ts';
import type { FetchPlan, FetchPlanContext } from './types.ts';

/**
 * Installing a marketplace plugin as two pure steps (plugins/loading.md, "Find plugins on disk",
 * "In-place and copied plugins", "Cleanup of previous versions"; plugins/marketplace-reference.md,
 * "Copy mode and link mode"): what to fetch and how it will be installed, then, once the host has
 * fetched it and the version is known, where it goes and what `installed_plugins.json` becomes.
 * The kit moves nothing; the host runs the fetch, the copy or the links and writes the file.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/install_plan.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/install-plan');

export interface InstallStart {
  /** What to fetch; `undefined` when the source is refused. */
  readonly fetch: FetchPlan | undefined;
  readonly decision: InstallDecision | undefined;
  readonly issues: readonly PluginIssue[];
}

export interface InstallStartInput {
  readonly source: PluginSource;
  /** `marketplaceOrigin` is how the marketplace the plugin is listed in was added. */
  readonly fetch: FetchPlanContext;
}

/** Before the fetch: its plan and how the plugin will be installed. A refusal of either is an issue and plans nothing. */
export const planInstallStart = (input: InstallStartInput): InstallStart =>
  call('planInstallStart', input);

/** A link of a link-mode cache entry: one per top-level entry of the printed directory. */
export interface CacheLink {
  /** The entry's name inside the cache directory. */
  readonly name: string;
  /** What it links to: that entry of the printed directory. */
  readonly target: string;
}

export interface InstallFinish {
  readonly name: string;
  readonly marketplace: string;
  readonly decision: InstallDecision;
  readonly version: PluginVersion;
  readonly layout: StoreLayout;
  /** The host's path join, for the targets of the links. */
  readonly join: PathJoin;
  readonly platform: 'posix' | 'windows';
  readonly scope: InstallScope;
  /** The folder of a `project` or `local` install. */
  readonly projectPath?: string;
  /** Where a plugin loaded in place lies, or the directory a `command` source printed. */
  readonly sourceDirectory?: string;
  /** The top-level names of the printed directory, for link mode. */
  readonly topLevel?: readonly string[];
  /** The commit a git-sourced plugin was installed at. */
  readonly gitCommitSha?: string;
  /** The records now in `installed_plugins.json`. */
  readonly existing: InstalledRecords;
  /** The time of the install, as the text the file records (the kit reads no clock). */
  readonly at: string;
}

export interface InstallFinished {
  /** The folder a copy is committed to, or a link-mode entry is filled in; `undefined` for a plugin loaded in place. */
  readonly destination?: string;
  /** The host copies the fetched files into `destination`; false for link mode, which copies nothing, and for a plugin loaded in place. */
  readonly copy: boolean;
  readonly links: readonly CacheLink[];
  /** Link mode: "Keep the directory in place". The links point into this directory, so it must stay where it is. */
  readonly mustStayAt?: string;
  readonly record?: InstallRecord;
  /** The records to write; `existing` when nothing could be planned. */
  readonly records: InstalledRecords;
  /** The version directories the update left behind: write `.orphaned_at` into each. */
  readonly orphaned: readonly string[];
  readonly issues: readonly PluginIssue[];
}

/**
 * After the fetch: the cache folder and the record. An update puts the new record in the old one's
 * place and orphans the version folder it leaves behind, "so a session that already loaded the old
 * version keeps running"; a folder outside the cache (a plugin loaded in place) is never marked,
 * since a sweep would delete the user's own files. Link mode copies nothing: it links each
 * top-level entry of the printed directory, which `mustStayAt` says must stay where it is.
 */
export const planInstallFinish = (input: InstallFinish): InstallFinished =>
  call('planInstallFinish', input);
