import type { CatalogSource } from '../agents/catalog/index.ts';
import type { Command, Skill } from '../format/skill-types.ts';
import type { AgentDefinition } from '../protocol/official.ts';
import type { PreloadPlan } from '../agents/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { Entry } from './session-skill-entries.ts';
import type { HostState } from './state.ts';

/**
 * The skills a subagent preloads.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_skill_preload.rs`,
 * docs/RUST.md). An entry crosses as its data and its reader, its plugin as its id and name.
 */

const call = coreModule('host/session-skill-preload');

export interface PreloadedSkills {
  /** The listed skills with their full content, in the list's order. */
  readonly preloaded: readonly (Skill | Command)[];
  readonly skipped: PreloadPlan['skipped'];
}

/** What `session.preloadSkills` takes besides the definition. */
export interface PreloadOptions {
  readonly signal: AbortSignal;
  /**
   * The agent's `CatalogAgent.source`: a plugin agent's bare name is looked for in its own plugin
   * before the other plugins. Without it the agent is read as one of no plugin.
   */
  readonly source?: CatalogSource | undefined;
}

/** The plan over the session's catalog, then each planned skill's body as its file holds it now. */
export function preloadFrom(
  entries: readonly Entry[],
  state: HostState,
  request: PreloadOptions & { readonly definition: Pick<AgentDefinition, 'skills'> },
): Promise<PreloadedSkills> {
  const given = entries.map(({ invocable, names, readBody, file, directory, plugin }) => ({
    invocable,
    names,
    readBody,
    file,
    directory,
    plugin: plugin === undefined ? undefined : { id: plugin.id, name: plugin.plugin.name },
  }));
  return call.async(
    'preloadFrom',
    given,
    {
      overrides: state.view.get('skillOverrides'),
      source: request.source,
      skills: request.definition.skills,
    },
    request.signal,
  );
}
