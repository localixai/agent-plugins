import type { HooksSettings } from '../protocol/hook-config.ts';
import type { AgentDefinition } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { AgentExtras } from './agent-record.ts';
import type { InvocableFrontmatter } from './skill-types.ts';

/**
 * A skill, command or agent file from the kit's own definition types, the inverse of `readSkill`,
 * `readCommand` and `readAgent`: each documented key in its table's order (skills.md and
 * sub-agents.md, "Frontmatter reference"), written by `writeFrontmatter`, then the host's own keys
 * (`extra`) after them. A skill or command value equal to its documented default is left out, as an
 * author leaves it out; `shell` is the exception, since an explicit `shell: bash` "fails the
 * invocation" when bash is missing (skills.md, "How injected commands run").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/frontmatter_render.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/frontmatter-render');

/** What an author sets of the shared fields: the read-only views (`frontmatter`, `undescribed`) are the reader's. */
export type InvocableDraft = Partial<
  Omit<InvocableFrontmatter, 'frontmatter' | 'undescribed' | 'shellDeclared'>
>;

export interface SkillDraft extends InvocableDraft {
  readonly name?: string;
  /** The Markdown after the frontmatter, written as given. */
  readonly body: string;
  /** Keys the kit does not model, written after the documented ones. */
  readonly extra?: Readonly<Record<string, unknown>>;
}

/** A command file takes neither `name` nor `paths` (the reader leaves both untaken). */
export interface CommandDraft extends Omit<InvocableDraft, 'paths'> {
  readonly body: string;
  readonly extra?: Readonly<Record<string, unknown>>;
}

export interface AgentDraft {
  readonly name: string;
  /** The official shape; `prompt` is the body. */
  readonly definition: AgentDefinition;
  readonly extras?: AgentExtras;
  readonly hooks?: HooksSettings;
  readonly extra?: Readonly<Record<string, unknown>>;
}

export const renderSkillFile = (draft: SkillDraft): string => call('renderSkillFile', draft);

/** A command file is a skill file without `name` and `paths`: the one writer. */
export const renderCommandFile = (draft: CommandDraft): string => call('renderCommandFile', draft);

export const renderAgentFile = (draft: AgentDraft): string => call('renderAgentFile', draft);
