/**
 * The two managed settings that narrow where hooks may come from (hooks.md, "Hook locations" and
 * "Limits on skill hooks"; settings-reference.md, "allowManagedHooksOnly" and
 * "strictPluginOnlyCustomization.hooks"). Each is a gate per source kind, typed over the kinds.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/lock_gates.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { RegisteredHandler } from '../contracts.ts';

const call = coreModule('hooks/select/lock-gates');

/**
 * Whether a plugin-only lock on `hooks` leaves the handler's source alone. It reads the source, not
 * the session, so a host can tell which registered handlers a lock silences.
 */
export const pluginOnlyAllows = (entry: RegisteredHandler): boolean =>
  call('pluginOnlyAllows', entry);
