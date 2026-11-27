import type { PluginIssue } from '../format/issues.ts';
import type { PluginSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ArchiveLimits, FetchPlan, FetchPlanContext } from './types.ts';

/**
 * Planning how a plugin source is fetched (plugins/marketplace-reference.md, "Plugin sources" and
 * its sub-sections; plugins/host-marketplace.md, "Stay within the download limits for hosted
 * files"): pure, a source and what the host knows in, a `FetchPlan` or the issues that refuse it
 * out. Issue paths are relative to the entry (`source`, `source.url`): the caller prefixes
 * `plugins[i].`. Nothing is fetched here; a host runs the plan, or an engine over ports does.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/fetch_plan.rs`,
 * docs/RUST.md).
 */

const call = coreModule('marketplace/fetch-plan');

export interface Planned {
  readonly plan: FetchPlan | undefined;
  readonly issues: readonly PluginIssue[];
}

/** The limits of an `archive` source: the download and the extraction ("Stay within the download limits"). */
export const ARCHIVE_LIMITS: ArchiveLimits = call('ARCHIVE_LIMITS');

/**
 * How `source` is fetched, or why it may not be. A source kind this build has no planner for (an
 * entry written for a newer Claude Code) is refused with the message the docs print, "Update
 * Claude Code and try again" (plugins/troubleshooting.md, "This plugin uses a source type your
 * Claude Code version does not support"), instead of failing in the planner lookup.
 */
export const planPluginFetch = (source: PluginSource, context: FetchPlanContext): Planned =>
  call('planPluginFetch', source, context);
