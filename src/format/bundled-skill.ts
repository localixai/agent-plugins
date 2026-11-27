import type { PluginIssue } from './issues.ts';
import type { InvocableHead } from './skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * A skill the host supplies itself: Claude Code's bundled skills ("Bundled skills": "You invoke a
 * bundled skill the same way as any other skill", skills.md). It arrives as data, not a file the kit
 * reads, so its head is read here from the name and the frontmatter the host gives.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/bundled_skill.rs`, docs/RUST.md).
 */
const call = coreModule('format/bundled-skill');

export interface BundledSkillHead {
  /** The command, as typed without the slash: `review`, `code-review`. */
  readonly name: string;
  /** The frontmatter as a skill's YAML would hold it (`description`, `allowed-tools`, any key). */
  readonly frontmatter?: Readonly<Record<string, unknown>>;
  /** The skill's `SKILL.md`, absolute, when it has one: it is then a skill with a folder. */
  readonly file?: string;
}

export interface BundledSkillRead {
  /** Absent when the name is refused: an error issue says why. */
  readonly invocable?: InvocableHead;
  readonly issues: readonly PluginIssue[];
}

/**
 * Judgment (spec/judgments.md): the name is the command as given, refused as a command name is
 * (`commandNameIssues`: an empty name, a character a command name can't hold, `anthropic-skills`),
 * and the frontmatter is read as a skill's, `name` aside (the host's `name` is the command). With a
 * `file` it is a skill whose folder is the file's directory; without one, a command without a
 * folder. `body` is the text when the host gave text, for the `description` fallback. The head
 * crosses as its three fields: a host's skill also holds its body reader.
 */
export const readBundledSkill = (head: BundledSkillHead, body: string): BundledSkillRead =>
  call(
    'readBundledSkill',
    { name: head.name, frontmatter: head.frontmatter, file: head.file },
    body,
  );
