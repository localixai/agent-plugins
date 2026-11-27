/**
 * The rules, the starting mode and the session facts of the permission settings as the input of
 * `decide` (docs/API.md, "Permissions in a session"): where the settings (layer 3) and the rules
 * (layer 3) meet. Pure over the settings policy and what the host knows of the launch.
 */
import type { PluginIssue } from '../format/issues.ts';
import type { PermissionMode } from '../protocol/official.ts';
import type { DecideContext, MatchContext, PathContext } from '../rules/index.ts';
import type { PermissionsPolicy, StartInput } from '../settings/index.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('host/permission-session');

/** Rules the launch adds: `--allowedTools` and `--disallowedTools` (permissions.md, "Settings precedence"). */
export interface CommandLineRules {
  readonly allowed?: readonly string[];
  readonly disallowed?: readonly string[];
}

export interface PermissionSessionInput {
  readonly policy: PermissionsPolicy;
  readonly start: StartInput;
  /** The folders rules are read against; the settings' `additionalDirectories` are added to the session's own. */
  readonly paths: PathContext;
  readonly match?: Omit<MatchContext, 'paths'>;
  readonly commandLine?: CommandLineRules;
  /** A `--permission-prompt-tool` or an SDK `canUseTool` callback answers what would prompt; not asked under `permissions.denyInsteadOfPrompting`. */
  readonly permissionCallback?: boolean;
  /**
   * Someone can answer a prompt in this session. Defaults to a `terminal` surface; a host with its
   * own approval interface (a web or desktop chat) says `true` without claiming to be a terminal,
   * which would also give it the built-in `auto` default (spec/judgments.md).
   */
  readonly interactive?: boolean;
}

export interface PermissionSession {
  readonly mode: PermissionMode;
  /** What `decide` takes besides the call: the mode, the rules and the facts the settings decide. */
  readonly context: DecideContext;
  /** The notices the session shows at its start, verbatim from the docs (`StartingMode.notices`). */
  readonly notices: readonly string[];
  readonly issues: readonly PluginIssue[];
}

export type StartedPermissions =
  | ({ readonly outcome: 'started' } & PermissionSession)
  | { readonly outcome: 'refused'; readonly reason: string };

/**
 * Starts the permission side of a session: the mode, then the context `decide` reads. A start the
 * settings refuse (`--dangerously-skip-permissions` with `disableBypassPermissionsMode`, a background
 * bypass request nobody accepted) comes back as `refused`.
 */
export const startPermissions = (input: PermissionSessionInput): StartedPermissions =>
  call('startPermissions', input);
