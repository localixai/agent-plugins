import type { HookHandlerType } from '../../protocol/hook-config.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { EventFactsOf } from '../contracts.ts';

/**
 * hooks.md, "Events that fire before MCP servers are available": `SessionStart` at launch and every
 * `Setup` event "skip their `mcp_tool` hooks without calling the tool", and "the debug log records
 * `mcp_tool hooks are not available for the 'SessionStart' hook event (no MCP client context)`".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/mcp_client.rs`,
 * docs/RUST.md).
 */
const call = coreModule('hooks/select/mcp-client');

/** Whether a handler of `type` is skipped on this occurrence of `event` for want of an MCP client. */
export const lacksMcpClient = <E extends HookEvent>(
  event: E,
  facts: EventFactsOf<E>,
  type: HookHandlerType,
): boolean => call('lacksMcpClient', event, facts, type);

/** What the skip is reported as: the docs' own debug-log line (judgment: a warning, spec/judgments.md). */
export const noMcpClientMessage = (event: HookEvent): string => call('noMcpClientMessage', event);
