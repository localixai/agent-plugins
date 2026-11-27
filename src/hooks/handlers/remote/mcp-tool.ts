import { coreModule } from '../../../wasm/bridge.ts';
import type { HookEvent } from '../../../protocol/official.ts';
import type { HandlerEntry, HandlerResult, RunContext } from '../context.ts';

/**
 * An `mcp_tool` hook (hooks.md, "MCP tool hook fields"). The `server` is passed as the hook writes
 * it: for a plugin-bundled server that is "the scoped name `plugin:<plugin-name>:<server-name>`,
 * not the bare server key". Running no hook on `SessionStart` at launch and on `Setup` (no MCP
 * client yet) is the select stage's (`crates/agent-plugins/src/hooks/select/mcp_client.rs`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/mcp_tool.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/handlers/remote/mcp-tool');

/**
 * "On events where a hook can block or change the result, such as `PreToolUse` or `Stop`, Claude
 * Code waits for a connecting server ... On observational events, such as `Notification` or
 * `SessionEnd`, it doesn't wait."
 *
 * Judgment (spec/judgments.md): the docs name four events, so the kit sorts the rest by the same
 * test. An event waits when exit code 2 can stop what it guards ("Can block?: Yes" in "Exit code 2
 * behavior per event") or its output decides a result: `PermissionRequest` (`decision`),
 * `PermissionDenied` (`retry`), `PostToolUse` (`updatedToolOutput`, `decision`), `MessageDisplay`
 * (`displayContent`). An event whose hooks only add context or watch (`SessionStart`,
 * `SubagentStart`, `PostToolUseFailure`) does not.
 */
export const WAITS_FOR_SERVER: { readonly [E in HookEvent]: boolean } = call('WAITS_FOR_SERVER');

export const runMcpTool = (
  entry: HandlerEntry<'mcp_tool'>,
  context: RunContext,
): Promise<HandlerResult> => call.async('runMcpTool', entry, context);
