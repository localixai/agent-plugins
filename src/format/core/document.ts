/**
 * The outer object of `mcp.json` and `hooks.json` (core §7.2.1, §7.3.1, §10.1): exactly `$schema`
 * and one member (`mcpServers`, `hooks`) that is an object; a `$schema` the kit implements, of the
 * same Agent Plugins version as `plugin.json`. Any failure disables that component type only (core
 * §7.2.2 item 2, §7.3.2 item 2).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/document.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { CoreDocument, CoreVersion } from './versions.ts';

const call = coreModule('format/core/document');

/** The member the document holds, or why the whole component is disabled. */
export const documentMembers = (
  raw: unknown,
  document: Exclude<CoreDocument, 'plugin'>,
  member: string,
  manifestVersion: CoreVersion,
): { readonly members: Readonly<Record<string, unknown>> } | { readonly problem: string } =>
  call('documentMembers', raw, document, member, manifestVersion);
