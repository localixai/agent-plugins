/**
 * Whether a tool call may run: the PreToolUse hooks, then the permission decision over the input
 * they return.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_tool_check.rs`, docs/RUST.md).
 */
import type { HookOutcome } from '../hooks/index.ts';
import type { PortFailure } from '../ports/failure.ts';
import type {
  ToolNames,
  DecideContext,
  Decision,
  PathResolver,
  PreparedCall,
} from '../rules/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { HookPart } from './session-hooks.ts';

const call = coreModule('host/session-tool-check');

/** What `checkTool` found for one tool call. */
export interface ToolCheck {
  /** What the PreToolUse hooks answered: their messages, context and effects. */
  readonly hook: HookOutcome<'PreToolUse'>;
  /** The input to run: the hook's `updatedInput`, unless the hook deferred or returned none. */
  readonly input: unknown;
  /** The permission decision over that input, with the hook's answer applied. */
  readonly decision: Decision;
  /**
   * Why the path of that input could not be resolved (symlinks that form a loop): "the Read, Edit,
   * and Write tools refuse the operation" (permissions.md, "Paths that can't be resolved or that
   * change"), so `decision` is a deny for `unresolvable-path`. Only with a `resolver`.
   */
  readonly refused?: PortFailure;
}

type PermissionContext = Omit<DecideContext, 'hook'>;

export interface ToolCheckPart {
  /**
   * Whether a tool call may run: the PreToolUse hooks, then the permission decision over the input
   * they return (hooks.md, "PreToolUse decision control": "Claude Code evaluates permission rules ...
   * against the input your hook returns, not the input Claude sent"; for `"defer"` it is ignored).
   * With `resolver`, the symlink check (permissions.md, "Symlinks") runs here over that input:
   * `prepareCall` (the call's own `resolvedPaths` are dropped, they named the input Claude sent), and
   * `resolveRuleDirectories` when `permissionContext.match.paths` has no `realDirectories`. Without
   * it, `call` is taken as the host prepared it.
   * With `SessionOptions.toolNames`, `call.toolName` is the host's name: the hooks and the rules read
   * the Claude Code name, `call.input` is taken as Claude Code's input for that tool.
   * The prompt, a deferred turn and the auto-mode review of a changed input stay the host's.
   */
  checkTool(
    call: PreparedCall,
    options: {
      readonly toolUseId: string;
      readonly permissionContext: PermissionContext;
      readonly signal: AbortSignal;
      readonly resolver?: PathResolver;
    },
  ): Promise<ToolCheck>;
}

/**
 * `names` reads the call's tool name in Claude Code's terms (`SessionOptions.toolNames`). The
 * permission context's three functions cross beside its data, as `decide`'s do.
 */
export function createToolCheckPart(
  hooks: Pick<HookPart, 'dispatch'>,
  names: () => ToolNames,
): ToolCheckPart {
  return {
    checkTool: (given, { toolUseId, permissionContext, signal, resolver }) => {
      const { requiresPermission, preapprovedFetchDomain, foreignArtifact, ...context } =
        permissionContext;
      return call.async(
        'checkTool',
        hooks,
        names(),
        given,
        { toolUseId, signal, resolver },
        context,
        requiresPermission,
        preapprovedFetchDomain,
        foreignArtifact,
      );
    },
  };
}
