import type { RegisteredHandler } from '../hooks/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SessionCore } from './session-core.ts';
import type { HookHandle } from './session-hook-scope.ts';

/**
 * The hooks of a session and every change made to them: what the state declares, what a skill, an
 * agent or the host's run registers, what an update keeps and what a `once` run spends. Nothing
 * else writes the core's registry.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_registry.rs`,
 * docs/RUST.md).
 */

const call = coreModule('host/session-registry');

/**
 * Registers `entries` under `prefix`, replacing the ones the prefix registered before ("replaces,
 * never stacks", as a skill invoked again does). An entry the state the session last took up would
 * drop at an update (its plugin no longer runs, or the plugin-only lock silences it) is not
 * registered, as docs/API.md ("Updating") keeps scoped hooks "unless their pluginId no longer
 * runs". The handle removes them unless the prefix was registered again since.
 */
export const registerEntries = (
  core: SessionCore,
  prefix: string,
  entries: readonly RegisteredHandler[],
): HookHandle => call('registerEntries', core.handle, prefix, entries);

/**
 * Drops the `once` registrations a run spent (`OnceClaims`): by serial, so a hook registered again
 * under the same id while the run was in flight stays.
 */
export function dropSpent(core: SessionCore): void {
  const spent = core.registry
    .get()
    .entries.filter((entry) => core.onceClaims.spent(entry))
    .flatMap((entry) => core.registry.serialOf(entry) ?? []);
  if (spent.length > 0) call('dropSpent', core.handle, spent);
}
