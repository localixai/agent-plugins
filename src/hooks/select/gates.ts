import { coreModule } from '../../wasm/bridge.ts';
import type { HookSession, RegisteredHandler } from '../contracts.ts';

/**
 * What keeps a registered handler from running whatever the event says: `disableAllHooks`, workspace
 * trust, a plugin that is not enabled and the two managed locks (`lock-gates.ts`). Each is the docs'
 * rule, applied to the handler's source.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/gates.rs`, docs/RUST.md).
 */
const call = coreModule('hooks/select/gates');

/** Whether the handler may run in this session at all. */
export const passesGates = (entry: RegisteredHandler, session: HookSession): boolean =>
  call('passesGates', entry, session);
