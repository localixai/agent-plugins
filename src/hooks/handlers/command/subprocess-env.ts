/**
 * The environment of a subprocess Claude Code starts (env-vars.md, "Set variables in settings files",
 * "Precedence", `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`, `CLAUDECODE`, `CLAUDE_CODE_CHILD_SESSION`,
 * `CLAUDE_CODE_SESSION_ID` and `CLAUDE_PID`; hooks.md, "Common input fields"): the host's environment
 * with the settings files' `env` written over it, its credentials removed when the scrub is on (and
 * the Anthropic ones under the HIPAA configuration), the
 * `OTEL_*` exporter variables removed ("from every subprocess it spawns"), and the variables Claude
 * Code sets for that kind of subprocess. One pure function, so a host builds the environment of its
 * own Bash, PowerShell, stdio MCP and Monitor processes the way the kit builds a hook's.
 *
 * Which kind gets which variable is the docs' own list:
 *
 * | variable                    | hook | Bash | PowerShell | stdio MCP | Monitor |
 * | --------------------------- | ---- | ---- | ---------- | --------- | ------- |
 * | `CLAUDECODE`                | yes  | yes  | yes        | yes       | yes     |
 * | `CLAUDE_CODE_CHILD_SESSION` | yes  | yes  | yes        | no        | yes     |
 * | `CLAUDE_CODE_SESSION_ID`    | yes  | yes  | yes        | yes       | no      |
 * | `CLAUDE_PID`                | yes  | yes  | yes        | no        | no      |
 * | `CLAUDE_EFFORT`             | yes  | yes  | no         | no        | no      |
 *
 * Judgments (spec/judgments.md): `CLAUDECODE` is set for a Monitor process too (the docs list the
 * tools that get it by example); a variable the table does not give a kind is neither set nor
 * removed when the host's own environment holds it; `CLAUDE_ENV_FILE` of the host's environment
 * reaches no subprocess (a hook gets the file of its event from `processEnvironment`, a Bash command
 * runs the files); a variable of a session or process id the host did not give is not set.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/command/subprocess_env.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('hooks/handlers/command/subprocess-env');

export const SUBPROCESS_KINDS: readonly ['hook', 'bash', 'powershell', 'mcp', 'monitor'] =
  call('SUBPROCESS_KINDS');
export type SubprocessKind = (typeof SUBPROCESS_KINDS)[number];

export interface SubprocessEnvironmentInput {
  readonly kind: SubprocessKind;
  /** The host process's environment. */
  readonly environment: Readonly<Record<string, string>>;
  /** `resolveSettingsEnv(...).values`: written over `environment`. */
  readonly settingsEnv?: Readonly<Record<string, string>>;
  /** The host's variable names are case-insensitive. */
  readonly windows: boolean;
  /** The session's id, for `CLAUDE_CODE_SESSION_ID`. */
  readonly sessionId?: string | undefined;
  /** The host process's id, for `CLAUDE_PID`. */
  readonly processId?: number | undefined;
  /** The effort level when the process starts, for `CLAUDE_EFFORT` ("only set when the model supports" it). */
  readonly effort?: string | undefined;
  /** The HIPAA configuration applies to the session: the Anthropic credentials are removed. */
  readonly hipaa?: boolean | undefined;
}

/** The environment a subprocess of this kind inherits, with what Claude Code sets for it. */
export const subprocessEnvironment = (input: SubprocessEnvironmentInput): Record<string, string> =>
  call('subprocessEnvironment', input);
