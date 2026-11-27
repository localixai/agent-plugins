/**
 * Whether a folder is a plugin (plugins/loading.md, "Find where a plugin came from"; plugins/
 * manifest-reference.md, "Standard layout"; plugins/create.md, "Plugin layout" and "Name the plugin's
 * skills"; skills.md, "Skill folder as a plugin"): the probe a host runs over a folder of a skills
 * directory (`@skills-dir`) or one handed to `--plugin-dir` (`@inline`). It reads the folder through
 * `PluginFiles` and decides; loading the plugin stays `loadPlugin`'s.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/folder_plugin.rs`, docs/RUST.md).
 */
import type { ComponentKind } from '../format/layout.ts';
import type { PluginIssue } from '../format/issues.ts';
import type { PluginFiles } from '../ports/files.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { Plugin } from './plugin-model.ts';

const call = coreModule('loading/folder-plugin');

/** `skills-dir`: a folder of `~/.claude/skills/` or `.claude/skills/`; `inline`: a `--plugin-dir` directory. */
export const FOLDER_ORIGINS: readonly ['skills-dir', 'inline'] = call('FOLDER_ORIGINS');
export type FolderOrigin = (typeof FOLDER_ORIGINS)[number];

export interface FolderPlugin {
  /** The manifest's `name`, else the folder's name. */
  readonly name: string;
  /** `<name>@skills-dir` or `<name>@inline`. */
  readonly id: string;
  /** The manifest's `defaultEnabled`, `true` when it sets none. */
  readonly defaultEnabled: boolean;
  /** A `SKILL.md` sits at the plugin root. */
  readonly rootSkill: boolean;
  /** The component kinds the folder shows at its root, in layout order. */
  readonly components: readonly ComponentKind[];
}

export interface FolderProbe {
  readonly plugin: FolderPlugin | undefined;
  /** A skills-directory folder with a root `SKILL.md` and no manifest: a plain skill named after the folder. */
  readonly plainSkill: { readonly name: string } | undefined;
  /** Why a manifest or a name keeps the folder from being a plugin. */
  readonly issues: readonly PluginIssue[];
}

/**
 * Probes one folder. With a manifest the folder is a plugin named by it. Without one, a `--plugin-dir`
 * folder with any component (or a root `SKILL.md`) is a plugin named after the folder, and a
 * skills-directory folder is a plain skill when it has a root `SKILL.md`, otherwise nothing.
 */
export const probeFolder = (
  files: PluginFiles,
  folderName: string,
  origin: FolderOrigin,
  options: { readonly signal?: AbortSignal | undefined } = {},
): Promise<FolderProbe> => call.async('probeFolder', files, folderName, origin, options);

/**
 * Whether a skills-directory plugin is on: "The manifest's `defaultEnabled`, unless a settings file
 * sets `<name>@skills-dir` to `true` or `false`."
 */
export const skillsDirPluginEnabled = (
  defaultEnabled: boolean,
  setting: boolean | undefined,
): boolean => call('skillsDirPluginEnabled', defaultEnabled, setting);

export interface SkillsDirFolder {
  /** The folder's name under the skills directory. */
  readonly name: string;
  readonly scope: 'user' | 'project';
  /** The text of the `SKILL.md` at the folder's root, when there is one. */
  readonly rootSkill?: string | undefined;
}

/**
 * The commands of a skills-directory plugin (plugins/create.md, "Name the plugin's skills"): "The root
 * skill ... is also a personal skill, so you invoke it as `/my-tool`, not `/my-tool:my-tool`. Skills you
 * add under `skills/` inside the plugin get the plugin-name prefix, such as `/my-tool:example`." The
 * root skill is the folder's, not the plugin's: it is a skill of the skills directory whether or not
 * the plugin loads it too, so the plugin's own copy of it is left out.
 */
export const skillsDirCommands = (plugin: Plugin, folder: SkillsDirFolder): readonly string[] =>
  call('skillsDirCommands', { name: plugin.name, skills: plugin.skills }, folder);
