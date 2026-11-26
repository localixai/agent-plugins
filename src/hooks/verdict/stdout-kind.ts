/**
 * What a hook's stdout is (hooks.md, "Exit code 0"): "For a hook that isn't async, your stdout
 * counts as JSON output only when you print one JSON object, on one line or several, with nothing
 * around it but whitespace. Claude Code reads any other stdout as plain text or a parse failure."
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/stdout_kind.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/verdict/stdout-kind');

export interface StdoutReading {
  /** The stdout without the whitespace around it. */
  readonly text: string;
  /** One JSON object, to be checked against the output schema. */
  readonly json?: object;
  /** Why it is a parse failure: it looks like JSON and is not one object. Neither `json` nor this: plain text or nothing. */
  readonly parseFailure?: string;
}

export const readStdout = (stdout: string): StdoutReading => call('readStdout', stdout);

/**
 * An `http` hook's body (hooks.md, "HTTP response handling"): "2xx with an empty body: success";
 * "2xx with a JSON object body: parsed using the same JSON output schema"; "2xx with any other body,
 * such as plain text: non-blocking error".
 */
export interface BodyReading {
  readonly empty: boolean;
  readonly json?: object;
  /** Text that is not a JSON object (plain text, an array, a quoted string, bad JSON). */
  readonly notObject?: string;
}

export const readBody = (body: string): BodyReading => call('readBody', body);

/**
 * An async hook's response (hooks.md, "How async hooks execute"): "Alone on stdout: when the
 * response is the only text on stdout, it can span several lines" or "On one line of its own: an
 * async hook can print other text to stdout when the response fits on one line by itself". Judgment
 * (spec/judgments.md): when several lines are objects, the last one is the response.
 */
export const readAsyncResponse = (stdout: string): object | undefined =>
  call('readAsyncResponse', stdout);
