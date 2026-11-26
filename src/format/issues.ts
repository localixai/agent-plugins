/**
 * What a check found, in the shape `claude plugin validate` reports (plugins/manifest-reference.md,
 * "Validate the manifest"): an error keeps the plugin from loading, a warning does not. `path`
 * names the field (`userConfig.api_token`) or the file the issue is about.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/issues.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/issues');

export const SEVERITIES: readonly ['error', 'warning'] = call('SEVERITIES');
export type Severity = (typeof SEVERITIES)[number];

export interface PluginIssue {
  readonly severity: Severity;
  readonly path: string;
  readonly message: string;
  /**
   * The advice the `/plugin` Errors tab shows under the message, when Claude Code gives one
   * (plugins/troubleshooting.md, "commands path not found": "with the guidance `Check that the path
   * in your manifest or marketplace config is correct`"). A host shows it with the message.
   */
  readonly guidance?: string;
}

export const error = (path: string, message: string): PluginIssue => call('error', path, message);

export const warning = (path: string, message: string): PluginIssue =>
  call('warning', path, message);

/** True when any issue keeps the plugin from loading. */
export const blocksLoading = (issues: readonly PluginIssue[]): boolean =>
  call('blocksLoading', issues);
