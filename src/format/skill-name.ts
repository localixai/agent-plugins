import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { SkillLayout } from './skill-types.ts';

/**
 * How a skill gets its command name (skills.md, "How a skill gets its command name" and
 * "Characters a skill name can't hold"; plugins/components.md, "Skills").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/skill_name.rs`, docs/RUST.md).
 */
const call = coreModule('format/skill-name');

export interface SkillName {
  /** The name the skill goes by: the frontmatter `name`, else the folder's name. */
  readonly name: string;
  /** What the user types after `/`. */
  readonly command: string;
  /**
   * `path` is the field the issue is about: `name` (the frontmatter) or `folder`. A refused name's
   * message is its debug-log line, `[names] <name> is not offered` (the skill does not load) or
   * `[names] <value> is not shown for <skill>` (it loads without that `name`), hidden characters
   * spelled `\u{200B}`; a host writes it to the debug log and counts the lines for the startup
   * notice (`skillNamesRefused`). Any other message is a whole sentence that starts with the field
   * (`folder name uses ...`). An error means the skill does not load; a warning means it loads.
   */
  readonly issues: readonly PluginIssue[];
}

/**
 * The command name of a skill named by itself (the plugin root, or a manifest path to its folder),
 * oracle 2.1.319 (spec/judgments.md): the plugin's own `<plugin>:` prefix is dropped, then each
 * UTF-16 unit outside `[A-Za-z0-9_-]` is a hyphen, so `Plan Review` is `Plan-Review`, `other:x` is
 * `other-x` and an emoji is two. `claude plugin details` prints the name as written.
 */
export const selfNamedCommand = (name: string, plugin: string): string =>
  call('selfNamedCommand', name, plugin);

/**
 * Resolve the name and the command of a skill from where it sits and its frontmatter `name`
 * (`declared`, as read; undefined when absent). A `name` that already starts with the prefix is
 * not prefixed again (v2.1.246 and later). Never throws: a refusal is an error issue, and `name`
 * and `command` are then the text as written, for the host to report.
 */
export const resolveSkillName = (layout: SkillLayout, declared: string | undefined): SkillName =>
  call('resolveSkillName', layout, declared);
