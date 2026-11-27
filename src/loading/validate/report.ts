import type { ValidationIssue } from '../../format/validate/finding.ts';
import { coreModule } from '../../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/report.rs`, docs/RUST.md). */
const call = coreModule('loading/validate/report');

/** What `validatePlugin` looked at: the folder's marketplace, its plugin, only its components, or nothing it knows. */
export const VALIDATION_TARGETS: readonly ['marketplace', 'plugin', 'components', 'none'] =
  call('VALIDATION_TARGETS');
export type ValidationTarget = (typeof VALIDATION_TARGETS)[number];

export interface ValidationReport {
  readonly target: ValidationTarget;
  /** Every finding, in the order the validator reports them: the manifest, then each file. */
  readonly issues: readonly ValidationIssue[];
}
