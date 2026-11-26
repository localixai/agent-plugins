/**
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/adapters/claude_code_runner_source.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('convert/adapters/claude-code-runner-source');

/**
 * The body of `run-claude-code-handler.mjs` (Agent Plugins Extended §18.4.4): a Claude Code `command`
 * hook run under a core client. Its input is built from the core input by §18.2, leaving out what the
 * core does not supply; `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA` and `CLAUDE_PROJECT_DIR` (the core
 * input's `cwd`) are set; the hook runs in the core working directory, in exec form with the three
 * placeholders substituted per argument, or in shell form (`bash` through `$SHELL`, `pwsh` with the
 * placeholders as `$env:` references). Exit 2 becomes the block of its event, plain stdout context on
 * `SessionStart` and `UserPromptSubmit`; what the core cannot express is written to stderr, and the
 * adapter always exits 0 with one valid result or nothing (item 4). Judgment (spec/judgments.md): a
 * denial or a Stop block without a reason gets the reason `Blocked by a hook`, since the core's
 * closed shapes require one.
 */
export const CLAUDE_CODE_RUNNER_SOURCE: string = call('CLAUDE_CODE_RUNNER_SOURCE');
