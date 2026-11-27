/**
 * The deprecated `claude-code-plugins` marketplace (plugins/anthropic-marketplaces.md, "Deprecated
 * `claude-code-plugins` marketplace"): what to run for an install that used to come from it and for the
 * two plugins that changed. Pure; the docs' commands.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/deprecated_marketplace.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/deprecated-marketplace');

/** Where a plugin of the deprecated marketplace is installed from now: "run `claude plugin install <plugin>@claude-plugins-official`". */
export const officialInstallCommand = (plugin: string): string =>
  call('officialInstallCommand', plugin);

/** The tutorial step to skip. */
export const SKIPPED_TUTORIAL_STEP: string = call('SKIPPED_TUTORIAL_STEP');

export interface DeprecatedPluginAdvice {
  /** What changed. */
  readonly change: 'renamed' | 'retired';
  /** The command to run in a shell. */
  readonly command: string;
  /** For a rename, the prefix the commands have now. */
  readonly commandPrefix?: string;
}

/** Advice for the two plugins that need it; `undefined` for every other: "you can leave your plugins and the marketplace as they are". */
export const deprecatedPluginAdvice = (plugin: string): DeprecatedPluginAdvice | undefined =>
  call('deprecatedPluginAdvice', plugin);
