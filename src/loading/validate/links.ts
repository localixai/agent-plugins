import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * What the validator says about links (oracle 2.1.319). It never follows a symlink: a folder that
 * is one is not read, an entry that is one is not read, and each warns, because Claude Code does
 * follow them in some places. A plugin's own folders and a project's `.claude` folders word it
 * differently, for a session always follows the links of a project it loads.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/links.rs`, docs/RUST.md).
 */
const call = coreModule('loading/validate/links');

export const isLink = (files: PluginFiles, path: string, signal?: AbortSignal): Promise<boolean> =>
  call.async('isLink', files, path, signal);

/** The hooks folder is a link, or `hooks.json` is not a regular file. */
export const HOOKS_LINK: { readonly folder: string; readonly file: string } = call('HOOKS_LINK');

export const MCP_NOT_REGULAR: string = call('MCP_NOT_REGULAR');

/** The most of an MCP server file the validator reads (oracle 2.1.325, spec J8.27). */
export const MCP_FILE_CAP_BYTES: number = call('MCP_FILE_CAP_BYTES');

export const MCP_PAST_CAP: string = call('MCP_PAST_CAP');
