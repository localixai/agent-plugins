/**
 * The managed tier of what a host found at each managed source of the machine.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/load.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { ManagedSource, SourceRead } from './admin-source.ts';
import type { ComposeOptions, ManagedComposition } from './compose.ts';
import type { ManagedFileRead } from './drop-in.ts';

const call = coreModule('settings/managed/load');

/**
 * What a host found at each managed source of the machine (managed-settings.md, "Choose a delivery
 * mechanism"): the kit reads no file, plist, registry value or fetch. A source the machine does not
 * have is left out. `labels` name a source in a report: a path, `per-user managed preferences`,
 * `Registry: HKLM\SOFTWARE\Policies\ClaudeCode\Settings`.
 */
export type ManagedReads = {
  readonly [S in Exclude<ManagedSource, 'file'>]?: SourceRead;
} & {
  /** `managed-settings.json` and the `managed-settings.d/` folder beside it. */
  readonly file?: ManagedFileRead;
  readonly labels?: Readonly<Partial<Record<ManagedSource, string>>>;
};

/** The managed tier of the sources the host found. */
export const composeManagedReads = (
  reads: ManagedReads,
  options: ComposeOptions = {},
): ManagedComposition => call('composeManagedReads', reads, options);
