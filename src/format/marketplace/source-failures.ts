/**
 * The two errors Claude Code gives for a marketplace source it parses and cannot load
 * (marketplace-reference.md, the `npm` row of "Marketplace sources" and "Source values valid only in
 * policy lists"), defined once for the load status, the notice and the fetch plan.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/source_failures.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/marketplace/source-failures');

export const SOURCE_FAILURE: {
  readonly npm: 'NPM marketplace sources not yet implemented';
  readonly policyOnly: 'Unsupported marketplace source type';
} = call('SOURCE_FAILURE');

/** The notice for a type only a policy list takes, outside one. */
export const policyOnlyNotice = (kind: string): string => call('policyOnlyNotice', kind);
