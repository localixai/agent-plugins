import { error, type PluginIssue } from '../issues.ts';

/**
 * The `renames` map of `marketplace.json` (plugins/host-marketplace.md, "Migrate users with a
 * renames map"; plugins/marketplace-reference.md, "Validation messages"): each former plugin name
 * maps to its current name, or to `null` for a plugin that is gone. "Claude Code follows the chain
 * from the oldest name", so a name may map to a name that is itself a key, and the chain must end
 * at `null` or at a name in `plugins` (`claude plugin validate` rejects one that cycles or ends
 * anywhere else).
 */
export type Renames = Readonly<Record<string, string | null>>;

/**
 * Where a chain from `start` ends. `plugins` is checked before the keys: a name that is still a
 * plugin is where users end up (judgment, spec/judgments.md). Keys are read as own properties only,
 * so `constructor` or `__proto__` in a map or a target is a plain name.
 */
export type Chain =
  | { readonly end: 'plugin'; readonly name: string }
  | { readonly end: 'removed' }
  | { readonly end: 'dangling'; readonly name: string }
  | { readonly end: 'cycle'; readonly path: readonly string[] };

const isKey = (renames: Renames, name: string): boolean => Object.hasOwn(renames, name);

export function followRenames(
  renames: Renames,
  start: string,
  plugins: ReadonlySet<string>,
): Chain {
  const path = [start];
  let current = start;
  for (;;) {
    const target = renames[current];
    if (target === undefined || target === null) return { end: 'removed' };
    if (plugins.has(target)) return { end: 'plugin', name: target };
    if (!isKey(renames, target)) return { end: 'dangling', name: target };
    if (path.includes(target)) return { end: 'cycle', path: [...path, target] };
    path.push(target);
    current = target;
  }
}

const UNRESOLVED_TARGET = 'target must be a name in plugins[], a key in renames, or null';

/** The reason in the parentheses: `cycle`, or `target-missing` for a chain that ends at no plugin and no key (spec J15.11). */
const REASON: { readonly [E in 'dangling' | 'cycle']: string } = {
  dangling: 'target-missing',
  cycle: 'cycle',
};

/**
 * What `claude plugin validate` reports about `renames`, on `renames.<old>`: a chain that cycles or
 * ends at a name that is neither a plugin nor a key. A target is held to the id grammar only when it
 * names neither an entry nor a key, and such a target already ends the chain as `target-missing`,
 * so `"bad name"` draws that reason and a target that is itself a key (`"bad key"`) is followed
 * (spec J15.11, oracle 2.1.325): the docs' PluginIdSchema row is never the one printed.
 */
export function validateRenames(renames: Renames, pluginNames: readonly string[]): PluginIssue[] {
  const plugins = new Set(pluginNames);
  const found: PluginIssue[] = [];
  for (const old of Object.keys(renames)) {
    const chain = followRenames(renames, old, plugins);
    if (chain.end === 'cycle' || chain.end === 'dangling') {
      found.push(
        error(
          `renames.${old}`,
          `chain does not resolve (${REASON[chain.end]}) — ${UNRESOLVED_TARGET}`,
        ),
      );
    }
  }
  return found;
}
