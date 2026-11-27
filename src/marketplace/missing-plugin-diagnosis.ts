import { coreModule } from '../wasm/bridge.ts';

/**
 * "Plugin doesn't appear or its skills don't show up" (plugins/troubleshooting.md): the three checks, in
 * the docs' order, and what to do next. Pure over what the host can read.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/missing_plugin_diagnosis.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/missing-plugin-diagnosis');

export interface MissingPluginFacts {
  readonly installed: boolean;
  readonly enabled: boolean;
  /** The Errors tab holds entries for the plugin. */
  readonly errors: readonly string[];
  /** Installed during this session, not yet reloaded. */
  readonly installedThisSession: boolean;
  /** Whose plugin it is, for the last step. */
  readonly ownPlugin: boolean;
}

export type Step =
  | { readonly step: 'confirm-installed-and-enabled'; readonly where: string }
  | { readonly step: 'read-errors-tab'; readonly errors: readonly string[] }
  | { readonly step: 'reload'; readonly command: '/reload-plugins' }
  | { readonly step: 'own-plugin'; readonly see: 'Plugin loads but its skills are missing' }
  | { readonly step: 'someone-elses-plugin'; readonly look: string };

/** The next step: the first check that fails, else what to do when the plugin loads with no error and still shows no skills. */
export const nextStep = (facts: MissingPluginFacts): Step => call('nextStep', facts);
