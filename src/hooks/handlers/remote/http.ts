import { coreModule } from '../../../wasm/bridge.ts';
import type { HandlerEntry, HandlerResult, RunContext } from '../context.ts';

/**
 * An `http` hook (hooks.md, "HTTP hook fields"): "Claude Code sends the hook's JSON input as the
 * POST request body with `Content-Type: application/json`". The run is the raw answer; what a
 * status or a body means ("HTTP response handling") is the interpreter's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/http.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/handlers/remote/http');

export const runHttp = (entry: HandlerEntry<'http'>, context: RunContext): Promise<HandlerResult> =>
  call.async('runHttp', entry, context);
