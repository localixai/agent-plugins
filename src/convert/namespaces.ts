/**
 * Every client extension namespace a core package carries (core §8), so a conversion loads the
 * package with all of them and reports each one instead of the loader dropping the namespaces the
 * host does not implement unseen (Agent Plugins Extended §18.4.1 item 1): the `extensions` keys of
 * `plugin.json`, of each `mcp.json` server and of each `hooks.json` handler, and every top-level
 * directory named as a reverse-domain namespace. Judgment (spec/judgments.md): a directory counts
 * when its name is two or more dot-separated labels of letters, digits and hyphens, the form core §8
 * gives namespaces (`com.example.client`); `assets/` never does.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/namespaces.rs`, docs/RUST.md).
 */
import type { PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('convert/namespaces');

export const extensionNamespaces = (
  files: PluginFiles,
  signal?: AbortSignal,
): Promise<readonly string[]> => call.async('extensionNamespaces', files, signal);

/** Whether `hooks/hooks.json` declares `modules`: the plugin is a mod (plugins/mods/reference.md, "Files"). */
export const declaresModules = (files: PluginFiles, signal?: AbortSignal): Promise<boolean> =>
  call.async('declaresModules', files, signal);
