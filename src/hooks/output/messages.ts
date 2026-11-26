import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { Read } from './merge-rules.ts';

/**
 * Where a hook's `systemMessage` goes (hooks.md, "JSON output": "Warning message shown to the
 * user"). `DirectoryAdded` is the one event that routes it by source: "`slash_command`: Claude Code
 * delivers the hook's `systemMessage` to Claude as context on the next conversation turn, rather
 * than showing it to you. `register_repo_root`: Claude Code writes `systemMessage` output ... to
 * the debug log only".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/messages.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/output/messages');

export interface RoutedMessages {
  /** Shown to the user, in registry order. */
  readonly toUser: readonly string[];
  /** Added to Claude's context on the next turn. */
  readonly toClaude: readonly string[];
}

export const routeMessages = (
  event: HookEvent,
  reads: readonly Read[],
  facts: object,
): RoutedMessages => call('routeMessages', event, reads, facts);
