/**
 * The two outer limits on `http` hooks (hooks.md, "Hook locations"; settings-reference.md,
 * `allowedHttpHookUrls` and `httpHookAllowedEnvVars`). They "apply to hooks from every source,
 * including managed policy settings", and the merged lists are the settings stream's: the host
 * passes them, already merged, in `RunContext.httpPolicy`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/http_policy.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

export interface HttpHookPolicy {
  /**
   * "When you define this key, Claude Code runs an HTTP hook only if its URL matches one of the
   * patterns and blocks the rest without running them; an empty array blocks every HTTP hook."
   * Patterns are URLs with `*` as a wildcard; unset allows any URL.
   */
  readonly allowedUrls?: readonly string[];
  /**
   * "A hook can use a variable only if both its own `allowedEnvVars` and this key name it." Unset
   * leaves each hook's own list.
   */
  readonly allowedEnvVars?: readonly string[];
}

const call = coreModule('hooks/handlers/remote/http-policy');

/** Whether `url` may run under the merged allowlist; no list allows every URL. */
export const urlAllowed = (url: string, allowedUrls: readonly string[] | undefined): boolean =>
  call('urlAllowed', url, allowedUrls);

/** The variables a hook may interpolate: its own list, cut to the settings list when one is set. */
export const permittedEnvVars = (
  own: readonly string[] | undefined,
  policy: HttpHookPolicy | undefined,
): readonly string[] | undefined => call('permittedEnvVars', own, policy);

/** The issue of an `http` hook whose URL the allowlist blocks; the hook does not run. */
export const blockedUrlIssue = (path: string, url: string): PluginIssue =>
  call('blockedUrlIssue', path, url);
