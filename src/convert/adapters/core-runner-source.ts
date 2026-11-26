/**
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/adapters/core_runner_source.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('convert/adapters/core-runner-source');

/**
 * The body of `run-core-handler.mjs` (Agent Plugins Extended §18.4.4): which Claude Code occurrences
 * are core occurrences (§18.1.2: `SessionStart` on `startup`, `clear` and `fork` outside a subagent,
 * `UserPromptSubmit` on every turn but a loop or schedule wakeup (§18.4.4 item 5, J18.23: `system`
 * also carries peer messages and task notifications, which a hook cannot tell apart), `PostToolUse` also on a failure that is no interrupt), the
 * core input of each, the run of the core handler as core §7.3.3 starts one (the plugin root as its
 * working directory, `PLUGIN_ROOT` and `PLUGIN_DATA` set, the data directory created), and its result
 * read by the closed shapes and written as Claude Code's (§18.2). A failure is a non-blocking error:
 * exit 1, never 2 (§18.4.4 item 3).
 */
export const CORE_RUNNER_SOURCE: string = call('CORE_RUNNER_SOURCE');
