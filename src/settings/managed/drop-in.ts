/**
 * A file-based policy split across teams (managed-settings.md, "Split a file-based policy across
 * teams"): `managed-settings.json` first, then every `*.json` of `managed-settings.d/` in
 * alphabetical order, "the later file's value replaces the earlier one". The host lists the folder
 * and reads each file; this module orders them, merges them and reports what is broken.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/drop_in.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { AdminDocument, SourceRead, UnreadableRead } from './admin-source.ts';

const call = coreModule('settings/managed/drop-in');

export interface DropInFile {
  readonly name: string;
  readonly read: SourceRead;
}

/** The `managed-settings.d/` folder: absent, present and unreadable, or its files. */
export type DropInDirectory =
  | undefined
  | UnreadableRead
  | { readonly files: readonly DropInFile[] };

export interface ManagedFileRead {
  /** `managed-settings.json`. */
  readonly base: SourceRead;
  readonly dropIns?: DropInDirectory;
  /** What a report calls each file and the folder: its path. */
  readonly label?: {
    readonly base?: string;
    readonly directory?: string;
    readonly dropIn?: (name: string) => string;
  };
}

/** "Claude Code ignores hidden files and files that don't end in `.json`." */
export const isDropInName = (name: string): boolean => call('isDropInName', name);

/**
 * The drop-in names in the order they merge: alphabetical, so numeric prefixes (`10-telemetry.json`,
 * `20-security.json`) control it. Plain code-unit order: the docs say "alphabetical" and no locale
 * (spec/judgments.md).
 */
export const dropInOrder = (names: readonly string[]): string[] => call('dropInOrder', names);

/** The shapes the file source takes in `/status`: `(file)`, `(drop-ins)` or `(file + drop-ins)`. */
export const FILE_SHAPES: readonly ['file', 'drop-ins', 'file + drop-ins'] = call('FILE_SHAPES');
export type FileShape = (typeof FILE_SHAPES)[number];

/**
 * The file source as one document: the base then the drop-ins merged; a part that is not a JSON
 * object fails the whole source (its issue names that part); a part that cannot be read leaves the
 * source present.
 */
export const readFileDocument = (read: ManagedFileRead): AdminDocument =>
  call('readFileDocument', read);
