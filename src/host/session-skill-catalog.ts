import type { PluginIssue } from '../format/issues.ts';
import type {
  InvocationPolicy,
  ModelListing,
  NamedInvocable,
  OverrideState,
  SkillOverrides,
} from '../skills/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { Entry, SkillPlace, SkillSource } from './session-skill-entries.ts';

/**
 * The session's skills and commands as rows: what the kit itself lists (`skills()`), expands
 * (`expandSkill`, `expandMessage`) and preloads, so a host's own list, roster and prompt listing are
 * the kit's. Pure over the entries the session already holds: no port is called.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_skill_catalog.rs`,
 * docs/RUST.md). An entry crosses as its data: its `readBody` and its plugin stay here.
 */

const call = coreModule('host/session-skill-catalog');

/**
 * One skill or command the session can invoke. `file` and `directory` say where it lives, absolute
 * for every source (`SkillPlace`): `directory` is the value of `${CLAUDE_SKILL_DIR}`, placed once
 * when the session built its entries, so reading the catalog calls no port.
 */
export interface SessionSkill extends NamedInvocable, SkillPlace {
  /**
   * Every name that invokes this row through `expandSkill`, `expandMessage` and a subagent's `skills`,
   * the listed command first. A name another row holds (`skillLookup`: a listed command wins over an
   * alias) or a workflow holds is left out.
   */
  readonly names: readonly string[];
  readonly source: SkillSource;
  /** The `skillOverrides` state in force; always `on` for a plugin skill, which the setting leaves alone. */
  readonly override: OverrideState;
  /** Who may invoke it and whether the `/` menu shows it, with the override applied. */
  readonly policy: InvocationPolicy;
  /** What the model's listing says of it before the budget (`listedAs`). */
  readonly listed: ModelListing;
  /** It sets `paths` and no file touched in this session activated it: listed by name alone. */
  readonly dormant: boolean;
}

/** Every row, and what keeps the list from being whole or a host skill from loading. */
export interface SkillCatalog {
  readonly skills: readonly SessionSkill[];
  /**
   * A warning while the folders are still to read (`reloadSkills`: the rule `reloadAgents` shares),
   * and the issues of `SessionOptions.hostSkills` (a refused name, a second skill of one name).
   */
  readonly issues: readonly PluginIssue[];
}

export interface SkillCatalogOptions {
  /** Commands the host knows a `paths` match activated, beyond the files `touchFile` was told of. */
  readonly activated?: ReadonlySet<string>;
}

export interface CatalogInput {
  readonly entries: readonly Entry[];
  /** The names the plugins' workflows hold: no skill row answers to them. */
  readonly workflows: ReadonlySet<string>;
  readonly overrides: SkillOverrides | undefined;
  readonly activated: ReadonlySet<string>;
}

/** The rows, in listing order: plugins first, then the folders, then the host's skills. */
export const catalogRows = (input: CatalogInput): readonly SessionSkill[] =>
  call('catalogRows', {
    entries: input.entries.map(({ invocable, names, source, file, directory }) => ({
      invocable,
      names,
      source,
      file,
      directory,
    })),
    workflows: input.workflows,
    overrides: input.overrides,
    activated: input.activated,
  });
