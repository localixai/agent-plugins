/**
 * Which of the user's plugins a session on another machine gets (desktop.md, "Install plugins",
 * "Windows remote machines", "Files and processes on the remote machine"): a session over SSH gets
 * the plugins copied from the user's computer to `~/.claude/remote/plugins/` and loads them from
 * there; on a Windows remote machine a plugin that "holds a file or folder name Windows can't store,
 * such as a name with `:`" is not copied and "the session lists the plugins that Desktop left out";
 * "Plugins aren't available in WSL sessions". Pure over the paths the host listed: copying, the SSH
 * connection and the list the session shows are the host's.
 */

/** Where the copies go on the remote machine, under its user's home directory. */
export const REMOTE_PLUGINS_DIRECTORY = '~/.claude/remote/plugins/';

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

/**
 * Judgment (spec/judgments.md): a name Windows can't store is one with a character it refuses in a
 * file name (`< > : " \ | ? *` or a control character), one ending in a space or a dot, or a
 * reserved device name with or without an extension. The docs name only `:`.
 */
const REFUSED_CHARACTER = /[<>:"\\|?*]/;
/** Code points 0 to 31: all in the Basic Multilingual Plane, so one UTF-16 unit each. */
const hasControlCharacter = (segment: string): boolean => {
  for (let index = 0; index < segment.length; index += 1) {
    if (segment.charCodeAt(index) < 32) return true;
  }
  return false;
};
const RESERVED_NAME = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.[^.]*)?$/i;
const BAD_ENDING = /[ .]$/;

const storableOnWindows = (segment: string): boolean =>
  !REFUSED_CHARACTER.test(segment) &&
  !hasControlCharacter(segment) &&
  !RESERVED_NAME.test(segment) &&
  !BAD_ENDING.test(segment);

const pluginStorable = (plugin: RemotePluginFacts): boolean =>
  plugin.paths.every((path) => path.split('/').every(storableOnWindows));

/** What each target does with a plugin: copy it, leave it out, or give the session none at all. */
const TARGETS: {
  readonly [T in RemoteTarget]: (plugins: readonly RemotePluginFacts[]) => RemotePluginPlan;
} = {
  posix: (plugins) => ({ copy: plugins.map((plugin) => plugin.id), leftOut: [] }),
  windows: (plugins) => ({
    copy: plugins.filter(pluginStorable).map((plugin) => plugin.id),
    leftOut: plugins.filter((plugin) => !pluginStorable(plugin)).map((plugin) => plugin.id),
  }),
  wsl: () => ({ copy: [], leftOut: [] }),
};

export const planRemotePlugins = (
  target: RemoteTarget,
  plugins: readonly RemotePluginFacts[],
): RemotePluginPlan => TARGETS[target](plugins);
