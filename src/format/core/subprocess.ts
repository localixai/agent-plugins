import type { PathPlatform } from '../path-text.ts';
import { relativeCommandPath } from './command.ts';
import type { CoreHookHandler } from './hooks.ts';
import type { CoreStdioServer } from './mcp-server.ts';
import { expandPlaceholders, resolveCwd, type CorePaths } from './placeholders.ts';

/**
 * What a host launches for a core stdio server or hook handler (core §7.2.1, §7.3.3, §9): the
 * command as one token, `./` resolved against the plugin root and a bare name left to the
 * platform's search; the arguments expanded; the working directory (the plugin root unless a
 * server's `cwd` says otherwise; always the root for a handler); and the environment: the host's
 * base, then a server's `env` expanded, then `PLUGIN_ROOT` and `PLUGIN_DATA`, each replacing an
 * entry of the same name by the platform's rule (case-insensitive on Windows).
 */

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

const sameName = (platform: PathPlatform, first: string, second: string): boolean =>
  platform === 'windows' ? first.toUpperCase() === second.toUpperCase() : first === second;

function overlay(
  env: Readonly<Record<string, string>>,
  entries: readonly (readonly [string, string])[],
  platform: PathPlatform,
): Record<string, string> {
  const result: Record<string, string> = { ...env };
  for (const [name, value] of entries) {
    for (const existing of Object.keys(result)) {
      if (sameName(platform, existing, name)) delete result[existing];
    }
    result[name] = value;
  }
  return result;
}

/** The subprocess environment of core §9.1, `configured` already expanded. */
export function coreEnvironment(
  input: CoreProcessInput,
  configured: Readonly<Record<string, string>> = {},
): Readonly<Record<string, string>> {
  const platform = input.platform ?? 'posix';
  const own = overlay(input.base ?? {}, Object.entries(configured), platform);
  const reserved = [
    ['PLUGIN_ROOT', input.paths.root],
    ['PLUGIN_DATA', input.paths.data],
  ] as const;
  return overlay(own, reserved, platform);
}

/** The command a host runs: a `./` path made absolute under the root, a bare name as written. */
function commandOf(command: string, paths: CorePaths): string {
  const relative = relativeCommandPath(command);
  return relative === undefined ? command : `${paths.root}/${relative}`;
}

const expandAll = (values: readonly string[] | undefined, paths: CorePaths): string[] =>
  (values ?? []).map((value) => expandPlaceholders(value, paths));

/** A hook handler's process (core §7.3.3, §9.2: only `args` is expanded). */
export function coreHandlerProcess(handler: CoreHookHandler, input: CoreProcessInput): CoreProcess {
  return {
    command: commandOf(handler.command, input.paths),
    args: expandAll(handler.args, input.paths),
    cwd: input.paths.root,
    env: coreEnvironment(input),
  };
}

/**
 * A stdio server's process (core §7.2.1, §9.2: `args`, `env` values and `cwd` are expanded), or why
 * its `cwd` leaves its base once the real paths are known.
 */
export function coreServerProcess(
  server: CoreStdioServer,
  input: CoreProcessInput,
): { readonly process: CoreProcess } | { readonly problem: string } {
  const cwd = server.cwd === undefined ? undefined : resolveCwd(server.cwd, input.paths);
  if (cwd !== undefined && 'problem' in cwd) return { problem: `"cwd" ${cwd.problem}` };
  const env = Object.fromEntries(
    Object.entries(server.env ?? {}).map(([name, value]) => [
      name,
      expandPlaceholders(value, input.paths),
    ]),
  );
  return {
    process: {
      command: commandOf(server.command, input.paths),
      args: expandAll(server.args, input.paths),
      cwd: cwd?.cwd.path ?? input.paths.root,
      env: coreEnvironment(input, env),
    },
  };
}
