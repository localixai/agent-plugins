/** Loading a core package and deciding the form of a plugin root (docs/API.md, "Core packages"). */
export { checkDualPackage } from './dual-check.ts';
export {
  detectPluginForm,
  PLUGIN_FORMS,
  type PluginForm,
  type PluginFormDecision,
} from './form.ts';
export { loadCorePackage } from './load-core.ts';
export {
  loadPluginPackage,
  type LoadedPackage,
  type LoadPluginPackageInput,
} from './load-package.ts';
export type {
  CoreLoadOptions,
  CorePackage,
  LoadCorePackageInput,
  LoadedCorePackage,
} from './model.ts';
