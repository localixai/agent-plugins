/**
 * The keys Claude Code reads from every admin source, under both settings of `managedSourcesBehavior`
 * (managed-settings.md, "Keys read from every admin source"): "a lower-ranked MDM policy or managed
 * settings file can still set them when the selected source doesn't". The user-writable HKCU
 * registry is left out of the scan; the caller passes the admin sources only, highest first.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/cross_source.rs`, docs/RUST.md).
 */
import type { Settings } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/managed/cross-source');

export interface CrossSourceInput {
  /** The repaired settings of every admin source that delivers a policy key, highest first. */
  readonly admins: readonly Settings[];
  /** Whether each of those is a machine source (an MDM policy or a managed file), not server-managed settings. */
  readonly onMachine: readonly boolean[];
  /** `"merge"`: the machine's lists combine by the table rather than the highest alone. */
  readonly merging: boolean;
}

/** The managed settings with every cross-source key read from all the admin sources. */
export const withCrossSourceKeys = (input: CrossSourceInput, selected: Settings): Settings =>
  call('withCrossSourceKeys', input, selected);

/** `/status` names a source whose lock or allowlist is not the one in force (managed-settings.md, the end of "Keys read from every admin source"). */
export interface InForceNote {
  readonly index: number;
  readonly key: 'allowManagedMcpServersOnly' | 'allowedMcpServers';
}

export const notInForce = (admins: readonly Settings[], result: Settings): readonly InForceNote[] =>
  call('notInForce', admins, result);
