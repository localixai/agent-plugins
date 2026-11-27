import type { SkillOrigin } from '../format/skill-types.ts';
import type { Locations } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The files a text names with `@`. Claude Code attaches them for a local skill's or command's body:
 * a synced skill's body "doesn't attach the files that `@` references name the way it does for a
 * local skill" (skills.md, "How Claude Code handles the body of a synced skill"; changelog.md,
 * "Custom slash commands: ... @-mention files", "Added support for @-mentions in slash command
 * arguments"). The kit finds and resolves them; reading and attaching a file is the host's, as for
 * an `@` the user types (common-workflows.md, "Reference files and directories": the Read tool's
 * limits apply, and a directory or an MCP resource is the host's to tell apart).
 *
 * The docs give no syntax for a skill's references. Oracle 2.1.322 (spec/judgments.md): a reference
 * is `@` at the start of the text or after whitespace, then a path up to the next whitespace, or a
 * path in double quotes, which may hold spaces; a backslash does not escape a space; Markdown code
 * spans and fences are not skipped; trailing `` .,;:!?)"` `` and a `#<line range>` are not part of the
 * path; a relative path resolves against the working directory (not the skill's folder), `~/` against
 * the home folder; the same reference twice is attached once.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/file_references.rs`, docs/RUST.md).
 */
const call = coreModule('skills/file-references');

export interface FileReference {
  /** The reference as written, `@` included: `@src/a.ts`, `@"Design Docs/api.md"`, `@notes.md#2-3`. */
  readonly text: string;
  /** The file it names, resolved (`Locations.joinPath`): the host checks it exists and reads it. */
  readonly path: string;
}

export interface ReferenceBase {
  /** The working directory a relative path resolves against. */
  readonly cwd: string;
  readonly locations: Pick<Locations, 'homeDir' | 'joinPath'>;
}

/** The references of `text`, in order, each once. */
export const readFileReferences = (text: string, base: ReferenceBase): FileReference[] =>
  call('readFileReferences', text, base);

/** Whether an expansion of a skill from `origin` attaches its references; `isolated`: a cloud or Cowork session. */
export const attachesFileReferences = (origin: SkillOrigin, isolated: boolean): boolean =>
  call('attachesFileReferences', origin, isolated);
