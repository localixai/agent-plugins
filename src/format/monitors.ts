import type { z } from 'zod';
import { error, type PluginIssue } from './issues.ts';
import { ALWAYS, monitorEntrySchema, skillOf, UNIQUE_NAMES, WHEN_SKILL } from './monitor-schema.ts';
import { isRecord } from '../protocol/is-record.ts';

/**
 * Monitor entries of a plugin (plugins/manifest-reference.md, "monitors"; plugins/components.md,
 * "Monitors"). The SDK has no monitor config type (no match in sdk.d.ts), so `Monitor` is the kit's,
 * transcribed from the field table: "Each entry is a strict object with these fields", and "An
 * unknown key inside one is an error, and the plugin doesn't load" (manifest-reference, "Unrecognized
 * fields").
 */

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

/** What a monitor without `when` starts on: "`\"always\"`, the default". */
const ALWAYS_TRIGGER: MonitorTrigger = { kind: 'always' };

/**
 * `"always"` or `"on-skill-invoke:<skill>"`. The skill after the colon must not be empty (oracle
 * 2.1.325: `on-skill-invoke: must specify a skill name`), and the value is read as written, with no
 * trimming or case folding (spec J9.8).
 */
export function parseMonitorTrigger(when: string): MonitorTrigger | undefined {
  if (when === ALWAYS) return ALWAYS_TRIGGER;
  const skill = skillOf(when);
  return skill.length > 0 ? { kind: 'on-skill-invoke', skill } : undefined;
}

const reason = (failure: z.ZodError): string =>
  failure.issues
    .map((found) =>
      found.path.length > 0 ? `${found.path.join('.')}: ${found.message}` : found.message,
    )
    .join('; ');

const invalid = (origin: string, index: number, why: string): PluginIssue =>
  error(`${origin}[${index}]`, `Invalid monitor at ${origin}[${index}]: ${why}`);

export interface MonitorRead {
  /** Undefined when Claude Code would not load the entry. */
  readonly monitor: Monitor | undefined;
  readonly issues: readonly PluginIssue[];
}

/** One entry of the array; `index` is its position, for the issue. */
export function readMonitor(raw: unknown, origin: string, index: number): MonitorRead {
  if (!isRecord(raw)) {
    return { monitor: undefined, issues: [invalid(origin, index, 'expected an object')] };
  }
  const parsed = monitorEntrySchema.safeParse(raw);
  if (!parsed.success) {
    return { monitor: undefined, issues: [invalid(origin, index, reason(parsed.error))] };
  }
  const { name, command, description, when } = parsed.data;
  const trigger = when === undefined ? ALWAYS_TRIGGER : parseMonitorTrigger(when);
  // The schema refused every other `when`; this narrows the type.
  if (!trigger) return { monitor: undefined, issues: [invalid(origin, index, WHEN_SKILL)] };
  return { monitor: { name, command, description, when: trigger }, issues: [] };
}

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
export function readMonitors(raw: unknown, origin: string): MonitorsRead {
  if (!Array.isArray(raw)) {
    const why = 'expected an array of monitors';
    return { monitors: [], issues: [error(origin, `Invalid monitors in ${origin}: ${why}`)] };
  }
  const reads = raw.map((entry: unknown, index) => readMonitor(entry, origin, index));
  const failures = reads.flatMap((read) => [...read.issues]);
  if (failures.length > 0) return { monitors: [], issues: failures };
  const monitors: Monitor[] = [];
  const issues: PluginIssue[] = [];
  reads.forEach(({ monitor }, index) => {
    if (!monitor) return;
    if (monitors.some((kept) => kept.name === monitor.name)) {
      issues.push(
        invalid(origin, index, `${UNIQUE_NAMES}; "${monitor.name}" is kept from the first entry`),
      );
      return;
    }
    monitors.push(monitor);
  });
  return { monitors, issues };
}
