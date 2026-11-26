/**
 * What a host launches for a core stdio server or hook handler (core §7.2.1, §7.3.3, §9): the
 * command as one token, `./` resolved against the plugin root and a bare name left to the
 * platform's search; the arguments expanded; the working directory (the plugin root unless a
 * server's `cwd` says otherwise; always the root for a handler); and the environment: the host's
 * base, then a server's `env` expanded, then `PLUGIN_ROOT` and `PLUGIN_DATA`, each replacing an
 * entry of the same name by the platform's rule (case-insensitive on Windows).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/subprocess.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PathPlatform } from '../path-text.ts';
import type { CoreHookHandler } from './hooks.ts';
import type { CoreStdioServer } from './mcp-server.ts';
import type { CorePaths } from './placeholders.ts';

const call = coreModule('format/core/subprocess');

export interface CoreProcess {
  readonly command: string;
  readonly args: readonly string[];
  readonly cwd: string;
  readonly env: Readonly<Record<string, string>>;
}

export interface CoreProcessInput {
  /** The resolved plugin root and the instance's data directory (core §9.1). */
  readonly paths: CorePaths;
  /** The base environment the host chose (core §9.1 lets it inherit, omit or sanitise). */
  readonly base?: Readonly<Record<string, string>>;
  readonly platform?: PathPlatform;
}

/** The subprocess environment of core §9.1, `configured` already expanded. */
export const coreEnvironment = (
  input: CoreProcessInput,
  configured: Readonly<Record<string, string>> = {},
): Readonly<Record<string, string>> => call('coreEnvironment', input, configured);

/** A hook handler's process (core §7.3.3, §9.2: only `args` is expanded). */
export const coreHandlerProcess = (
  handler: CoreHookHandler,
  input: CoreProcessInput,
): CoreProcess => call('coreHandlerProcess', handler, input);

/**
 * A stdio server's process (core §7.2.1, §9.2: `args`, `env` values and `cwd` are expanded), or why
 * its `cwd` leaves its base once the real paths are known.
 */
export const coreServerProcess = (
  server: CoreStdioServer,
  input: CoreProcessInput,
): { readonly process: CoreProcess } | { readonly problem: string } =>
  call('coreServerProcess', server, input);
