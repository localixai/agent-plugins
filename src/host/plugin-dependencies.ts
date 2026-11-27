import type { DependencyIssue } from '../marketplace/dependencies.ts';
import { coreModule } from '../wasm/bridge.ts';
import { givenOf } from './core-view.ts';
import type { HostedPlugin, HostPlugin } from './types.ts';

/**
 * Which plugins stay off for a dependency (plugins/troubleshooting.md, "Dependency errors"; the core's
 * `host/plugin_dependencies.rs` holds the rules). Each plugin crosses as the core's copy of it
 * (`givenOf`), as the host's state takes it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/plugin_dependencies.rs`, docs/RUST.md).
 */

const call = coreModule('host/plugin-dependencies');

/**
 * The load-time dependency verdict without a host or a session (docs/API.md, "Host and sessions"):
 * each plugin given with whether it is on, and back whether it runs once every plugin whose
 * dependency is not met is off, transitively, with one error per such plugin under its id (the
 * `errorType` of `claude plugin list --json`). A host gives every installed plugin, a disabled one
 * with `enabled: false`; one not given reads as not installed. The sessions use the same verdict.
 */
export const pluginsWithDependencies = (
  plugins: readonly Required<Pick<HostPlugin, 'id' | 'plugin' | 'enabled'>>[],
): { readonly plugins: readonly HostedPlugin[]; readonly issues: readonly DependencyIssue[] } =>
  call(
    'pluginsWithDependencies',
    plugins.map(({ id, plugin, enabled }) => ({ id, plugin: givenOf(plugin).handle, enabled })),
  );
