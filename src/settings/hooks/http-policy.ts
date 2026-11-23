import type { PolicyView } from '../view.ts';
import { matchUrlPattern } from './url-pattern.ts';

/**
 * The two allowlists of HTTP hooks (spec §12.7.1, `allowedHttpHookUrls` and
 * `httpHookAllowedEnvVars`; Judgment J12.18): "defined in managed settings it covers the hooks of
 * managed settings, defined in any file it covers every other HTTP hook". A covered hook is checked
 * against the list merged across files, so a user entry can admit a managed hook's URL once a managed
 * list exists. A hook of a plugin force-enabled by managed settings is an other hook.
 */
export type HttpHookVerdict =
  | { readonly run: true; readonly interpolate: readonly string[] }
  | { readonly run: false; readonly message: string };

/** Which hook asks: `managed` is a hook of managed settings; absent or `false`, any other. */
export interface HttpHookOrigin {
  readonly managed?: boolean;
}

export interface HttpHookUrlPolicy {
  /**
   * Whether an HTTP hook runs, and which environment variables its headers may interpolate.
   * `hookAllowedEnvVars` is the hook's own `allowedEnvVars`.
   */
  check(url: string, hookAllowedEnvVars: readonly string[], hook?: HttpHookOrigin): HttpHookVerdict;
}

/** The merged lists that limit one kind of hook; a key that does not cover it is absent. */
export interface HttpHookLists {
  readonly allowedUrls?: readonly string[];
  readonly allowedEnvVars?: readonly string[];
}

/**
 * Judgment (spec/judgments.md): the docs say "blocks the rest without running them" and give no
 * text. The message names the key and not the URL, which can carry a token.
 */
const BLOCKED_MESSAGE =
  'HTTP hook blocked: its URL matches no pattern in allowedHttpHookUrls, so the hook did not run';

const unique = (names: readonly string[]): readonly string[] => [...new Set(names)];

type ListKey = 'allowedHttpHookUrls' | 'httpHookAllowedEnvVars';

/** The merged list when it covers the hook: a managed hook only by a managed definition. */
function coveringList(view: PolicyView, key: ListKey, managedHook: boolean) {
  const merged = view.get(key);
  if (merged === undefined || !managedHook) return merged;
  return view.eachFromHighest(key).some((holder) => holder.scope === 'managed')
    ? merged
    : undefined;
}

/** The lists that cover the hooks of managed settings, or every other hook (J12.18). */
export function httpHookLists(view: PolicyView, hook: HttpHookOrigin = {}): HttpHookLists {
  const managedHook = hook.managed ?? false;
  const urls = coveringList(view, 'allowedHttpHookUrls', managedHook);
  const variables = coveringList(view, 'httpHookAllowedEnvVars', managedHook);
  return {
    ...(urls === undefined ? {} : { allowedUrls: urls }),
    ...(variables === undefined ? {} : { allowedEnvVars: variables }),
  };
}

export function createHttpHookPolicy(view: PolicyView): HttpHookUrlPolicy {
  return {
    check: (url, hookAllowedEnvVars, hook) => {
      const { allowedUrls, allowedEnvVars } = httpHookLists(view, hook);
      if (
        allowedUrls !== undefined &&
        !allowedUrls.some((pattern) => matchUrlPattern(pattern, url))
      ) {
        return { run: false, message: BLOCKED_MESSAGE };
      }
      // A covered hook can use a variable only if both its own list and the key name it.
      const interpolate = unique(hookAllowedEnvVars).filter(
        (name) => allowedEnvVars === undefined || allowedEnvVars.includes(name),
      );
      return { run: true, interpolate };
    },
  };
}
