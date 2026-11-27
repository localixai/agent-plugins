import { coreModule } from '../wasm/bridge.ts';

/**
 * Which of the user's plugins a session on another machine gets (desktop.md, "Install plugins",
 * "Windows remote machines", "Files and processes on the remote machine"): a session over SSH gets
 * the plugins copied from the user's computer to `~/.claude/remote/plugins/` and loads them from
 * there; on a Windows remote machine a plugin that "holds a file or folder name Windows can't store,
 * such as a name with `:`" is not copied and "the session lists the plugins that Desktop left out";
 * "Plugins aren't available in WSL sessions". Pure over the paths the host listed: copying, the SSH
 * connection and the list the session shows are the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/remote_plugins.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/remote-plugins');

/** Where the copies go on the remote machine, under its user's home directory. */
export const REMOTE_PLUGINS_DIRECTORY: string = call('REMOTE_PLUGINS_DIRECTORY');

/** What the session runs on: an SSH remote machine of either system, or a WSL distribution. */
export type RemoteTarget = 'posix' | 'windows' | 'wsl';

export interface RemotePluginFacts {
  /** `<name>@<marketplace>`. */
  readonly id: string;
  /** Every file and folder path inside the plugin, relative and `/`-separated. */
  readonly paths: readonly string[];
}

export interface RemotePluginPlan {
  /** The plugins to copy to `REMOTE_PLUGINS_DIRECTORY`, in the order given. */
  readonly copy: readonly string[];
  /** The plugins left out, for the session to list; a WSL session leaves none out: it gets none. */
  readonly leftOut: readonly string[];
}

export const planRemotePlugins = (
  target: RemoteTarget,
  plugins: readonly RemotePluginFacts[],
): RemotePluginPlan => call('planRemotePlugins', target, plugins);
