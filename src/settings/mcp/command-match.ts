/**
 * A `serverCommand` entry against a server (managed-mcp.md, "How `serverCommand` entries match"):
 * "Every argument, in order", so `["npx", "-y", "server"]` matches neither `["npx", "server"]` nor
 * `["npx", "-y", "server", "--flag"]`. The `env` block is not compared: the caller passes the command
 * and arguments only. Both sides are `${VAR}`-expanded before they are compared ("Environment
 * variables in `serverCommand` and `serverUrl` entries"); `expandEntry` is the entry side, and the
 * server's command arrives expanded from the live environment.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/command_match.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/command-match');

export const commandMatches = (
  entry: readonly string[],
  server: readonly string[],
  expandEntry: (text: string) => string,
): boolean => call('commandMatches', entry, server, expandEntry);
