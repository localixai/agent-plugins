/**
 * What the planned source of an `extraKnownMarketplaces` entry keeps of its request headers and
 * `headersHelper` commands (settings-reference.md, `extraKnownMarketplaces`; plugins/host-marketplace.md,
 * "When Claude Code skips a headersHelper command or drops its output"). Running a command is the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/headers_helper.rs`, docs/RUST.md).
 */
import type { SdkMarketplaceSource } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyScope } from '../layers.ts';

const call = coreModule('settings/marketplaces/headers-helper');

/**
 * The source a host plans from, for an entry a file of this scope declares. A kind nobody lists is
 * left as it is: the entry is invalid (`source-validate.ts`).
 */
export const plannedSource = (
  source: SdkMarketplaceSource,
  from: PolicyScope,
): SdkMarketplaceSource => call('plannedSource', source, from);
