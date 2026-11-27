/**
 * `gatewayInternalNetworks` (settings-reference.md, `gatewayInternalNetworks`; claude-apps-gateway.md,
 * "Allow a gateway on public address space you own"): an array of strings, at most four IPv4 CIDR
 * blocks, each `/8` to `/32`, not overlapping one another, none overlapping private space or space
 * that is never an organization's network. `240.0.0.0/4` is allowed.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/repair/network.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/repair/network');

export const gatewayNetworksFault = (value: unknown): string | undefined =>
  call('gatewayNetworksFault', value);
