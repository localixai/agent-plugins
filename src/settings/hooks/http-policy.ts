/** Implemented by the Rust core (`crates/agent-plugins/src/settings/hooks/http_policy.rs`, docs/RUST.md). */

/**
 * The two allowlists of HTTP hooks (settings-reference.md, `allowedHttpHookUrls` and
 * `httpHookAllowedEnvVars`). Both merge across files, both are read from "hooks from every source,
 * including managed settings", so the check takes a URL and a hook's own list and no origin.
 */
export type HttpHookVerdict =
  | { readonly run: true; readonly interpolate: readonly string[] }
  | { readonly run: false; readonly message: string };

export interface HttpHookUrlPolicy {
  /**
   * Whether an HTTP hook runs, and which environment variables its headers may interpolate.
   * `hookAllowedEnvVars` is the hook's own `allowedEnvVars`.
   */
  check(url: string, hookAllowedEnvVars: readonly string[]): HttpHookVerdict;
}
