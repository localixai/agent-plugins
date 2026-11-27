/**
 * When the configuration dialog appears and what it starts with (components.md, "When the
 * configuration dialog appears").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/configure_dialog.rs`, docs/RUST.md).
 */
import type { ResolvedUserConfig, UserConfigValue } from '../loading/resolve/user-config.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/configure-dialog');

/** The events that can open the dialog. */
export type ConfigurationEvent =
  | 'session-install'
  | 'session-enable'
  | 'session-configure'
  | 'vscode-install'
  | 'vscode-configure'
  | 'shell-install';

export interface ConfigurationDialog {
  readonly open: boolean;
  readonly keys: readonly string[];
  /** Sensitive starting values never leave the secure store. */
  readonly inputs: Readonly<Record<string, UserConfigValue>>;
}

/** components.md § When the configuration dialog appears. The host renders this plan, never shell install. */
export const configurationDialog = (
  event: ConfigurationEvent,
  config: ResolvedUserConfig,
): ConfigurationDialog => call('configurationDialog', event, config);
