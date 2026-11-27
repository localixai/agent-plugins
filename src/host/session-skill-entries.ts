import type { FolderSource, PluginSource } from '../format/component-source.ts';
import type { BodyReader, InvocableHead } from '../format/skill-types.ts';
import { type ScanScope } from '../skills/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ActivePlugin, HostState } from './state.ts';

/**
 * The session's skills and commands without their bodies, with every name that invokes them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_skill_entries.rs`,
 * docs/RUST.md). Each plugin entry names its plugin by its index into `state.active`, mapped back
 * here to the host's `ActivePlugin`; the memo by the host's state object stays here until the state
 * is the core's.
 */

const call = coreModule('host/session-skill-entries');

/** The host gave the skill itself (`SessionOptions.hostSkills`): a bundled skill of the host. */
export interface HostSource {
  readonly kind: 'host';
}

/**
 * Where a skill came from: a plugin, a folder scope (skills.md, "Choose where skills load"), or the
 * host; the plugin and folder shapes are the ones agent rows share (`format/component-source`).
 */
export type SkillSource = PluginSource | FolderSource<ScanScope> | HostSource;

/** Where a skill lives, as the host writes paths: absolute, each absent when the skill has none. */
export interface SkillPlace {
  /** The `SKILL.md` or the command file. An inline manifest command has none. */
  readonly file?: string;
  /** The skill's folder, the value of `${CLAUDE_SKILL_DIR}`. A command has none. */
  readonly directory?: string;
}

/** One skill or command of the session, without its body, with every name that invokes it. */
export interface Entry extends SkillPlace {
  /** The skill or command without its body: the session reads the body when it runs (`readBody`). */
  readonly invocable: InvocableHead;
  /** Every name that invokes it, the listed command first. */
  readonly names: readonly string[];
  readonly readBody: BodyReader;
  readonly source: SkillSource;
  /** The plugin it belongs to, for a plugin's skill. */
  readonly plugin?: ActivePlugin;
}

/** A plugin entry as the core answers it: its plugin by its index into `state.active`. */
interface PluginEntry extends Omit<Entry, 'plugin'> {
  readonly plugin: number;
}

const ENTRIES = new WeakMap<HostState, readonly Entry[]>();

/**
 * The plugins' skills and commands of one host generation, built once for it: the plugin root of
 * each is asked of the state's `Locations` here, so a catalog read after it calls no port.
 */
export function entriesOf(state: HostState): readonly Entry[] {
  const cached = ENTRIES.get(state);
  if (cached !== undefined) return cached;
  const given = state.active.map(({ id, catalog }) => ({
    id,
    skills: catalog.skills,
    commands: catalog.commands,
  }));
  const built: readonly PluginEntry[] = call('entriesOf', given, state.options.ports.locations);
  const entries = built.flatMap(({ plugin, ...entry }): Entry[] => {
    const active = state.active[plugin];
    return active === undefined ? [] : [{ ...entry, plugin: active }];
  });
  ENTRIES.set(state, entries);
  return entries;
}
