/**
 * WebFetch's cache time, its download deadline and its notice of a redirect to another host
 * (tools-reference.md, "WebFetch tool behavior").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/web_fetch_timings.rs`,
 * docs/RUST.md).
 */
import type { Environment } from '../../../ports/environment.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/web-fetch-timings');

export interface FetchTimings {
  /** How long a fetched URL's response stays cached. */
  readonly cacheTtlMs: number;
  /** How long a download may take, redirects included; `undefined` when `0` removed the limit. */
  readonly deadlineMs: number | undefined;
}

/**
 * The cache time and the download deadline of WebFetch (tools-reference.md, "WebFetch tool
 * behavior"; env-vars.md). `CLAUDE_CODE_WEBFETCH_CACHE_TTL_MS` "takes plain digits only; `0`, a
 * decimal, or any other spelling keeps the default". `CLAUDE_CODE_WEBFETCH_DEADLINE_MS` takes plain
 * digits, `0` removes the limit, and a decimal or any other spelling keeps the default.
 */
export const webFetchTimings = (env: Pick<Environment, 'get'>): FetchTimings =>
  call('webFetchTimings', env);

/**
 * "When a URL redirects to a different host, WebFetch returns a text result that names the original
 * URL and the redirect target instead of following it" (tools-reference.md). `undefined` when the
 * target is on the same host, which is followed. Hosts compare by hostname, ignoring case, port and
 * scheme, and the wording is a judgment (spec/judgments.md).
 */
export const redirectNotice = (original: string, target: string): string | undefined =>
  call('redirectNotice', original, target);
