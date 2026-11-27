/**
 * The loader of an Extended client (Agent Plugins Extended §1.2, §1.3; docs/API.md, "Core
 * packages"): the form of the root is decided first, then a Claude Code plugin loads by `loadPlugin`
 * exactly as it does alone, and a core package by `loadCorePackage`. A dual package is a Claude Code
 * plugin: its core manifest, `mcp.json` and `hooks.json` are not read for loading (J1.2). A core
 * package's marketplace entry gives no components: what it configures is reported (J18.15).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/load_package.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { LoadPluginInput } from '../load-plugin.ts';
import type { LoadedPlugin } from '../plugin-model.ts';
import type { CoreLoadOptions, LoadedCorePackage } from './model.ts';

const call = coreModule('loading/core/load-package');

export type LoadedPackage =
  | {
      readonly form: 'claude-code';
      /** The root also carries a core manifest, which was not loaded (§1.3.1: say so in the details). */
      readonly dual: boolean;
      readonly loaded: LoadedPlugin;
    }
  | { readonly form: 'core'; readonly dual: false; readonly loaded: LoadedCorePackage };

export interface LoadPluginPackageInput extends LoadPluginInput, CoreLoadOptions {}

/** Loads a plugin root in the form §1.3 decides. Never throws. */
export const loadPluginPackage = (input: LoadPluginPackageInput): Promise<LoadedPackage> =>
  call.async('loadPluginPackage', input);
