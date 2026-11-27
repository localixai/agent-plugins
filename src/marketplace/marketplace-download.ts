import type { PluginIssue } from '../format/issues.ts';
import type { HttpClient } from '../ports/http-client.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { HeaderMap, MarketplaceFetchPlan } from './types.ts';

/**
 * Running the `http-file` plan of a `url` marketplace source (plugins/host-marketplace.md, "Stay
 * within the download limits for hosted files"): the plan's 5 MiB and 10 seconds, and a redirect to
 * a different origin "must use `https://` and can't point at a loopback, link-local, or
 * cloud-metadata host", its request carrying none of the configured headers. The docs give no
 * redirect count for this file: the archive's five holds, and a same-origin redirect is followed
 * unjudged (judgment, spec/judgments.md). `readMarketplaceText` reads what comes back.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/marketplace_download.rs`,
 * docs/RUST.md).
 */
const call = coreModule('marketplace/marketplace-download');

export type HttpFilePlan = Extract<MarketplaceFetchPlan, { readonly kind: 'http-file' }>;

/**
 * Downloads the marketplace file: its text, or the issue that refuses it. `headers` are the ones the
 * host resolved for the source (`resolveHeaders` over `urlSourceLayer`, the command's output
 * included); absent, the plan's own `headers`.
 */
export const fetchMarketplaceFile = (
  plan: HttpFilePlan,
  ports: { readonly http: HttpClient },
  options: { readonly signal: AbortSignal; readonly headers?: HeaderMap },
): Promise<{ readonly text: string } | PluginIssue> =>
  call.async('fetchMarketplaceFile', plan, ports, options);
