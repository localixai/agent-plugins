import { coreModule } from '../wasm/bridge.ts';
import type { Invocable } from './invocation.ts';

/**
 * Which skill a typed or called name invokes. A skill answers to its listed command and to its other
 * names: a plugin skill's bare name (skills.md, "How a skill gets its command name"), a folder skill's
 * directory-qualified and alias names. One lookup serves the `/` prompt, the model's Skill call and a
 * subagent's `skills` list, so a name that runs a skill one way runs it every way.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/lookup.rs`, docs/RUST.md).
 */

const call = coreModule('skills/lookup');

/** A catalog row: the skill and every name that invokes it, the listed command first. */
export interface NamedInvocable {
  readonly invocable: Invocable;
  readonly names: readonly string[];
}

export interface SkillLookup<E extends NamedInvocable> {
  /** The row a name invokes, or undefined. */
  find(name: string): E | undefined;
  /** Every name that invokes a row. */
  readonly names: ReadonlySet<string>;
}

/**
 * The rows by name. A synced skill's command is claimed first; otherwise the earlier row wins a
 * command. Judgment (spec/judgments.md): a listed command wins over another row's alias of the same
 * text, and among aliases the earlier row wins. The core answers each name with its row's index; the
 * host's own row is what `find` hands back, so the index is mapped back here.
 */
export function skillLookup<E extends NamedInvocable>(rows: readonly E[]): SkillLookup<E> {
  const given = rows.map(({ invocable, names }) => ({
    command: invocable.command,
    kind: invocable.origin.kind,
    names,
  }));
  const claims: readonly (readonly [string, number])[] = call('skillLookup', given);
  const byName = new Map<string, E>();
  for (const [name, index] of claims) {
    const row = rows[index];
    if (row !== undefined) byName.set(name, row);
  }
  return { find: (name) => byName.get(name), names: new Set(byName.keys()) };
}
