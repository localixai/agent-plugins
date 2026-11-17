import type { PluginIssue } from '../format/issues.ts';
import type { CorePackage } from '../loading/core/index.ts';
import type { Plugin, ShellPaths } from '../loading/index.ts';
import type { FolderScan } from '../ports/folder-scan.ts';
import type { EnvFileStore, HookEngineOptions, HookPorts } from '../hooks/index.ts';
import type { ParentSettingsContext } from '../settings/managed/index.ts';
import type { PolicyView } from '../settings/index.ts';
import type { OptionValues } from '../settings/plugins/index.ts';
import type { WorkflowsContext } from '../settings/workflows/index.ts';
import type { ExpandSession, ShellArguments } from '../skills/index.ts';

/** A plugin the host was given: loaded once (`loadPlugin`), known by its `name@marketplace` id. */
export interface HostPlugin {
  /** The id the plugin is installed under, `name@marketplace`: the key of `enabledPlugins`. */
  readonly id: string;
  readonly plugin: Plugin;
  /**
   * Whether the host runs it. Absent, the `enabledPlugins` of the settings decide, and a plugin
   * nothing names is on. Managed settings that turn it off win over any value given here.
   */
  readonly enabled?: boolean;
  /** Where the plugin came from; see `SkillPluginSource`. Default `installed`. */
  readonly source?: NonNullable<ExpandSession['pluginSource']>;
}

/**
 * A core Agent Plugins package the host was given (`loadPluginPackage`, form `core`), known by its
 * `name@marketplace` id like any plugin (Agent Plugins Extended §18.1.5): `enabledPlugins`, managed
 * settings, name conflicts and dependencies treat it as one. Its skills, MCP servers and hook
 * handlers run by the core (`docs/API.md`, "Core packages in a session").
 */
export interface HostCorePackage {
  readonly id: string;
  readonly package: CorePackage;
  /** As `HostPlugin.enabled`. */
  readonly enabled?: boolean;
  /** As `HostPlugin.source`. */
  readonly source?: HostPlugin['source'];
}

/** A plugin the host knows and whether it runs. */
export interface HostedPlugin {
  readonly id: string;
  readonly name: string;
  readonly enabled: boolean;
  /** A core package (Agent Plugins Extended §18.1.5 item 6: a plugin list says so). */
  readonly core?: true;
}

/** What the host passes: the ports, with the ones its hook handler types need. */
export type HostPorts = HookPorts & { readonly folders?: FolderScan };

