import type { RegisterTier } from '../types.ts';
import { fullProvides, type PluginProvides, type ProvidesKind } from './provides.ts';
import type { ModUses } from './scan/uses.ts';

/**
 * The event of `plugin.register` (Agent Plugins Extended §10.19.5; `PluginRegisterInput`,
 * claude-code/index.d.ts:9937 of 2.1.330), built from what the loader read and what the host knows
 * of the plugin's other parts.
 */

/** `e.uses.hosts`: the hosts the module's `$.http.fetch` and `$.audio.play({ url })` calls spell, and which of the two it calls with a URL the scan cannot read. */
export interface RegisterUsesHosts {
  readonly named: readonly string[];
  readonly unread: readonly string[];
}

/** `e.uses`: what `claude plugin validate` lists, each call without `$.`; empty for a plugin with no module. */
export interface RegisterUses {
  readonly events: readonly string[];
  readonly calls: readonly string[];
  readonly env?: { readonly reads: readonly string[]; readonly writes: readonly string[] };
  readonly state?: { readonly reads: readonly string[]; readonly writes: readonly string[] };
  readonly hosts?: RegisterUsesHosts;
}

/** What the host knows of a plugin beyond its hooks module (`ModPlugin.register`). */
export interface PluginRegisterFacts {
  /** The plugin's directory; the kit's default is the plugin root its locations port gives. */
  readonly root?: string;
  /** Where the plugin came from, as the host names it; the kit's default is `installed`, or `inline` for `--plugin-dir` (judgment). */
  readonly provenance?: string;
  /** Handed over by the app or claude.ai as required by the organization (§10.19.5 item 5); it grants nothing. */
  readonly requiredByOrganization?: boolean;
  /** The parts the plugin ships other than its hooks module, by kind; `modules` is the loader's. */
  readonly provides?: Partial<PluginProvides>;
  /** The kinds whose list may be short because something could not be read (§10.19.5 item 3). */
  readonly unread?: readonly ProvidesKind[];
  /** The whole plugin's `PluginHosts` (claude-code/index.d.ts:9715), passed through unchanged. */
  readonly hosts?: Readonly<Record<string, unknown>>;
}

export type PluginRegisterInput = {
  readonly name: string;
  readonly root: string;
  readonly version?: string;
  readonly tier: RegisterTier;
  readonly provenance: string;
  readonly requiredByOrganization?: true;
  readonly uses: RegisterUses;
  readonly provides: PluginProvides;
  readonly unread: readonly ProvidesKind[];
  readonly hosts: Readonly<Record<string, unknown>>;
};

const pair = (reads: readonly string[], writes: readonly string[]) =>
  reads.length + writes.length > 0 ? { reads, writes } : undefined;

/** `e.uses` from what the scan found: `env`, `state` and `hosts` only when there is something in them. */
export function registerUses(uses: ModUses | undefined): RegisterUses {
  if (uses === undefined) return { events: [], calls: [] };
  const env = pair(uses.envReads, uses.envWrites);
  const state = pair(uses.stateReads, uses.stateWrites);
  const hosts = pair(uses.hostsNamed, uses.hostsUnread);
  return {
    events: uses.events,
    calls: uses.calls,
    ...(env === undefined ? {} : { env }),
    ...(state === undefined ? {} : { state }),
    ...(hosts === undefined ? {} : { hosts: { named: hosts.reads, unread: hosts.writes } }),
  };
}

/** What the loader adds to the host's facts: the mod, its tier, its version and its hooks module. */
export interface RegisterSubject {
  readonly name: string;
  readonly id: string;
  readonly tier: RegisterTier;
  readonly root: string;
  readonly version?: string;
  /** The hooks module's path as `hooks.json` names it, when it has one. */
  readonly module?: string;
  readonly uses?: ModUses;
}

/** The `plugin.register` event of `subject`, over the host's `facts`. */
export function registerEvent(
  subject: RegisterSubject,
  facts: PluginRegisterFacts = {},
): PluginRegisterInput {
  const modules = subject.module === undefined ? [] : [{ path: subject.module }];
  const isInline = subject.id.endsWith('@inline');
  return {
    name: subject.name,
    root: facts.root ?? subject.root,
    ...(subject.version === undefined ? {} : { version: subject.version }),
    tier: subject.tier,
    provenance: facts.provenance ?? (isInline ? 'inline' : 'installed'),
    ...(facts.requiredByOrganization === true ? { requiredByOrganization: true } : {}),
    uses: registerUses(subject.uses),
    provides: fullProvides({ ...facts.provides, modules }),
    unread: (facts.unread ?? []).filter((kind) => kind !== 'modules'),
    hosts: facts.hosts ?? { named: [], unread: [] },
  };
}
