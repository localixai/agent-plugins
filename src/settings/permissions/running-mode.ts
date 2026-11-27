/**
 * permission-modes.md, "Eliminate prompts with auto mode": with `disableAutoMode: "disable"`, "A
 * session already running in auto mode leaves it when the setting reaches that session from an
 * admin-deployed source, and shows `auto mode disabled by settings`". The admin-deployed sources are
 * the managed ones (managed-settings.md, "Which managed source Claude Code uses"); the mode it
 * leaves for is Manual, as a start in auto under the setting is (judgment, spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/running_mode.rs`, docs/RUST.md).
 */
import type { PermissionMode } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/permissions/running-mode');

/** The notice the session shows, verbatim. */
export const AUTO_MODE_DISABLED_NOTICE: string = call('AUTO_MODE_DISABLED_NOTICE');

/** What a running session's mode becomes once new settings reach it. */
export interface RunningMode {
  readonly mode: PermissionMode;
  /** Set when the settings moved the session out of its mode. */
  readonly notice?: string;
}
