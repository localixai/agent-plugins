import { coreModule } from '../../wasm/bridge.ts';
import type { PermissionRule } from '../types.ts';
import type { HostToolNames } from './host-tool-names.ts';

/**
 * Which of the tools the host can provide Claude sees, by the documented rules.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/availability.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/tools/availability');

/**
 * The facts of a session that decide which of the tools the host can provide Claude actually sees.
 * The kit holds no model, provider or surface table: the candidates a host passes are the tools its
 * model and surface offer (tools-reference.md, "Task tool availability", "EndConversation tool
 * behavior"), and this narrows them by the documented rules (spec/judgments.md).
 */
export interface ToolSession {
  /** WSL counts as `posix`: Glob and Grep are absent by default everywhere but Windows. */
  readonly platform: 'posix' | 'windows';
  /** Every deny and allow rule in force, whatever its source, including `--disallowedTools` and `--allowedTools` entries. */
  readonly rules: readonly PermissionRule[];
  /** The `--tools` list: "restricts the session's built-in tools to the ones it names". */
  readonly toolsFlag?: readonly string[];
  /** The `--allowedTools` entries (rule syntax), for the one thing they do to availability: restore Glob and Grep. */
  readonly allowedToolsFlag?: readonly string[];
  /**
   * A `--restricted` session removes the built-in tools that run commands or code and `WebFetch`,
   * unless `--tools` names them individually (cli-reference.md, `--restricted`).
   */
  readonly restricted?: boolean;
  /** Windows with Git Bash installed: a Bash deny rule then turns the PowerShell tool off. */
  readonly gitBash?: boolean;
  /** `CLAUDE_CODE_USE_POWERSHELL_TOOL=1`, in the environment or a settings file's `env` block. */
  readonly powerShellOptIn?: boolean;
  /** `permissions.blockWritesOutsideWorkingDirectories` is on: the worktree tools are removed. */
  readonly blockWritesOutsideWorkingDirectories?: boolean;
  /** `permissions.denyInsteadOfPrompting` is on: the question and plan tools are left out. */
  readonly denyInsteadOfPrompting?: boolean;
  /** The session is a teammate in an agent team: it keeps the plan tools under `denyInsteadOfPrompting`. */
  readonly teammate?: boolean;
  /**
   * How the candidates' names correspond to Claude Code's, when the host's tools carry other names:
   * the rules and flags read the Claude Code names, the outcome names the host's tools. Absent, the
   * candidates are Claude Code's names.
   */
  readonly names?: HostToolNames;
}

/**
 * The tools Claude sees, from the candidates the host can provide (order kept). A deny rule with no
 * specifier removes the tools it names, and so does a glob (permissions.md, "Manage permissions",
 * "Tool name wildcards"); a scoped or parameter rule, an ask rule and an allow rule remove nothing.
 * `EndConversation` is the exception (tools-reference.md, "EndConversation tool behavior"): no deny
 * rule and no `--tools` list removes it while another tool remains; `--tools ""` removes it "only
 * when no MCP tools remain" (cli-reference.md); when the deny rules remove every other tool and also
 * match it, as `*` does, it goes too. An allow rule naming `EndConversation` explicitly saves it in
 * both cases. With `session.names` the candidates are the host's tools, read through their Claude
 * Code names (judgment, spec/judgments.md).
 */
export const availableTools = (
  candidates: readonly string[],
  session: ToolSession,
): readonly string[] => call('availableTools', candidates, session);
