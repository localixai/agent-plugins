import type { PluginIssue } from '../format/issues.ts';
import type { LinkRoots } from '../format/path-escape.ts';
import type { PathPlatform } from '../format/path-text.ts';
import type { PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What stops a plugin before anything of it is read (spec §17.18.3, §17.18.5): its own directory
 * that the system could not check (`plugin path could not be checked: . (<code>)`), and a
 * `.claude-plugin/plugin.json` the kit cannot confirm is inside the plugin, or for a plugin of a
 * marketplace inside that marketplace: `Plugin not loaded: its manifest <path> was not read,
 * because <reason>`, the reason ending in the error code when the path could not be checked. The
 * component name `plugin` and the reason `it could not be checked (<code>)` are judgments
 * (spec/judgments.md): the specification gives the form of the message, not these words. A link is
 * judged only when the host passes `roots`, as for every other plugin path.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/manifest_gate.rs`, docs/RUST.md).
 */
const call = coreModule('loading/manifest-gate');

interface GateOptions {
  readonly roots?: LinkRoots;
  readonly platform?: PathPlatform;
  readonly signal?: AbortSignal;
}

/** The issue that keeps the whole plugin from loading, or `undefined` when it may load. */
export const manifestGate = (
  files: PluginFiles,
  options: GateOptions = {},
): Promise<PluginIssue | undefined> => call.async('manifestGate', files, options);
