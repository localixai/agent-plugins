/**
 * One runner per handler type (hooks.md, "Hook handler fields"): tells the host a hook started
 * (`statusMessage`), then runs it through the ports of its type.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/registry.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { RegisteredHandler } from '../contracts.ts';
import type { HandlerResult, RunContext } from './context.ts';

const call = coreModule('hooks/handlers/registry');

/** Runs one handler: tells the host it started (`statusMessage`), then hands it to the runner of its type. */
export const runHandler = (entry: RegisteredHandler, context: RunContext): Promise<HandlerResult> =>
  call.async('runHandler', entry, context);
