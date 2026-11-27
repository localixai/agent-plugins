import { coreModule } from '../wasm/bridge.ts';

/**
 * What counts as a registry package (plugins/loading.md, "Limits on the dependency install":
 * "every dependency must be a registry package ... A plugin with a git, GitHub, folder, workspace,
 * or linked dependency gets no install", "a download link in the lockfile must use `https`, unless
 * it points at the installing user's own default npm registry"; plugins/marketplace-reference.md,
 * "npm plugin source": a git address, a folder or `file:` path, an `npm:` alias and some tarball
 * links are refused). One classifier of a specifier serves the `package` value of an npm source and
 * the dependencies of a plugin's `package.json`; a registry of rules typed over the kinds says what
 * each kind means for each of the two (K1).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/node_deps_spec.rs`,
 * docs/RUST.md).
 */
const call = coreModule('marketplace/node-deps-spec');

export type SpecifierKind =
  | 'registry'
  | 'alias'
  | 'folder'
  | 'git-scheme'
  | 'git-address'
  | 'workspace'
  | 'linked'
  | 'tarball-host'
  | 'tarball-http'
  | 'tarball-https';

export const classifySpecifier = (value: string): SpecifierKind => call('classifySpecifier', value);

/**
 * The refusal of a download link: it "must use `https`, unless it points at the installing user's
 * own default npm registry". Any other scheme (`ftp:`, `file:`) and a link with none is refused too.
 */
export const downloadLinkRefusal = (
  link: string,
  defaultRegistry: string | undefined,
): string | undefined => call('downloadLinkRefusal', link, defaultRegistry);

/** Why a `package.json` dependency specifier gets no install; `undefined` for a registry package. */
export const dependencyRefusal = (specifier: string): string | undefined =>
  call('dependencyRefusal', specifier);

/** Whether an npm lockfile `resolved` value is a registry download (its scheme is checked as a download link). */
export const isRegistryOrigin = (resolved: string): boolean => call('isRegistryOrigin', resolved);

/** Whether the specifier of a `bun.lock` entry id (`name@spec`) is a registry package. */
export const isRegistryId = (specifier: string): boolean => call('isRegistryId', specifier);

// ---- package.json ----

export const DEPENDENCY_SECTIONS: readonly [
  'dependencies',
  'devDependencies',
  'optionalDependencies',
  'peerDependencies',
] = call('DEPENDENCY_SECTIONS');
export type DependencySection = (typeof DEPENDENCY_SECTIONS)[number];

export type DependencyMap = Readonly<Record<string, string>>;

/** The parts of a plugin's `package.json` the dependency install looks at. */
export interface PackageJson {
  readonly dependencies?: DependencyMap;
  readonly devDependencies?: DependencyMap;
  readonly optionalDependencies?: DependencyMap;
  readonly peerDependencies?: DependencyMap;
  /** npm `overrides`, present as written. */
  readonly overrides?: unknown;
  /** Bun `patchedDependencies`, present as written. */
  readonly patchedDependencies?: unknown;
}

/** A `package.json` that is there but is not a JSON object. */
export type ReadPackageJson = PackageJson | 'unreadable';

/** A parsed `package.json` value narrowed to the parts the install reads; `'unreadable'` for a value that is not an object. */
export const readPackageJson = (value: unknown): ReadPackageJson => call('readPackageJson', value);

export interface SpecifierRefusal {
  readonly section: DependencySection;
  readonly name: string;
  readonly specifier: string;
  readonly reason: string;
}

/** Every dependency of `package.json` that is not a registry package, with the reason. */
export const specifierRefusals = (packageJson: PackageJson): readonly SpecifierRefusal[] =>
  call('specifierRefusals', packageJson);
