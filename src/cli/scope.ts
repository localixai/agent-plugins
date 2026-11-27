import type { InstallScope } from '../marketplace/install-scopes.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { UsageError } from './result.ts';

/**
 * The scope grammar of the plugin commands (plugins/cli-reference.md, "claude plugin commands":
 * "`--scope` takes `user`, `project`, or `local`, and names the settings file the command writes
 * to. `update` also takes `managed`"; each command's `--scope` row; "plugin enable"; "plugin
 * update", "Which scope the command updates"; "claude plugin marketplace commands": "their `--scope`
 * flag has no `-s` short form"). Pure: which scope a command works on from the flag and the scopes a
 * plugin is declared in; the host reads and writes the settings files.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/scope.rs`, docs/RUST.md).
 */

const call = coreModule('cli/scope');

/** The values `--scope` takes on every command that takes it. */
export const CONFIG_SCOPES: readonly ['user', 'project', 'local'] = call('CONFIG_SCOPES');
export type ConfigScope = (typeof CONFIG_SCOPES)[number];

/** `update` also takes `managed`, the one scope that can be updated but not installed to. */
export type CommandScope = InstallScope;

/** The commands that take `--scope`. */
export const SCOPED_COMMANDS: readonly [
  'install',
  'uninstall',
  'enable',
  'disable',
  'update',
  'prune',
  'marketplace-add',
  'marketplace-remove',
] = call('SCOPED_COMMANDS');
export type ScopedCommand = (typeof SCOPED_COMMANDS)[number];

/** What a command does when `--scope` is omitted. */
export type ScopeChoice =
  | { readonly scope: CommandScope }
  /** The scope is found from where the plugin is declared (`detectScope`). */
  | { readonly autoDetect: true }
  /** The command acts on every scope. */
  | { readonly allScopes: true };

/** How the help spells the flag: `-s, --scope <scope>`, or `--scope <scope>` for a marketplace command. */
export const scopeFlagSpelling = (command: ScopedCommand): string =>
  call('scopeFlagSpelling', command);

/** The commands that write to one scope, `user` when `--scope` is omitted. */
export type DefaultedCommand = 'install' | 'uninstall' | 'prune' | 'marketplace-add';

/**
 * The scope `command` works on: the flag's value, or what the command does without it. A value the
 * command does not take is a usage error ("An invalid `--scope`": no result line, exit `1`, the
 * reason on stderr; the wording is the kit's, spec/judgments.md).
 */
export function chooseScope(
  command: DefaultedCommand,
  given: string | undefined,
): { readonly scope: ConfigScope } | UsageError;
export function chooseScope(
  command: ScopedCommand,
  given: string | undefined,
): ScopeChoice | UsageError;
export function chooseScope(
  command: ScopedCommand,
  given: string | undefined,
): ScopeChoice | UsageError {
  return call('chooseScope', command, given);
}

/**
 * The scope an omitted `--scope` resolves to: "the most specific scope it's installed at ... checking
 * local, project, user, then managed" (`update`); `enable` and `disable` check "local, project,
 * user" and use "the first scope that mentions the plugin". `undefined` when none does.
 */
export const detectScope = (
  command: ScopedCommand,
  declaredAt: readonly CommandScope[],
): CommandScope | undefined => call('detectScope', command, declaredAt);

export type ScopeWrite =
  | { readonly kind: 'write'; readonly scope: CommandScope }
  /** "Claude Code writes an override at the scope you passed." */
  | { readonly kind: 'override'; readonly scope: CommandScope }
  | { readonly kind: 'refused'; readonly message: string }
  | { readonly kind: 'not-installed' };

/**
 * `--scope` given to `enable` or `disable` for a plugin (plugins/cli-reference.md, "plugin enable"):
 * where the plugin is declared the scope is written; "a scope that takes precedence over the
 * declaring one" writes an override; "any other scope" fails with `Plugin "<name>" is installed at
 * <scope> scope, not <given>. Use --scope <scope> or omit --scope to auto-detect.`, naming the scope
 * auto-detect would pick. A plugin no scope declares is the caller's `not-installed`.
 */
export const writeScope = (
  plugin: string,
  requested: CommandScope,
  declaredAt: readonly CommandScope[],
): ScopeWrite => call('writeScope', plugin, requested, declaredAt);
