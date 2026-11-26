import type { ComponentKind, ComponentPlace } from '../../format/validate/component-frontmatter.ts';
import type { ValidationIssue } from '../../format/validate/finding.ts';
import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { FolderStyle } from './links.ts';

/**
 * The Markdown components `claude plugin validate` reads (oracle 2.1.319): each skill folder's
 * `SKILL.md`, and every `.md` below `agents/` and `commands/` at any depth. Paths the manifest
 * declares are not read for frontmatter. A link is never followed: the folder or entry is skipped,
 * and a skills folder says so.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/components.rs`,
 * docs/RUST.md).
 */
const call = coreModule('loading/validate/components');

/** The strings of a manifest key: one path, or the paths of a list. */
export const manifestPaths = (value: unknown): string[] => call('manifestPaths', value);

/** What a scan of one folder found: the files to read and what it has to say about links. */
export interface Scan {
  readonly files: readonly string[];
  readonly issues: readonly ValidationIssue[];
}

/** The `SKILL.md` of each skill folder directly below `dir`, and the links among them. */
export const scanSkillFolders = (
  files: PluginFiles,
  dir: string,
  style: FolderStyle,
  signal?: AbortSignal,
): Promise<Scan> => call.async('scanSkillFolders', files, dir, style, signal);

/** The Markdown files below a folder at any depth, links left alone; `style` decides whether they are counted. */
export const scanMarkdownFolder = (
  files: PluginFiles,
  dir: string,
  style: FolderStyle,
  signal?: AbortSignal,
): Promise<Scan> => call.async('scanMarkdownFolder', files, dir, style, signal);

/** The frontmatter findings of the files of a scan. */
export const frontmatterIssues = (
  kind: ComponentKind,
  files: PluginFiles,
  paths: readonly string[],
  options: {
    readonly signal?: AbortSignal | undefined;
    readonly place?: ComponentPlace;
  } = {},
): Promise<ValidationIssue[]> => call.async('frontmatterIssues', kind, files, paths, options);

/** Link warnings and frontmatter findings of every skill, agent and command file, kind by kind. */
export const componentIssues = (
  files: PluginFiles,
  signal?: AbortSignal,
): Promise<ValidationIssue[]> => call.async('componentIssues', files, signal);
