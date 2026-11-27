import type { PluginIssue } from '../../format/issues.ts';
import type { PermissionMode } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

/**
 * Which permission mode a session starts in (permission-modes.md, "Which mode a session starts in",
 * "Start in a different permission mode", "Skip all checks with bypassPermissions mode",
 * "Bypass permissions ignored in a background session"; settings-reference.md,
 * `permissions.defaultMode`, `disableAutoMode`, `permissions.disableBypassPermissionsMode`).
 * Pure over the settings and what the host knows of the launch.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/default_mode.rs`,
 * docs/RUST.md).
 */

const call = coreModule('settings/permissions/default-mode');

/** Where the session runs: the terminal, the VS Code extension, or `claude -p` and the Agent SDK. */
export const LAUNCH_SURFACES: readonly ['terminal', 'vscode', 'headless'] = call('LAUNCH_SURFACES');
export type LaunchSurface = (typeof LAUNCH_SURFACES)[number];

/** What the host knows of the launch and the session. */
export interface StartInput {
  readonly surface: LaunchSurface;
  /** `--permission-mode <value>` as typed (`manual` is an alias for `default`). */
  readonly permissionMode?: string;
  /** `--dangerously-skip-permissions`. */
  readonly dangerouslySkipPermissions?: boolean;
  /** `--allow-dangerously-skip-permissions`: makes `bypassPermissions` selectable later. */
  readonly allowDangerouslySkipPermissions?: boolean;
  /**
   * Auto mode can be used: the model supports it and Anthropic has not turned it off ("Auto mode is
   * unavailable when the session doesn't meet the availability requirements").
   */
  readonly autoAvailable: boolean;
  /** The HIPAA configuration is applied and the session is eligible for it. */
  readonly hipaa?: boolean;
  /** A background session (`claude --bg`). */
  readonly background?: boolean;
  /** Started with `--restricted`, which refuses `bypassPermissions`. */
  readonly restricted?: boolean;
  /** Running as root or under `sudo` on Linux or macOS, outside a recognized sandbox. */
  readonly privileged?: boolean;
}

/** Why the session starts in the mode it does. */
export type StartSource = 'flag' | 'settings' | 'built-in';

export type StartingMode =
  | {
      readonly outcome: 'started';
      readonly mode: PermissionMode;
      readonly source: StartSource;
      /** Bypass permissions was enabled at launch, so plan mode does not enforce its blocks at a terminal. */
      readonly bypassAvailable: boolean;
      /** Auto mode is in the session's mode cycle. */
      readonly autoAvailable: boolean;
      /** An interactive session entering `bypassPermissions` shows the warning dialog first. */
      readonly bypassWarning: boolean;
      /** Notices the session shows at its start, verbatim (`startNotices`). */
      readonly notices: readonly string[];
      /** What the session shows and the settings said, as warnings. */
      readonly issues: readonly PluginIssue[];
    }
  | { readonly outcome: 'refused'; readonly reason: string };

/** The notice a background session shows for an ignored bypass request (permission-modes.md, verbatim). */
export const BYPASS_IGNORED_NOTICE: string = call('BYPASS_IGNORED_NOTICE');

/** The settings keys the start reads, from the view. */
export interface StartSettings {
  readonly disableAutoMode: boolean;
  readonly disableBypassPermissionsMode: boolean;
  readonly skipDangerousModePermissionPrompt: boolean;
}

/** permission-modes.md, verbatim. */
export const BYPASS_AS_ROOT: string = call('BYPASS_AS_ROOT');

/**
 * The mode a session starts in. The first of these applies: the flag, `permissions.defaultMode`,
 * the built-in default; `disableAutoMode` turns an `auto` start (from any of the three) into
 * `default`, and `permissions.disableBypassPermissionsMode` rejects the flag.
 */
export const startingMode = (
  view: PolicyView,
  settings: StartSettings,
  input: StartInput,
): StartingMode => call('startingMode', view, settings, input);
