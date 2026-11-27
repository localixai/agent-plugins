/**
 * `disableCommandPluginSources`: whether Claude Code may run a command a marketplace declares
 * (settings-reference.md, `disableCommandPluginSources` and "What runs under `allowManagedHooksOnly`";
 * plugins/host-marketplace.md, "When Claude Code skips a headersHelper command"). The kit returns the
 * decision; running, installing or loading a command-sourced plugin stays the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/command_sources.rs`,
 * docs/RUST.md).
 */
import type { PolicyScope } from '../layers.ts';
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/marketplaces/command-sources');

/** The key that blocks a command: the explicit one, or the one that decides while it is unset. */
export type CommandBlocker = 'disableCommandPluginSources' | 'allowManagedHooksOnly';

export type CommandVerdict =
  | { readonly run: true }
  | { readonly run: false; readonly reason: CommandBlocker };

export interface CommandSourcePolicy {
  /** Command sources are blocked, by either key. */
  readonly disabled: boolean;
  /**
   * A plugin with a `command` source: its command never runs, it is not installed or updated, and one
   * already installed stops loading, a plugin force-enabled in managed `enabledPlugins` included.
   */
  readonly pluginSource: CommandVerdict;
  /**
   * A marketplace's or an inline plugin entry's `headersHelper`, given the scope of the file that
   * declares the marketplace. Blocked with the command sources, "except for a marketplace that managed
   * settings themselves declare".
   */
  headersHelper(declaredIn: PolicyScope): CommandVerdict;
}

export const createCommandSourcePolicy = (view: PolicyView): CommandSourcePolicy =>
  call('createCommandSourcePolicy', view);
