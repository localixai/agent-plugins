/**
 * Where a symbolic link leads, judged against the plugin and its marketplace (plugins/host-
 * marketplace.md, "Share files within a marketplace with symlinks"; plugins/loading.md, "Paths
 * that escape the plugin directory"). Pure over paths the host resolved (real paths, `/`-separated,
 * the same spelling for the roots and the target): the kit looks nothing up.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/link_rule.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/link-rule');

/** What a link's resolved target is, relative to the plugin and, when it has one, its marketplace. */
export type LinkReach = 'within-plugin' | 'within-marketplace' | 'outside';

/**
 * Where a target resolves: "Within the plugin's own directory", "Elsewhere within the same
 * marketplace", or "Outside the marketplace". A plugin with no marketplace (a local path, a
 * `command` source in copy mode) has only its own directory to stay in.
 */
export const linkReach = (input: {
  readonly target: string;
  readonly pluginRoot: string;
  readonly marketplaceRoot?: string;
}): LinkReach => call('linkReach', input);

/** `rel` (a path below the plugin root) as a path from the folder `fromDirectory` (also below it). */
export const relativeFrom = (fromDirectory: string, rel: string): string =>
  call('relativeFrom', fromDirectory, rel);
