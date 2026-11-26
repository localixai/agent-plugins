import type { PluginForm } from '../loading/core/form.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The two forms' path placeholders (Agent Plugins Extended §18.3.3, §18.4.2 item 3, §18.4.3 item
 * 3): the core's `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` (core §9.2) and Claude Code's
 * `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}`, which name the same directories (J18.13).
 * A rename is one pass over the original text, so a renamed placeholder is never renamed again.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/placeholders.rs`, docs/RUST.md).
 */

const call = coreModule('convert/placeholders');

/** `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}` written as the core's names. */
export const toCorePlaceholders = (text: string): string => call('toCorePlaceholders', text);

/** Whether text holds a `${`, which the other form would expand or keep literally. */
export const holdsReference = (text: string): boolean => call('holdsReference', text);

/** Whether text written for `form` holds a `${` other than that form's two path placeholders. */
export const holdsOtherReference = (text: string, form: PluginForm): boolean =>
  call('holdsOtherReference', text, form);

/**
 * The `env` entries that bridge the environment in configuration (J18.18), by the form whose
 * configuration holds them: a Claude Code stdio server gets the core's two variables, a core stdio
 * server Claude Code's two.
 */
const BRIDGES = {
  'claude-code': { PLUGIN_ROOT: '${CLAUDE_PLUGIN_ROOT}', PLUGIN_DATA: '${CLAUDE_PLUGIN_DATA}' },
  core: { CLAUDE_PLUGIN_ROOT: '${PLUGIN_ROOT}', CLAUDE_PLUGIN_DATA: '${PLUGIN_DATA}' },
} as const satisfies Record<PluginForm, Readonly<Record<string, string>>>;

/** The form a conversion from each form writes. */
const OTHER_FORM: { readonly [F in PluginForm]: PluginForm } = {
  'claude-code': 'core',
  core: 'claude-code',
};

const isBridge = (form: PluginForm, key: string, value: string): boolean =>
  Object.entries(BRIDGES[form]).some(([name, bridged]) => name === key && bridged === value);

/**
 * A stdio server's `env` moved from one form to the other: the source's own bridge entries left out,
 * since the target sets those variables itself to the same directories (J18.13; judgment, so that a
 * converted package converts back to what it was), the values renamed by `rename`, then the
 * target's bridge added where the server sets no such variable.
 *
 * Stays TypeScript: `rename` is a synchronous host function and the converters call this one
 * synchronously, which a call into the core could not answer (docs/RUST.md, "What stays
 * TypeScript"); the core keeps its own copy (`bridged_environment`).
 */
export function bridgedEnvironment(
  env: Readonly<Record<string, string>> | undefined,
  from: PluginForm,
  rename: (text: string) => string,
): Readonly<Record<string, string>> {
  const kept = Object.entries(env ?? {})
    .filter(([key, value]) => !isBridge(from, key, value))
    .map(([key, value]): [string, string] => [key, rename(value)]);
  const added = Object.entries(BRIDGES[OTHER_FORM[from]]).filter(
    ([key]) => !kept.some(([name]) => name === key),
  );
  return Object.fromEntries([...kept, ...added]);
}
