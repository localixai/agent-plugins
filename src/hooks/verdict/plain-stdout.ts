/**
 * Plain stdout on exit 0 that reaches Claude (hooks.md, "Exit code 0"): "`UserPromptSubmit`,
 * `UserPromptExpansion`, `SessionStart`, and `PostModelSwitch`: Claude Code adds plain-text stdout
 * as context that Claude can see and act on. It doesn't add stdout it failed to parse. `PreCompact`:
 * your stdout becomes instructions for the compaction summary". On every other event it goes to the
 * debug log.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/plain_stdout.rs`, docs/RUST.md).
 */
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/verdict/plain-stdout');

export const PLAIN_STDOUT: { readonly [E in HookEvent]?: 'context' | 'compaction-instructions' } =
  call('PLAIN_STDOUT');
