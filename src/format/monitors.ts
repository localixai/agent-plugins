import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

/**
 * Monitor entries of a plugin (plugins/manifest-reference.md, "monitors"; plugins/components.md,
 * "Monitors"). The SDK has no monitor config type (no match in sdk.d.ts), so `Monitor` is the kit's,
 * transcribed from the field table: "Each entry is a strict object with these fields", and "An
 * unknown key inside one is an error, and the plugin doesn't load" (manifest-reference, "Unrecognized
 * fields").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/monitors.rs`, docs/RUST.md).
 */

const call = coreModule('format/monitors');

/** `always`, or the first run of one skill. */
export type MonitorTrigger =
  | { readonly kind: 'always' }
  | { readonly kind: 'on-skill-invoke'; readonly skill: string };

export interface Monitor {
  /** "Identifier unique within the plugin." */
  readonly name: string;
  /** "Shell command Claude Code runs as a persistent background process in the session working directory." */
  readonly command: string;
  /** "Short summary shown in the task panel and notification summaries." */
  readonly description: string;
  /** The `when` field; absent means `always`, "the default". */
  readonly when: MonitorTrigger;
}

/**
 * `"always"` or `"on-skill-invoke:<skill>"`. The skill after the colon must not be empty (oracle
 * 2.1.325: `on-skill-invoke: must specify a skill name`), and the value is read as written, with no
 * trimming or case folding (spec J9.8).
 */
export const parseMonitorTrigger = (when: string): MonitorTrigger | undefined =>
  call('parseMonitorTrigger', when);

export interface MonitorRead {
  /** Undefined when Claude Code would not load the entry. */
  readonly monitor: Monitor | undefined;
  readonly issues: readonly PluginIssue[];
}

/** One entry of the array; `index` is its position, for the issue. */
export const readMonitor = (raw: unknown, origin: string, index: number): MonitorRead =>
  call('readMonitor', raw, origin, index);

export interface MonitorsRead {
  readonly monitors: Monitor[];
  readonly issues: PluginIssue[];
}

/**
 * The monitors of one source: `monitors/monitors.json`, a `.json` file `experimental.monitors` names,
 * or its inline array (components.md: "The `experimental.monitors` manifest key takes the same array
 * inline or a path to a JSON file"; manifest-reference: "a `.json` file holding the monitors array").
 *
 * An inline array is part of the manifest's schema, so an invalid one never gets here: the plugin
 * does not load (spec §9.2.1, J9.7). A file with an invalid entry is skipped whole and reported, and
 * the plugin's other components load (J9.7). A later entry of a name already taken is left out, the
 * first keeping the name (J9.7, "at runtime a repeated name keeps the first entry").
 */
export const readMonitors = (raw: unknown, origin: string): MonitorsRead =>
  call('readMonitors', raw, origin);
