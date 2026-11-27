import type { ManagedException } from './general/exceptions.ts';
import type { ScopeRule } from './key-spec.ts';
import type { MergeKind } from './merge.ts';

/**
 * What the kit knows about each policy key, one row per key, from its entry in settings-reference.md:
 * the files it may come from (the entry's **Scope** line), how files combine (**Scope** notes and
 * settings.md, "Lists merge instead of overriding"), and what to do with an invalid value
 * (managed-settings.md, "Keys that fail closed"). The resolvers of each area read their key through
 * the view and never repeat these facts. A row is data: the functions that apply it (a merge's
 * `combine`, an admin tier's `pinnedWhen`, a nested key's reader) run in the core.
 */
export interface ValueSpec<V> {
  readonly scopes: ScopeRule;
  /** The merge strategy the files combine by. */
  readonly merge: { readonly kind: MergeKind };
  /** The key's managed value is not an override: a restrictive value of a lower scope stays (settings.md). */
  readonly exception?: ManagedException<V>;
  /** How the admin tier holds the key; absent, every scope combines by `merge`. */
  readonly admin?: AdminTier;
  /** The names down a block to a key inside it, so the merged settings carry the value this row decided. */
  readonly path?: readonly string[];
}

/**
 * How the admin tier holds a key (managed-settings.md, "Let an embedding host add policy"). The admin
 * tier is the highest managed source that sets the key, else the parent settings.
 */
interface AdminTier {
  /** A managed value blocks the parent's; the other scopes still combine (`allowedMcpServers`). */
  readonly blocksParent?: boolean;
  /** The admin tier's value applies alone when it sets one; no other scope adds to it (`availableModels`). */
  readonly owns?: boolean;
}
