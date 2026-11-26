import type { ShellKind } from '../../ports/index.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { EnvFileStore } from './ports.ts';
import type { PowershellVariables } from './powershell-reader.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/env-file/files.rs`, docs/RUST.md).
 */
const call = coreModule('hooks/env-file/files');

/**
 * What a command hook asks for its `CLAUDE_ENV_FILE`: the path for this event and this hook's shell,
 * or none. `shell` is `undefined` for a hook in exec form, which runs no shell.
 */
export interface HookEnvFiles {
  pathFor(
    event: HookEvent,
    shell: ShellKind | undefined,
    signal: AbortSignal,
  ): Promise<string | undefined>;
}

/** The files of one session: what hooks write them, and what the shells that run later read from them. */
export interface EnvFiles extends HookEnvFiles {
  /** Run before an event's hooks: a `CwdChanged` clears what `CwdChanged` and `FileChanged` hooks wrote. */
  beforeEvent(event: HookEvent, signal: AbortSignal): Promise<void>;
  /**
   * The scripts a Bash command runs before itself, in order: a script `CLAUDE_ENV_FILE` named at
   * launch, the file of `SessionStart` and `Setup`, the file of `CwdChanged` and `FileChanged`
   * ("Each Bash command runs the file's contents as shell code before the command itself").
   */
  scripts(signal: AbortSignal): Promise<readonly string[]>;
  /** The variables a PowerShell command receives (`readPowershellVariables` over every script). */
  powershellVariables(
    options: PowershellOptions,
    signal: AbortSignal,
  ): Promise<PowershellVariables>;
}

export interface PowershellOptions {
  readonly windows: boolean;
  /** A sandboxed PowerShell command receives none of the variables. */
  readonly sandboxed: boolean;
}

export interface EnvFilesOptions {
  /** The session the files are of: every call to the store names it. */
  readonly sessionId: string;
  /**
   * The script `CLAUDE_ENV_FILE` named in the launch environment ("any script you set `CLAUDE_ENV_FILE`
   * to before launch"). Judgment (spec/judgments.md): it is a third source that a shell runs first;
   * the hooks of the session never write it, they get the session's own files.
   */
  readonly launchScript?: string | undefined;
}

export const createEnvFiles = (store: EnvFileStore, options: EnvFilesOptions): EnvFiles =>
  call('createEnvFiles', store, options);
