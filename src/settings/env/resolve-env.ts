/**
 * What the `env` blocks of the settings files come to for a session (settings-reference.md, `env`:
 * "How `env` values interact with your shell", "When Claude Code applies `env` values", "Variables
 * Claude Code ignores in `env`"; env-vars.md, "Set variables in settings files" and "Precedence").
 * Pure over data: the host passes its environment and what it knows of trust, and writes the result
 * into the process it starts.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/env/resolve_env.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyLayer, PolicyScope } from '../layers.ts';
import type { IgnoreReason } from './ignored-variables.ts';

const call = coreModule('settings/env/resolve-env');

export interface EnvTrust {
  /** An interactive session; `claude -p` and an SDK session never show the trust dialog. */
  readonly interactive: boolean;
  /** The user accepted the trust dialog for the folder or a parent folder whose trust extends to it. */
  readonly trusted: boolean;
}

export type Variables = Readonly<Record<string, string | undefined>>;

export interface EnvInput {
  /** The settings layers, read by `readSettingsLayer`. */
  readonly layers: readonly PolicyLayer[];
  readonly trust: EnvTrust;
  /** The environment `claude` was started from. */
  readonly shell: Variables;
  /**
   * The launch environment the Claude Desktop app or a self-hosted environment runner builds: it
   * "takes precedence instead", so no settings file overwrites a variable it sets.
   */
  readonly launchEnvironment?: Variables;
  /**
   * The variables "Claude Code classifies as safe, such as model selection, timeouts and limits, and
   * feature toggles", applied at startup from every settings file. The docs name no list, so the host
   * supplies the classification; by default none is safe and a repository's `env` waits for trust.
   */
  readonly isSafeAtStartup?: (name: string) => boolean;
  /** The host's variable names are case-insensitive (Windows): the lists and the environments compare without case. */
  readonly windows?: boolean;
}

/** Why a settings value did not reach the session. */
export type NotApplied = IgnoreReason | 'launch-environment' | 'shell-keeps-it' | 'awaiting-trust';

export interface NotAppliedVariable {
  readonly name: string;
  readonly scope: PolicyScope;
  readonly why: NotApplied;
}

export interface EnvResolution {
  /** What the files come to, by variable: the highest file's value. */
  readonly settings: Readonly<Record<string, string>>;
  /** The file each of `settings` came from. */
  readonly origin: Readonly<Record<string, PolicyScope>>;
  /** The environment Claude Code's own process reads. */
  readonly process: Readonly<Record<string, string>>;
  /** The environment every subprocess it starts inherits. */
  readonly subprocess: Readonly<Record<string, string>>;
  /** The values left out, with the reason; the debug log names each one. */
  readonly notApplied: readonly NotAppliedVariable[];
}

/**
 * "`NO_COLOR` and `FORCE_COLOR` set here reach only subprocesses": the interface colors of Claude
 * Code itself are chosen from the shell it was launched from.
 */
export const SUBPROCESS_ONLY: readonly string[] = call('SUBPROCESS_ONLY');

/** Whether the files of `scope` take effect at startup in this session, or only after the folder is trusted. */
export const appliesAtStartup = (scope: PolicyScope, trust: EnvTrust): boolean =>
  call('appliesAtStartup', scope, trust);

/**
 * The environment of a session: the shell's, with the `env` of the settings files written over it,
 * "the highest-precedence one" for a variable more than one file sets.
 */
export const resolveEnv = (input: EnvInput): EnvResolution => call('resolveEnv', input);

/**
 * Whether a variable counts as unset: "To cancel a shell export, set the variable to `""`. Claude Code
 * treats an empty value as unset for provider selection, and subprocesses inherit the empty value."
 */
export const isUnsetForProvider = (environment: Variables, name: string): boolean =>
  call('isUnsetForProvider', environment, name);
