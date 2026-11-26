/**
 * The dual-package check a host's validator runs (Agent Plugins Extended §18.3.1, §18.1.5 item 6):
 * on a root with both manifests, what of the shared manifest fields loads in one form only. It adds
 * to `validatePlugin`, which validates the Claude Code view alone, as `claude plugin validate` does
 * (J1.1). A root that is not a dual package gives `dual: false` and no findings.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/dual_check.rs`, docs/RUST.md).
 */
import type { ValidationIssue } from '../../format/validate/finding.ts';
import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/core/dual-check');

export const checkDualPackage = (input: {
  readonly files: PluginFiles;
  readonly signal?: AbortSignal | undefined;
}): Promise<{ readonly dual: boolean; readonly issues: readonly ValidationIssue[] }> =>
  call.async('checkDualPackage', input);
