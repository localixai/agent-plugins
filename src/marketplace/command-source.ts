/**
 * Running a `command` source (plugins/marketplace-reference.md, "command plugin source"): the
 * command goes through the `CommandRunner` port, from the user's home directory; what it printed
 * and the directory it names are judged here, the copy or the link itself is the host's. Every
 * failure the docs list is a `PluginIssue` on `source`; nothing throws.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/command_source.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { CommandRunner } from '../ports/command-runner.ts';
import type { Environment } from '../ports/environment.ts';
import type { PluginFiles } from '../ports/files.ts';
import type { Locations } from '../ports/locations.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginDirectories } from './ports.ts';
import type { CommandFetchPlan } from './types.ts';

const call = coreModule('marketplace/command-source');

export interface CommandSourcePorts {
  readonly runner: CommandRunner;
  /** The environment the command inherits ("Claude Code's subprocess environment"). */
  readonly environment: Environment;
  /** `homeDir()` is the directory the command runs from. */
  readonly locations: Locations;
  readonly directories: PluginDirectories;
}

export interface CommandSourceSetting {
  readonly platform: 'posix' | 'windows';
  /** The directory the session was started in: a source may not print it or a parent of it. */
  readonly sessionCwd?: string;
}

/** What a successful run leaves for the host: the directory to copy or link, as read-only files. */
export interface FetchedCommandPlugin {
  readonly directory: string;
  readonly files: PluginFiles;
  readonly mode: CommandFetchPlan['mode'];
}

/** Runs the command and judges what it printed; the fetched plugin, or the issue that refuses it. */
export const runCommandSource = (
  plan: CommandFetchPlan,
  ports: CommandSourcePorts,
  setting: CommandSourceSetting,
  options: { readonly signal: AbortSignal },
): Promise<FetchedCommandPlugin | PluginIssue> =>
  call.async('runCommandSource', plan, ports, setting, options);
