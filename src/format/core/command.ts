/**
 * The `command` of a stdio MCP server or a hook handler (core §7.2.1, §7.3.1): one executable token,
 * never a shell command string, and either a bare executable name, resolved by the platform's
 * search, or a plugin-relative path that begins with `./`, resolved against the plugin root and
 * contained in it (core §4.1 item 4). No placeholder is expanded in it (core §9.2).
 *
 * Judgment (spec/judgments.md): a bare name is one token when it holds no whitespace and no path
 * separator, so `npx tsx`, `bin/server`, `/usr/bin/node`, `../server` and `${PLUGIN_ROOT}/server`
 * are refused; a `./` path is refused when it climbs with `..` or holds a backslash as written.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/command.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/core/command');

/** The root-relative path a `./` command names, or `undefined` for a bare name. */
export const relativeCommandPath = (command: string): string | undefined =>
  call('relativeCommandPath', command);

/** Why a `command` value breaks core §7.2.1 / §7.3.1, or `undefined` when it is a valid token. */
export const commandProblem = (command: unknown): string | undefined =>
  call('commandProblem', command);
