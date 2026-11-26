/**
 * A merged `HookOutcome` as the JSON a Claude Agent SDK hook callback returns (agent-sdk/hooks.md,
 * "Outputs": "SDK callback hooks use the same JSON output format as Claude Code shell command
 * hooks"). The kit has already merged the plugins' hooks of the event into one outcome; this writes
 * that outcome back as one hook's answer, each field typed by the SDK's own output types.
 *
 * Judgment (spec/judgments.md): several hooks' `additionalContext` and `systemMessage` texts are
 * joined with a newline, terminal sequences with nothing; what the SDK output has no field for
 * (`PreCompact` instructions, background deliveries, notices) stays on the outcome for the host.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/sdk_output.rs`, docs/RUST.md).
 */
import { type HookOutcome } from '../hooks/index.ts';
import type { HookEvent, SyncHookJSONOutput } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('host/sdk-output');

/** The outcome of the plugins' hooks of one event as the answer of one SDK hook callback. */
export const toSdkHookOutput = (outcome: HookOutcome<HookEvent>): SyncHookJSONOutput =>
  call('toSdkHookOutput', outcome);
