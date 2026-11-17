import { loadPlugin, type LoadPluginInput } from '../load-plugin.ts';
import type { LoadedPlugin } from '../plugin-model.ts';
import { coreEntryIssues } from './entry-fields.ts';
import { detectPluginForm, type PluginForm } from './form.ts';
import { loadCorePackage } from './load-core.ts';
import type { CoreLoadOptions, LoadedCorePackage } from './model.ts';

/**
 * The loader of an Extended client (Agent Plugins Extended §1.2, §1.3; docs/API.md, "Core
 * packages"): the form of the root is decided first, then a Claude Code plugin loads by `loadPlugin`
 * exactly as it does alone, and a core package by `loadCorePackage`. A dual package is a Claude Code
 * plugin: its core manifest, `mcp.json` and `hooks.json` are not read for loading (J1.2). A core
 * package's marketplace entry gives no components: what it configures is reported (J18.15).
 */

export type LoadedPackage =
  | {
      readonly form: 'claude-code';
      /** The root also carries a core manifest, which was not loaded (§1.3.1: say so in the details). */
      readonly dual: boolean;
      readonly loaded: LoadedPlugin;
    }
  | { readonly form: 'core'; readonly dual: false; readonly loaded: LoadedCorePackage };

export interface LoadPluginPackageInput extends LoadPluginInput, CoreLoadOptions {}

/** One loader per form: a registry over `PluginForm` (K1). */
const LOADERS: {
  readonly [F in PluginForm]: (
    input: LoadPluginPackageInput,
    dual: boolean,
  ) => Promise<LoadedPackage>;
} = {
  'claude-code': async (input, dual) => ({
    form: 'claude-code',
    dual,
    loaded: await loadPlugin(input),
  }),
  core: async (input) => {
    const loaded = await loadCorePackage({
      files: input.files,
      ...(input.extensions && { extensions: input.extensions }),
      ...(input.transports && { transports: input.transports }),
      ...(input.iconFormats && { iconFormats: input.iconFormats }),
      ...(input.signal && { signal: input.signal }),
    });
    const issues = [...loaded.issues, ...coreEntryIssues(input.entry)];
    return { form: 'core', dual: false, loaded: { ...loaded, issues } };
  },
};

/** Loads a plugin root in the form §1.3 decides. Never throws. */
export async function loadPluginPackage(input: LoadPluginPackageInput): Promise<LoadedPackage> {
  const decision = await detectPluginForm({ files: input.files, signal: input.signal });
  return LOADERS[decision.form](input, decision.dual);
}
