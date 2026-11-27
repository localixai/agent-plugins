/**
 * Loading a core package: one crossing per load, the host's files read through one load's view of
 * them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/load_core.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { LoadCorePackageInput, LoadedCorePackage } from './model.ts';

const call = coreModule('loading/core/load-core');

/**
 * Loads a core package (Agent Plugins 1.1.0, core §4–§10; docs/API.md, "Core packages"): the root
 * `plugin.json` first, which rejects the package when it fails (core §5.1–§5.3); then each portable
 * component type from its fixed location, each failing on its own narrowest boundary (core §4.1,
 * §6.2, §7.1, §7.2.2, §7.3.2, §11.3): an invalid type, server, event member, handler or skill is
 * reported and skipped while the rest loads. Nothing of the Claude Code layout is read: a core
 * package has no other component location (core §6.1). Never throws.
 */
export const loadCorePackage = (input: LoadCorePackageInput): Promise<LoadedCorePackage> =>
  call.async('loadCorePackage', input);
