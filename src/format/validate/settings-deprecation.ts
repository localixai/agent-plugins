import { isRecord } from '../../protocol/is-record.ts';
import { PLUGIN_AGENT_DEPRECATED } from '../plugin-settings.ts';
import { warningAt, type Finding } from './finding.ts';

/**
 * The validator's deprecation warnings for a plugin's defaults (spec §12.6 item 2, Judgment J2.9):
 * `agent` and `subagentStatusLine` in the manifest's `settings` or the root `settings.json`, whatever
 * the value; nothing for any other key, a value of the wrong type or a file that does not parse.
 * `subagentStatusLine`'s text names the key itself, so the kit gives it the path `root`, which names
 * no field (spec/judgments.md).
 */
const STATUS_LINE =
  'subagentStatusLine is deprecated in favor of mods and will stop working in a future release.';

/** `prefix` is `settings.` for the manifest and empty for `settings.json`. */
export function settingsDeprecations(value: unknown, prefix: string): Finding[] {
  if (!isRecord(value)) return [];
  return [
    ...(Object.hasOwn(value, 'agent')
      ? [warningAt(`${prefix}agent`, PLUGIN_AGENT_DEPRECATED)]
      : []),
    ...(Object.hasOwn(value, 'subagentStatusLine')
      ? [warningAt('root', `${prefix}${STATUS_LINE}`)]
      : []),
  ];
}
