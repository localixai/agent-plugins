/**
 * `claude plugin validate <dir>` over a folder: one crossing per validation, the host's files read
 * through one load's view of them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/validate_plugin.rs`,
 * docs/RUST.md).
 */
import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ValidationReport } from './report.ts';

const call = coreModule('loading/validate/validate-plugin');

export interface ValidatePluginInput {
  /** Read access to the folder: its root is what `claude plugin validate <dir>` is given. */
  readonly files: PluginFiles;
  /** What the report calls the folder when it has nothing to validate; default `.`. */
  readonly label?: string;
  readonly signal?: AbortSignal;
}

/**
 * `claude plugin validate <dir>` over a folder (docs/API.md, "Validating"): the marketplace when the
 * folder has `.claude-plugin/marketplace.json` (and its plugin too when it has a manifest), else the
 * plugin, else a bare skills tree; a folder with none of them is an error, and so is one that does not exist. Findings are in the shape
 * the official CLI of the same release reports (docs/SPEC.md, "Sources of truth"). Never throws.
 */
export const validatePlugin = (input: ValidatePluginInput): Promise<ValidationReport> =>
  call.async('validatePlugin', input);
