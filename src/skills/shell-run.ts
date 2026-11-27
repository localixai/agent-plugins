/**
 * Running the commands a skill injects (skills.md, "Inject dynamic context", "How injected commands
 * run", "When an injected command fails", "Permission checks on injected commands"; errors.md, "No
 * shell command in a skill or command was run"). Every effect is the `CommandRunner` port; the
 * permission answer is the host's (`ShellPermission`), since the deny, ask and allow engine and the
 * grants of `allowed-tools` meet the session's rules there.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/shell_run.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { SkillShell } from '../format/skill-types.ts';
import type { ScratchFiles } from '../ports/index.ts';
import type { PermissionBehavior, PermissionMode } from '../protocol/official.ts';
import type { DocumentedTool } from '../protocol/tool-names.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SkillCommandRunner } from './shell-ports.ts';

const call = coreModule('skills/shell-run');

/** The tool a command runs through, as the permission rules name it. */
type ShellTool = Extract<DocumentedTool, 'Bash' | 'PowerShell'>;

export interface ShellCall {
  readonly tool: ShellTool;
  /**
   * The command after substitution with its paths written in, as data: the text a permission rule
   * is written against (skills.md: an `allowed-tools` rule "matches the exact command"). Under the
   * default `shellPaths` the shell runs the same command with each path a reference to its
   * environment, so a quote or `$(...)` in a folder name is never parsed (spec/judgments.md).
   */
  readonly command: string;
  /**
   * The skill's `allowed-tools` with `${CLAUDE_*}` substituted. "Deny and ask rules still override
   * `allowed-tools`": the host builds its answer from its rules, then these grants, then the mode.
   */
  readonly grants: readonly string[];
}

/** The host's answer for one command: what its permission rules and the skill's grants decide. */
export type ShellPermission = (call: ShellCall) => PermissionBehavior | Promise<PermissionBehavior>;

/** One command to run: what is checked, what the shell gets, and its placeholder (for messages). */
export interface PlannedCommand {
  readonly pattern: string;
  /** The command with its paths written in: what the refusal, the rules and the messages read. */
  readonly command: string;
  /** What the shell runs: `command` with its paths as the host's `shellPaths` writes them. */
  readonly shellCommand: string;
  /** The values of the references `shellCommand` holds, added to the command's environment. */
  readonly environment: Readonly<Record<string, string>>;
}

export interface InjectionContext {
  /** The command name with its slash, as `No shell command in <name> was run` quotes it. */
  readonly name: string;
  readonly shell: SkillShell;
  readonly cwd: string;
  readonly env: Readonly<Record<string, string>>;
  readonly grants: readonly string[];
  readonly mode: PermissionMode | undefined;
  /** A forked skill that sets `agent`: the permission check always aborts there. */
  readonly forkedWithAgent: boolean;
  readonly signal: AbortSignal;
  /** The `bashOutputMaxChars` setting: how much of a result arrives inline. */
  readonly bashOutputMaxChars?: number;
  /** Bare mode: a command that reaches its timeout stops instead of moving to the background. */
  readonly bare?: boolean;
}

export interface InjectionPorts {
  readonly runner: SkillCommandRunner;
  readonly permissions: ShellPermission;
  /** Where a result past the inline ceiling is saved ("a file path plus a short preview"). */
  readonly scratch?: ScratchFiles;
}

export type InjectionResults =
  | { readonly ok: true; readonly outputs: readonly string[] }
  | { readonly ok: false; readonly issue: PluginIssue };

/**
 * Runs the commands one after another, in order (they share the working directory). The first
 * failure aborts the whole invocation: "Claude never sees the skill content for that invocation".
 */
export const runInjections = (
  commands: readonly PlannedCommand[],
  context: InjectionContext,
  ports: InjectionPorts,
): Promise<InjectionResults> => call.async('runInjections', commands, context, ports);
