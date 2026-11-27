/**
 * How each directory-shaped component turns one existing location (a file or a folder, default or
 * declared) into the files it holds (plugins/manifest-reference.md, "Standard layout" and "Path-only
 * fields"; plugins/components.md for each kind). One strategy per kind, typed over the union.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/walk.rs`, docs/RUST.md): the
 * core's `walk` is every strategy, by kind; the table keeps its shape here.
 */
import type { FileStat } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { DiscoverContext, DiscoveredFile, Found } from './discovered.ts';

const call = coreModule('loading/walk');

export const LISTED_KINDS: readonly [
  'skills',
  'commands',
  'agents',
  'outputStyles',
  'workflows',
  'themes',
] = call('LISTED_KINDS');
export type ListedKind = (typeof LISTED_KINDS)[number];

/** `declared`: the location comes from a manifest path rather than from the standard layout. */
type Walker = (
  context: DiscoverContext,
  path: string,
  stat: FileStat,
  declared: boolean,
) => Promise<Found<DiscoveredFile>>;

const walker =
  (kind: ListedKind): Walker =>
  (context, path, stat, declared) =>
    call.async('walk', kind, context, path, stat, declared);

export const WALKERS: { readonly [K in ListedKind]: Walker } = {
  skills: walker('skills'),
  commands: walker('commands'),
  agents: walker('agents'),
  outputStyles: walker('outputStyles'),
  workflows: walker('workflows'),
  themes: walker('themes'),
};
