/**
 * A core `hooks.json` from a Claude Code plugin's hooks (Agent Plugins Extended §18.4.3 item 4,
 * J18.17): only `command` hooks on the five shared events convert, each through the hook adapter,
 * limited to the profile its matcher selects. Hooks on the other events, the other handler types,
 * `if`, `async`, `asyncRewake`, `once` and `onFailure` are not converted; `statusMessage` is dropped
 * and reported. A hook naming `${user_config.*}`, or the core's own `${PLUGIN_ROOT}` and
 * `${PLUGIN_DATA}` (which a core client would expand in the adapter's arguments), is not converted.
 * A Claude Code hook this converter wrote around a core handler is unwrapped to that handler, and its
 * `PostToolUseFailure` twin is dropped quietly (judgment, spec/judgments.md). Positions are those of
 * the plugin's merged hooks.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_core/hooks.rs`, docs/RUST.md).
 */
import type { CoreHookEvent, CoreHookHandler } from '../../format/core/hooks.ts';
import type { HooksSettings } from '../../protocol/hook-config.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ConversionNote } from '../types.ts';

const call = coreModule('convert/to-core/hooks');

export const CORE_HOOKS: string = call('CORE_HOOKS');

export interface CoreHooksPart {
  readonly hooks: { readonly [E in CoreHookEvent]?: readonly CoreHookHandler[] };
  readonly adapted: boolean;
  readonly notes: readonly ConversionNote[];
}

export const coreHooksOf = (hooks: HooksSettings, adapter: boolean): CoreHooksPart =>
  call('coreHooksOf', hooks, adapter);
