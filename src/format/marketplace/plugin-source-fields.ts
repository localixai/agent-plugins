/**
 * The field checks every object plugin source shares (plugins/marketplace-reference.md, "Plugin
 * sources" and "Invalid input on a source"): a known type "with a required field missing or of the
 * wrong type" is `Invalid input` on the entry's `source`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/plugin_source_fields.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { ReadMode } from './types.ts';

const call = coreModule('format/marketplace/plugin-source-fields');

/** Where a source is being read: the path its issues name, and what bare names and `\` depend on. */
export interface SourceSite {
  readonly path: string;
  readonly pluginRoot?: string;
  readonly platform: 'posix' | 'windows';
  /** `validate` leaves out what only installing or loading refuses (a backslash path). */
  readonly mode?: ReadMode;
}

export const invalidInput = (site: SourceSite): PluginIssue[] => call('invalidInput', site);

export type RawSource = Readonly<Record<string, unknown>>;
