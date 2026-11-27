/**
 * The ways to change one setting for one session without saving it (settings.md, "Change a setting
 * for one session"): a flag for that key and an environment variable paired with it. Environment
 * variables "aren't a level in this stack": which one applies is decided per pair (env-vars.md,
 * "Precedence"; model-config.md, "Setting your model"; settings-reference.md, `effortLevel`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/session_overrides.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/general/session-overrides');

export interface SessionInput {
  /** The command-line flags, by name (`--model`), with the value given. */
  readonly flags: Readonly<Record<string, string | undefined>>;
  /** The host's environment. */
  readonly env: Readonly<Record<string, string | undefined>>;
  /** The key's value as the settings files combine it. */
  readonly settingsValue: string | undefined;
  /**
   * The `availableModels` list as the files combine it (`undefined` when none sets it). A flag such
   * as `--model` "picks only from the models your organization allows" (settings.md, "Settings
   * precedence"); `[]` blocks every named selection (model-config.md, "Default model behavior").
   */
  readonly availableModels?: readonly string[] | undefined;
}

/** One place a key's value can come from, as a reader of the session input. */
export interface OverrideSource {
  readonly name: string;
  readonly read: (input: SessionInput) => string | undefined;
  /** A value the source reads counts only when this says so; absent, any value counts. */
  readonly admit?: (value: string, input: SessionInput) => boolean;
}

/** The keys with a per-session override. */
export type OverriddenKey =
  | 'agent'
  | 'teammateMode'
  | 'processWrapper'
  | 'model'
  | 'effortLevel'
  | 'modelSettings'
  | 'permissions.defaultMode'
  | 'autoCompactWindow'
  | 'autoConnectIde';

/** The keys with a per-session override, each with its sources highest first. */
export const SESSION_OVERRIDES: Readonly<Record<OverriddenKey, readonly OverrideSource[]>> =
  call('SESSION_OVERRIDES');

export interface SessionValue {
  readonly value: string;
  /** Which source supplied it. */
  readonly from: string;
}

/** The value a session uses: the first source that has one. */
export const resolveSessionValue = (
  key: OverriddenKey,
  input: SessionInput,
): SessionValue | undefined => call('resolveSessionValue', key, input);