export interface PluginHostOptions {
  readonly plugins: readonly HostPlugin[];
  /** Core packages beside the plugins, loaded after them in the order given (J18.3). */
  readonly corePackages?: readonly HostCorePackage[];
  readonly ports: HostPorts;
  /** The settings after `createPolicyView`; absent, no settings file exists. */
  readonly settings?: PolicyView;
  /** How parent settings are treated; default: no administrator and no parent merge. */
  readonly parentSettings?: ParentSettingsContext;
  /**
   * The saved `userConfig` values by plugin id (the host's secure store); they win over `pluginConfigs`
   * of the settings. A bundled server's setting is saved under `<server>.<key>` (the `--config` form).
   * A session's own (`SessionOptions.userConfig`) replaces these for that session.
   */
  readonly userConfig?: Readonly<Record<string, OptionValues>>;
  /** A Windows host: paths are written with forward slashes where the docs say so. */
  readonly windows?: boolean;
  /** The shells a skill's `shell` field can pick; default `bash` only, plus PowerShell when `windows`. */
  readonly shells?: {
    readonly bash: boolean;
    readonly powershell: boolean;
  };
  /** The host process's id: `CLAUDE_PID` in every command hook (env-vars.md). Absent, the variable is not set. */
  readonly processId?: number;
  /**
   * The organisation's HIPAA configuration applies (hipaa-setup.md): the Anthropic credentials are
   * removed from the environment of every hook and subprocess the session starts.
   */
  readonly hipaa?: boolean;
  /** Host execution choices, forwarded unchanged to the hook engine; not plugin policy overrides. */
  readonly hookExecution?: Pick<
    HookEngineOptions,
    'defaultShell' | 'startDirectory' | 'tempDirectory'
  >;
  /**
   * How every string the session hands a POSIX shell carries `${CLAUDE_PLUGIN_ROOT}`,
   * `${CLAUDE_PLUGIN_DATA}` and `${CLAUDE_PROJECT_DIR}`: a monitor `command`, an MCP `headersHelper`,
   * a skill's `!` command. A bash shell-form hook always keeps the reference (Claude Code 2.1.322).
   * `environment-reference` (the default for a `headersHelper` and a `!` command, the explicit
   * opt-in for a monitor): the braced reference stays in the string and the value is in the process
   * environment the session gives with it, so a project or plugin folder named `x$(touch PWNED)` is
   * never parsed as shell. `pasted` (the default for a monitor, whose path variables are never
   * exported: Agent Plugins Extended §9.2.4, J9.17) writes the path itself, the docs' text, unsafe for
   * such a folder. Judgment (spec/judgments.md).
   */
  readonly shellPaths?: ShellPaths;
  /**
   * How a skill's or command's `!` command receives `$ARGUMENTS`, `$N` and `$name`. Default
   * `pasted`: the values are written into the command, as Claude Code does, so an argument holding
   * `; curl … | sh` is shell (docs/SECURITY.md, "`$ARGUMENTS` in `!` blocks").
   * `environment-reference`: a bash command gets each placeholder as a reference to a variable that
   * holds the value, and the permission check reads the command with each value quoted where it
   * lands; PowerShell stays pasted. Judgment (spec/judgments.md).
   */
  readonly shellArguments?: ShellArguments;
  /**
   * Where `CLAUDE_ENV_FILE` lives (hooks.md, "Persist environment variables"): the files a session's
   * hooks write and the shells of the host read. One store for every session: each call names the
   * session (`SessionOptions.sessionId`), so sessions never share a file. Absent, hooks get no
   * `CLAUDE_ENV_FILE`.
   */
  readonly envFiles?: EnvFileStore;
  /**
   * Folders beside `.claude` the host also reads skills, commands and agents from, in every base the
   * documented scopes read (`~`, the project chain, nested and added directories, the managed
   * directory; never claude.ai's synced folder): `['.agents']` reads `.agents/skills`,
   * `.agents/commands` and `.agents/agents` there. Within a scope every `.claude` folder wins a shared
   * name over an extra one, extra folders rank in the order given, and the scopes keep their
   * documented order; a skill folder below an extra one is never a plugin. Judgment
   * (spec/judgments.md): Claude Code reads `.claude` only. Absent, none.
   */
  readonly extraScopeFolders?: readonly string[];
  /**
   * An app or a self-hosted runner built this host's launch environment: a settings `env` value never
   * replaces a variable it sets (settings-reference.md, "How `env` values interact with your shell").
   */
  readonly launchOwnsEnvironment?: boolean;
  /**
   * What decides whether workflows are on besides the settings and `CLAUDE_CODE_DISABLE_WORKFLOWS`:
   * the signed-in plan (off by default on Pro) and the organization's switch (workflows.md, "Turn
   * workflows off"). Absent: not Pro, and the organization has not turned them off.
   */
  readonly workflows?: Omit<WorkflowsContext, 'env'>;
}

/**
 * What `host.update` replaces: each part given (and not `undefined`) replaces the host's own whole;
 * a part left out stays. The ports, `windows` and `shells` are the host's for good. To clear the
 * settings give `createPolicyView([])`; `userConfig` replaces the saved values of every plugin.
 */
export type HostUpdate = {
  readonly [K in 'plugins' | 'corePackages' | 'settings' | 'parentSettings' | 'userConfig']?:
    | PluginHostOptions[K]
    | undefined;
};

/** A resolved list and what was found wrong while resolving it. */
export interface Resolved<T> {
  readonly resolved: readonly T[];
  readonly issues: readonly PluginIssue[];
}
