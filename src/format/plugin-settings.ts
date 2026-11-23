import { z } from 'zod';
import type { Settings } from '../protocol/official.ts';
import { warning, type PluginIssue } from './issues.ts';
import { isRecord } from '../protocol/is-record.ts';

/**
 * A plugin's default settings (spec §12.6): the `settings.json` at the plugin root or the inline
 * `settings` manifest key. A deprecated feature: "Only `agent` takes effect. Every other key MUST be
 * dropped at load", `subagentStatusLine` included since 2.1.323 (Judgment J12.17). The type is the
 * official `Settings` one; applying the value (the lowest settings layer) is the host's.
 */
export type PluginSettings = Pick<Settings, 'agent'>;

/**
 * The deprecation warning a validator gives for a plugin's `agent` default, whatever the value,
 * without failing (spec §12.6 item 2): the text after the `agent:` path. The client's startup and
 * `claude plugin list` warnings use the same text (Judgment J12.14).
 */
export const PLUGIN_AGENT_DEPRECATED =
  "A plugin's `agent` setting is deprecated and will stop applying in a future release. Tell people to set `agent` in their own settings or pass --agent.";

/** One strategy per applied key: the value it keeps, or `undefined` when it is not valid. */
const APPLIED: { readonly [K in keyof PluginSettings]-?: (value: unknown) => PluginSettings[K] } = {
  agent: (value) => z.string().safeParse(value).data,
};

const isApplied = (name: string): name is keyof PluginSettings => Object.hasOwn(APPLIED, name);

export interface PluginSettingsRead {
  /** Only the keys that take effect and hold a valid value. */
  readonly settings: PluginSettings;
  readonly issues: readonly PluginIssue[];
}

/** True when a read holds a valid `agent`: `settings.json` then wins over the manifest (J12.10). */
export const hasAppliedSettings = (read: PluginSettingsRead): boolean =>
  Object.keys(read.settings).length > 0;

/**
 * Reads one settings object. Judgment (spec/judgments.md): a dropped key and an invalid value for an
 * applied key are warnings (the spec says "dropped" and names no message), and a value that is not
 * an object is a warning naming `origin`: a plugin's defaults never fail the plugin (spec §12.6 item
 * 1, J12.10).
 */
export function readPluginSettings(raw: unknown, origin: string): PluginSettingsRead {
  if (!isRecord(raw)) {
    return {
      settings: {},
      issues: [warning(origin, `${origin} must be a JSON object of settings`)],
    };
  }
  const issues: PluginIssue[] = [];
  const kept: Record<string, unknown> = {};
  for (const [name, value] of Object.entries(raw)) {
    if (!isApplied(name)) {
      issues.push(
        warning(
          `${origin}.${name}`,
          `Setting "${name}" in ${origin} is dropped: only "agent" takes effect from a plugin`,
        ),
      );
      continue;
    }
    const accepted = APPLIED[name](value);
    if (accepted === undefined) {
      issues.push(
        warning(`${origin}.${name}`, `Setting "${name}" in ${origin} is dropped: invalid value`),
      );
    } else {
      kept[name] = accepted;
    }
  }
  return { settings: kept, issues };
}
