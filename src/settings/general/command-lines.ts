import type { Settings } from '../../protocol/official.ts';
import type { HookPolicy } from '../hooks/policy.ts';
import type { PolicyScope } from '../layers.ts';
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginDefaults, PluginWithDefaults } from './plugin-defaults.ts';

/**
 * settings-reference.md, "Status line and file suggestion gates": commands follow hook policy.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/command_lines.rs`, docs/RUST.md).
 */

const call = coreModule('settings/general/command-lines');

export const COMMAND_LINE_KEYS: readonly ['statusLine', 'fileSuggestion', 'subagentStatusLine'] =
  call('COMMAND_LINE_KEYS');
type CommandKey = (typeof COMMAND_LINE_KEYS)[number];
type CommandValue = NonNullable<Settings['statusLine']>;

interface CommandRead {
  readonly runs: { readonly value: CommandValue; readonly scope: PolicyScope } | undefined;
  readonly skippedBecause?: 'off' | 'narrowed';
}

/** The selected command, never an execution: the host supplies the hook policy's session gates. */
export const readCommandLine = (
  view: PolicyView,
  key: CommandKey,
  policy: HookPolicy,
): CommandRead => call('readCommandLine', view, key, policy);

/** The subagent status line that runs: a settings source's (with its scope) or a plugin's default. */
export interface SubagentLineRead<P> {
  readonly runs:
    | { readonly value: CommandValue; readonly scope: PolicyScope }
    | { readonly value: CommandValue; readonly plugin: P }
    | undefined;
  readonly skippedBecause?: 'off' | 'narrowed';
}

/**
 * `subagentStatusLine` with the plugins' defaults below every settings source (plugins/components.md,
 * "Default settings"), under the same gates as `statusLine`. statusline.md, "Subagent status lines":
 * "unlike hooks, plugin values don't run under `allowManagedHooksOnly` even when the plugin is
 * force-enabled", so a narrowed policy never runs a plugin's value.
 */
export const readSubagentStatusLine = <P extends PluginWithDefaults>(
  view: PolicyView,
  policy: HookPolicy,
  defaults: PluginDefaults<P>,
): SubagentLineRead<P> => call('readSubagentStatusLine', view, policy, defaults);
