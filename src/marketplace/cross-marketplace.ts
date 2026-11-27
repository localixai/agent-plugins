import type { Marketplace } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { NormalizedDependency } from './types.ts';

/**
 * Cross-marketplace dependencies (plugins/dependencies.md, "Depend on a plugin from another
 * marketplace"; marketplace-reference.md, "Top-level fields"): a dependency in a marketplace other
 * than the declaring plugin's installs only when the root marketplace, the one that hosts the plugin
 * the user is installing, lists it in `allowCrossMarketplaceDependenciesOn` (its list alone applies,
 * for the whole dependency chain), or when the user already has the dependency enabled at the same
 * scope. The kit decides; the host installs or refuses.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/cross_marketplace.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/cross-marketplace');

/** Where the dependency is declared: it decides what a refusal does. */
export type DeclaredIn = 'entry' | 'plugin.json';

/** What a refusal does: a marketplace entry's refuses the install; `plugin.json`'s installs without the dependency, and the plugin then fails to load. */
export type RefusalEffect = 'refuse-install' | 'install-without-dependency';

export type CrossMarketplaceDecision =
  | { readonly allowed: true }
  | { readonly allowed: false; readonly message: string; readonly effect: RefusalEffect };

export interface CrossMarketplaceQuestion {
  readonly dep: NormalizedDependency;
  /** The plugin that declares the dependency; `marketplace` is absent for one that has none (loaded from claude.ai). */
  readonly requiredBy: { readonly name: string; readonly marketplace?: string };
  /** The marketplace that hosts the plugin the user is installing: its list alone decides. */
  readonly root: Pick<Marketplace, 'name' | 'allowCrossMarketplaceDependenciesOn'>;
  /**
   * The marketplace of the declaring plugin when it is not the root, for a link deeper in the
   * dependency chain. Its own `allowCrossMarketplaceDependenciesOn` is never consulted: only the
   * root's list applies, for the whole chain (marketplace-reference.md, "Top-level fields").
   */
  readonly declaringMarketplace?: Marketplace;
  /** `name@marketplace` of every plugin already enabled at the scope the install is for. */
  readonly enabledAtScope: ReadonlySet<string>;
  readonly declaredIn: DeclaredIn;
}

/**
 * Judgments (spec/judgments.md): a dependency in the root marketplace itself needs no entry on its
 * list (the user chose that marketplace); a name is matched exactly; a dependency with no
 * marketplace is never cross-marketplace.
 */
export const crossMarketplaceDecision = (
  question: CrossMarketplaceQuestion,
): CrossMarketplaceDecision => call('crossMarketplaceDecision', question);
