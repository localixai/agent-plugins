import type { PermissionMode } from '../../protocol/official.ts';
import type { PolicyContext } from '../context.ts';
import type { PolicyView } from '../view.ts';
import type { AutoModeSettings } from './auto-mode.ts';
import type { StartInput, StartingMode } from './default-mode.ts';
import type { SettingsDirectory, SettingsRule } from './managed-only.ts';
import type { RunningMode } from './running-mode.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/permissions/policy');

/**
 * The permission keys of the settings as one object over a view (docs/API.md, "Settings").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/policy.rs`, docs/RUST.md).
 */
export interface PermissionsPolicy {
  /** `allowManagedPermissionRulesOnly`: managed settings are the only source of permission rules. */
  readonly managedRulesOnly: boolean;
  /** The rules in force, as written, highest file first; the host parses each with `parseRule`. */
  readonly rules: readonly SettingsRule[];
  /** The directories `permissions.additionalDirectories` adds, each once. */
  readonly additionalDirectories: readonly SettingsDirectory[];
  /** A `true` in any file: the file tools refuse reads outside the working directories. */
  readonly blockReadsOutsideWorkingDirectories: boolean;
  /** A `true` in any file: `Write`, `Edit` and `NotebookEdit` refuse paths outside the working directories. */
  readonly blockWritesOutsideWorkingDirectories: boolean;
  /**
   * `permissions.denyInsteadOfPrompting`: a `true` from user, `--settings` or managed settings. A
   * call that would wait on a prompt is denied; project and local files are not read for it.
   */
  readonly denyInsteadOfPrompting: boolean;
  /** `permissions.disableBypassPermissionsMode` is `"disable"` in any file. */
  readonly disableBypassPermissionsMode: boolean;
  /** `disableAutoMode` (or `permissions.disableAutoMode`) is `"disable"` in any file. */
  readonly disableAutoMode: boolean;
  /** `skipDangerousModePermissionPrompt` is `true` in any file that carries it. */
  readonly skipDangerousModePermissionPrompt: boolean;
  readonly autoMode: AutoModeSettings;
  /** The mode a new session starts in. */
  startingMode(input: StartInput): StartingMode;
  /**
   * What a session running in `current` changes to once these settings reach it: an auto session
   * leaves auto under a managed `disableAutoMode`, with `auto mode disabled by settings`.
   */
  runningMode(current: PermissionMode): RunningMode;
}

export const createPermissionsPolicy = (
  view: PolicyView,
  context: PolicyContext,
): PermissionsPolicy => call('createPermissionsPolicy', view, context);
