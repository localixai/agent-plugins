/**
 * How the layers that set one key combine into its value. A strategy is typed over the key's own
 * value type `V`: `combine` takes the key's value in every layer that sets it, highest precedence
 * first, and returns a `V`. One strategy per kind, never a branch.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/merge.rs`, docs/RUST.md).
 */
import type { HooksSettings } from '../protocol/hook-config.ts';
import type { Settings } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('settings/merge');

/** The `permissions` object of the SDK's settings type, and the `autoMode` object. */
export type PermissionsBlock = NonNullable<Settings['permissions']>;
export type AutoModeBlock = NonNullable<Settings['autoMode']>;

const MERGE_KINDS = [
  'highestWins',
  'unionArrays',
  'perKeyHighestWins',
  'hookGroups',
  'deepMerge',
  'permissionBlocks',
  'autoModeBlocks',
  'anyTrue',
] as const;
export type MergeKind = (typeof MERGE_KINDS)[number];

/** What every strategy has, whatever value type it is typed over. */
interface AnyMerge {
  readonly kind: MergeKind;
  readonly combine: (values: readonly never[]) => unknown;
}

/**
 * The strategies, one per kind (settings.md, "Settings precedence" and "Lists merge instead of
 * overriding"; managed-settings.md, "Split a file-based policy across teams"). The core export takes
 * the kind as its first argument, as a table of functions cannot cross.
 */
export const MERGES = {
  highestWins: {
    kind: 'highestWins',
    combine: <T>(values: readonly T[]): T | undefined => call('MERGES', 'highestWins', values),
  },
  unionArrays: {
    kind: 'unionArrays',
    combine: <E>(values: readonly (readonly E[])[]): E[] => call('MERGES', 'unionArrays', values),
  },
  perKeyHighestWins: {
    kind: 'perKeyHighestWins',
    combine: <T>(values: readonly Readonly<Record<string, T>>[]): Record<string, T> =>
      call('MERGES', 'perKeyHighestWins', values),
  },
  hookGroups: {
    kind: 'hookGroups',
    combine: (values: readonly HooksSettings[]): HooksSettings =>
      call('MERGES', 'hookGroups', values),
  },
  deepMerge: {
    kind: 'deepMerge',
    combine: <V>(values: readonly V[]): V => call('MERGES', 'deepMerge', values),
  },
  permissionBlocks: {
    kind: 'permissionBlocks',
    combine: (blocks: readonly PermissionsBlock[]): PermissionsBlock =>
      call('MERGES', 'permissionBlocks', blocks),
  },
  autoModeBlocks: {
    kind: 'autoModeBlocks',
    combine: (blocks: readonly AutoModeBlock[]): AutoModeBlock =>
      call('MERGES', 'autoModeBlocks', blocks),
  },
  anyTrue: {
    kind: 'anyTrue',
    combine: (values: readonly boolean[]): boolean | undefined => call('MERGES', 'anyTrue', values),
  },
} as const satisfies { readonly [K in MergeKind]: AnyMerge & { readonly kind: K } };
