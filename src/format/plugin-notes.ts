/**
 * The notes a plugin loads with (spec §2.1 item 3, §2.4, §17.8.2): something about the plugin that
 * did not stop it loading but that `claude plugin list` and `/plugin` show, each with the
 * `noteDetails` type Claude Code gives it. A note is not an issue: the plugin is fine to use.
 */
export const PLUGIN_NOTE_TYPES = [
  'manifest-field-ignored',
  'engine-range-exempt',
  'engine-range-invalid',
  'plugin-agent-setting-deprecated',
] as const;
export type PluginNoteType = (typeof PLUGIN_NOTE_TYPES)[number];

export interface PluginNote {
  /** The `noteDetails` type of `claude plugin list --json`. */
  readonly type: PluginNoteType;
  /** The line `claude plugin list` prints after `Note:`. */
  readonly message: string;
}

export const note = (type: PluginNoteType, message: string): PluginNote => ({ type, message });
