import type { PluginIssue } from '../format/issues.ts';
import type { PluginSourceOf } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The `mode` of a `command` source (plugins/marketplace-reference.md, "Copy mode and link mode").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/command_mode.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/command-mode');

/** Which `mode` Windows refuses. */
export const LINKS_NEED_POSIX: { readonly [M in PluginSourceOf<'command'>['mode']]: boolean } =
  call('LINKS_NEED_POSIX');

/** "Claude Code refuses to install a link-mode plugin on Windows. Declare `"mode": "copy"` there." */
export const linkOnWindowsIssue = (): PluginIssue => call('linkOnWindowsIssue');
