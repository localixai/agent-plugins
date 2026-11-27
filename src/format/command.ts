import type { PluginIssue } from './issues.ts';
import type { CommandEntry } from './manifest-schema.ts';
import type { Command, SkillOrigin } from './skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Commands, "the older format" (plugins/components.md, "Commands"; skills.md, "Where skills
 * live"): a flat Markdown file that "takes the same frontmatter as skills" except `name` and
 * `paths`, or an entry of the manifest's `commands` object map (plugins/manifest-reference.md,
 * "commands"). Never throws; a refused command is an error issue and no command.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/command.rs`, docs/RUST.md).
 */
const call = coreModule('format/command');

export interface CommandRead {
  /** Absent when the command is refused: an error issue says why. */
  readonly command?: Command;
  readonly issues: readonly PluginIssue[];
}

export interface CommandLayout {
  readonly origin: SkillOrigin;
  /** The file's path below its `commands` folder, `/`-separated, with its extension: `db/migrate.md`. */
  readonly relativePath: string;
}

export interface CommandEntryContext {
  readonly origin: SkillOrigin;
}

/**
 * Read a command file. The name is its path below `commands/` without the extension, the folders
 * joined by `:` (`db/migrate.md` is `db:migrate`), after the plugin prefix for a plugin's file.
 */
export const readCommand = (path: string, text: string, layout: CommandLayout): CommandRead =>
  call('readCommand', path, text, layout);

/**
 * Build a `Command` from one entry of the manifest's `commands` map. `key` is the command name
 * after the plugin prefix. `sourceText` is the text of the entry's `source` file, `undefined` when
 * the file is not there. Exactly one of `source` and `content` must be set (the manifest says an
 * entry with both or neither "fails validation"). Judgment (spec/judgments.md): `content` is the
 * body as written, not parsed for frontmatter ("Inline Markdown for the command body").
 */
export const readCommandEntry = (
  key: string,
  entry: CommandEntry,
  sourceText: string | undefined,
  ctx: CommandEntryContext,
): CommandRead => call('readCommandEntry', key, entry, sourceText, ctx);
