import { coreModule } from '../wasm/bridge.ts';
import type { InstallScope } from './install-scopes.ts';

/**
 * The way out of `Version: unknown` for a plugin installed from a git repository (plugins/
 * troubleshooting.md, "A plugin installed from a git repository shows version `unknown`"): Claude
 * Code could not read the commit it uses as the plugin's version. The four steps are the section's,
 * with the commands as it writes them; the plan is data a host shows or runs.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/version_troubleshooting.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/version-troubleshooting');

export interface VersionStep {
  readonly command: string;
  /** Set on the step that needs a Claude Code release: the first release that clones in a readable format. */
  readonly minimumClaudeCode?: string;
}

export interface UnknownVersionPlan {
  readonly steps: readonly VersionStep[];
  /** `claude plugin update` "changes one scope's block per run": one rerun for each block still showing `unknown`. */
  readonly rerun: readonly { readonly scope: InstallScope; readonly command: string }[];
}

/** A plugin as `<name>@<marketplace>`: the two parts the commands use, never split out of one string. */
export interface PluginRef {
  readonly name: string;
  readonly marketplace: string;
}

/**
 * The steps for a plugin and the scopes of the blocks that still show `unknown` afterwards. The
 * plugin is given as its two parts, so the marketplace of step 2 is always the plugin's own.
 * Judgments: a managed scope gets no rerun, as the user cannot update it; a scope listed twice is
 * rerun once.
 */
export const unknownVersionSteps = (
  plugin: PluginRef,
  unknownScopes: readonly InstallScope[],
): UnknownVersionPlan => call('unknownVersionSteps', plugin, unknownScopes);
