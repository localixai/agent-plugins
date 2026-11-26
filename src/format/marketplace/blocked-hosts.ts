/**
 * Hosts a download may not point at: "can't point at a loopback, link-local, or cloud-metadata
 * host" (plugins/marketplace-reference.md, "archive plugin source"; plugins/host-marketplace.md,
 * "Stay within the download limits for hosted files"). The docs name three categories, not the
 * hosts in them, so the check is lexical, over the address as written, and the lists are the
 * kit's judgment (spec/judgments.md). The kit resolves no name: a DNS name that merely resolves to
 * a blocked address is for the host's `HttpClient` egress policy.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/blocked_hosts.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/marketplace/blocked-hosts');

/** True when `hostname` (as `URL.hostname` writes it, IPv6 in brackets) is loopback, link-local or a metadata host. */
export const isBlockedHost = (hostname: string): boolean => call('isBlockedHost', hostname);
