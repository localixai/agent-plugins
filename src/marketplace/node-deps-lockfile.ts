/**
 * The facts of a lockfile the dependency install checks (plugins/loading.md, "Limits on the
 * dependency install": a registry package "pinned in the lockfile to an exact version", an `https`
 * download link, the same dependencies in `package.json` and the lockfile; "When the dependency
 * install runs": `lockfileVersion`). The docs name only `lockfileVersion`; the rest of each layout
 * is npm's and Bun's own and is read as little as the checks need (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/node_deps_lockfile.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { DependencyMap, DependencySection } from './node-deps-spec.ts';

const call = coreModule('marketplace/node-deps-lockfile');

/** The lockfiles the install reads, in the order Claude Code checks them: "the first match". */
export const LOCKFILE_NAMES: readonly ['bun.lock', 'npm-shrinkwrap.json', 'package-lock.json'] =
  call('LOCKFILE_NAMES');
export type LockfileName = (typeof LOCKFILE_NAMES)[number];

/** A package the lockfile pins to something other than a registry version. */
export interface LockedOrigin {
  readonly name: string;
  /** What it is pinned to, in the specifier forms of `package.json` (`link:../a`, `git+ssh://...`). */
  readonly origin: string;
}

export type RootDependencies = { readonly [S in DependencySection]?: DependencyMap };

export interface LockfileFacts {
  /** `lockfileVersion`; `undefined` when the file is not readable or has none. */
  readonly version: number | undefined;
  /** What the lockfile records the root package as depending on. */
  readonly root: RootDependencies;
  /** Packages pinned to a non-registry origin. */
  readonly locked: readonly LockedOrigin[];
  /** Every download link the lockfile holds, whatever its scheme. */
  readonly downloads: readonly string[];
  /** Registry packages not pinned to an exact version, each as `name@version` (the name alone when none is given). */
  readonly unpinned: readonly string[];
}

/** The facts of a lockfile's text; text that is not JSON has no version, so no install runs from it. */
export const readLockfile = (name: LockfileName, text: string): LockfileFacts =>
  call('readLockfile', name, text);
