/**
 * What `skillOverrides` does to one skill (skills.md, "Override skill visibility from settings").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/skills/overrides.rs`,
 * docs/RUST.md).
 */
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/skills/overrides');

/**
 * What a state does to one skill (skills.md, "Override skill visibility from settings": the table
 * of "Listed to Claude" and "In `/` menu"). `inClientCommandLists` is the command list advertised
 * to Remote Control clients and Agent SDK callers; `invokableByFullName` is false when invoking the
 * skill by its full name returns the `skillOverrides` error (`blockedBy`) instead of running it.
 */
export interface SkillVisibility {
  readonly listedToModel: 'full' | 'nameOnly' | 'hidden';
  readonly inSlashMenu: boolean;
  readonly inClientCommandLists: boolean;
  readonly invokableByFullName: boolean;
  readonly blockedBy: 'skillOverrides' | undefined;
}

/** What the host knows about the skill it asks for. */
export interface SkillSubject {
  /** A skill a plugin provides: "Overrides don't apply to plugin skills". */
  readonly plugin?: boolean;
  /** The other names of a bundled skill, such as `checkup` for `/doctor`. */
  readonly aliases?: readonly string[];
}

/** What `skillOverrides` does to one skill; a plugin skill and a skill with no entry are `on`. */
export const skillVisibility = (
  view: PolicyView,
  name: string,
  subject: SkillSubject = {},
): SkillVisibility =>
  call('skillVisibility', view, name, { plugin: subject.plugin, aliases: subject.aliases });
