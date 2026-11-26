import type { HeaderMap } from './types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Header maps as the kit passes them to a download: names lower case, as on the `HttpClient` port
 * (`HeaderMap`). HTTP header names are case-insensitive, so "a header of the same name"
 * (plugins/host-marketplace.md, "Authenticate archive downloads") is compared without regard to
 * case; of two spellings in one map the later wins (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/header_map.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/header-map');

export const normalizeHeaders = (headers: Readonly<Record<string, string>>): HeaderMap =>
  call('normalizeHeaders', headers);

/** `later` replaces a header of the same name in `earlier`: the entry's value over the source's. */
export const mergeHeaders = (earlier: HeaderMap, later: HeaderMap): HeaderMap =>
  call('mergeHeaders', earlier, later);

/** `headers` without the request-routing and client-identity names; authentication names stay. */
export const withoutRoutingHeaders = (headers: HeaderMap): HeaderMap =>
  call('withoutRoutingHeaders', headers);
