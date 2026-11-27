/**
 * A command hook ready for the host's `CommandRunner` (plugins/manifest-reference.md, "Where each
 * variable resolves"; hooks.md, "Exec form and shell form").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/hook_command.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { CommandSpec, ShellKind } from '../../ports/index.ts';
import type { HookHandlerOf } from '../../protocol/hook-config.ts';
import type { SubstitutionScope } from './scope.ts';

const call = coreModule('loading/resolve/hook-command');

/** A `command` hook handler, as the official type has it. */
export type HookCommandHandler = HookHandlerOf<'command'>;

export interface HookCommandOptions {
  /** Where the hook sits, for the issues (`hooks.PostToolUse[0].hooks[1]`). Default `hook`. */
  readonly path?: string;
  /**
   * The shell a hook with no `shell` field runs in on this host. hooks.md: "Defaults to `bash`, or
   * to `powershell` on Windows when Git Bash isn't installed"; the kit cannot tell (L3), so a
   * Windows host without Git Bash says `powershell`. Default `bash`.
   */
  readonly defaultShell?: ShellKind;
  /**
   * False for a hook of no plugin (settings, a skill's frontmatter): `${user_config.*}` stays as
   * written in both forms, shell form is not refused for it (plugins/manifest-reference.md:
   * "Plugin hooks additionally substitute `${user_config.*}` values"), and `environment` holds only
   * `CLAUDE_PROJECT_DIR`. Default true.
   */
  readonly ofPlugin?: boolean;
  /** The hook's event, which the path-with-a-space notification names (`Stop hook needs quotes ...`). */
  readonly event?: string;
}

/**
 * A command hook ready for the host's `CommandRunner` (plugins/manifest-reference.md, "Where each
 * variable resolves"; hooks.md, "Exec form and shell form"). The kit never spawns it: the host runs
 * `spec` with `environment` added to the process, in the working directory and with the timeout
 * the hooks engine decides.
 */
export interface ResolvedHookCommand {
  readonly spec: CommandSpec;
  /**
   * The three path variables and `CLAUDE_PLUGIN_OPTION_<KEY>` for every option with a value,
   * sensitive ones included, in both forms (only `CLAUDE_PROJECT_DIR` for a hook of no plugin) (hooks.md: "both export them ... on the spawned process").
   */
  readonly environment: Readonly<Record<string, string>>;
}

export interface HookCommandResolution {
  /** Undefined when the hook does not run. */
  readonly resolved: ResolvedHookCommand | undefined;
  /**
   * Set when the hook runs but would start no program (`NO_PROGRAM`): the run is a failure to
   * start, with this message. Undefined for a hook that resolved or was refused.
   */
  readonly notStarted?: string;
  /**
   * Set when an `asyncRewake` shell-form hook does not run because an unquoted path placeholder
   * resolves to a path with a space: the notification for the conversation view (errors.md, "Hook
   * needs quotes around a path with a space").
   */
  readonly notice?: string;
  readonly issues: readonly PluginIssue[];
}

/**
 * Resolves one command hook. With `args` it is exec form: `command` and each element are
 * substituted as plain strings, `${user_config.*}` included, and `shell` is ignored. Without `args`
 * it is shell form: in a plugin's hook `${user_config.*}` anywhere in `command` is an error and the
 * hook does not run, whatever is configured (the check reads the template); the way out is the
 * `CLAUDE_PLUGIN_OPTION_<KEY>` variable in `environment`, or exec form. A command PowerShell would
 * only print does not start (`notStarted`).
 */
export const resolveHookCommand = (
  handler: HookCommandHandler,
  scope: SubstitutionScope,
  options: HookCommandOptions = {},
): Promise<HookCommandResolution> => call.async('resolveHookCommand', handler, scope, options);
