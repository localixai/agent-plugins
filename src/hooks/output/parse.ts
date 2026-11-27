/**
 * Reading one JSON value as the output of a hook (hooks.md, "JSON output", "Exit code 0"): "JSON
 * that can't be parsed or fails schema validation" is a non-blocking error, and the message names
 * what is wrong so the author can fix it. An unknown key is dropped; a value of the wrong type fails.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/parse.rs`, docs/RUST.md).
 */
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ParseOutput } from '../verdict/types.ts';
import type { CheckedOutput } from './schemas.ts';

const call = coreModule('hooks/output/parse');

export interface CheckedResult {
  readonly data?: CheckedOutput;
  readonly failure?: string;
}

export const checkOutput = (event: HookEvent, json: unknown): CheckedResult =>
  call('checkOutput', event, json);

/**
 * The output stage's `parse`: the checked output, or why it failed. A key the schema does not name
 * is ignored and listed in `unrecognizedKeys`, for the debug line `Hook JSON output had
 * unrecognized keys` (Agent Plugins Extended §7.4.5.1 item 1, Judgment J7.30).
 */
export const parseHookOutput: ParseOutput = (event, json) => call('parseHookOutput', event, json);
