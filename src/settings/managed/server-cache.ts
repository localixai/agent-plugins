import type { RemotePolicy } from '../../format/host-errors.ts';
import type { Settings } from '../../protocol/official.ts';

/**
 * The server-managed cache at startup (spec §14.3.2): "The cache expires 24 hours after the last
 * successful fetch: a startup fetch that fails before then applies the cache, with a warning ...
 * that the cached policy is in effect; one that fails after then starts without server-managed
 * settings, the warning saying no remote policy applies, unless `forceRemoteSettingsRefresh` makes
 * the client exit." Pure over what the host keeps: the cached payload and when it was fetched.
 */
export const SERVER_CACHE_LIFETIME_MS = 24 * 60 * 60 * 1000;

/** The cached payload and the time of the successful fetch that wrote it, in epoch milliseconds. */
export interface ServerCache {
  readonly settings: Settings;
  readonly fetchedAt: number;
}

export interface FailedFetchFacts {
  /** The time of the startup fetch, in epoch milliseconds (the host's `Clock`). */
  readonly now: number;
  /** Some admin source sets `forceRemoteSettingsRefresh` (`ManagedComposition.forceRemoteRefresh`). */
  readonly forceRemoteSettingsRefresh?: boolean;
}

/**
 * What a failed startup fetch leaves: the cached settings and `policy: 'cached'`, or no
 * server-managed settings and `policy: 'none'` (the text of `remoteManagedSettingsFailed`), or an
 * exit. Judgment (spec/judgments.md): the cache applies while less than 24 hours have passed; at
 * exactly 24 hours it has expired.
 */
export type FailedFetchStart =
  | { readonly outcome: 'starts'; readonly settings: Settings; readonly policy: 'cached' }
  | { readonly outcome: 'starts'; readonly policy: Extract<RemotePolicy, 'none'> }
  | { readonly outcome: 'exits' };

export function startupOnFailedFetch(
  cache: ServerCache | undefined,
  facts: FailedFetchFacts,
): FailedFetchStart {
  if (facts.forceRemoteSettingsRefresh === true) return { outcome: 'exits' };
  const fresh = cache !== undefined && facts.now - cache.fetchedAt < SERVER_CACHE_LIFETIME_MS;
  return fresh
    ? { outcome: 'starts', settings: cache.settings, policy: 'cached' }
    : { outcome: 'starts', policy: 'none' };
}

type Env = Readonly<Record<string, string | undefined>>;

/** Where the proxy of the fetch can come from (spec §14.3.2 item 4). */
export interface FetchProxySources {
  /** The `env` of the endpoint-managed source (2.1.223 and later). */
  readonly endpointEnv?: Env;
  /** The `env` of user settings. */
  readonly userEnv?: Env;
  /** The environment Claude Code was launched with. */
  readonly launchEnv?: Env;
}

/**
 * The proxy the server-managed fetch goes through: `HTTPS_PROXY`, never `ALL_PROXY` alone.
 * Judgment (spec/judgments.md): the endpoint-managed value first, then user settings (a settings
 * `env` value replaces the shell's, §12.7.4), then the launch environment; an empty value is unset.
 */
export function serverFetchProxy(sources: FetchProxySources): string | undefined {
  return [sources.endpointEnv, sources.userEnv, sources.launchEnv]
    .map((env) => env?.HTTPS_PROXY)
    .find((value) => value !== undefined && value !== '');
}
