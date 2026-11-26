/**
 * The request-routing and client-identity header names a catalog entry may not set
 * (plugins/host-marketplace.md, "When Claude Code skips a headersHelper command or drops its
 * output": "Claude Code drops request-routing and client-identity names such as `Host`, `Cookie`,
 * and `X-Forwarded-*` ... and keeps authentication names such as `Authorization`").
 *
 * The docs say "such as": the set is the one `claude plugin validate` 2.1.325 warns for, the spec's
 * Judgment J15.12 (spec/judgments.md). The one copy: the catalog validator, the archive download
 * and the settings-file filter all read it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/header_names.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/marketplace/header-names');

/** True for a routing or identity name, lowercased with `_` read as `-` (J15.12). */
export const isRoutingHeader = (name: string): boolean => call('isRoutingHeader', name);
