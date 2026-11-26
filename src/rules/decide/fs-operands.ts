import { coreModule } from '../../wasm/bridge.ts';
import type { ShellWord } from '../bash/words.ts';

/**
 * The filesystem commands `acceptEdits` approves (permission-modes.md, "Auto-approve file edits with
 * acceptEdits mode": "`mkdir`, `touch`, `rm`, `rmdir`, `mv`, `cp`, and `sed`") and the operands each
 * writes, or `undefined` when the call does more than write them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/fs_operands.rs`, docs/RUST.md).
 */
const call = coreModule('rules/decide/fs-operands');

export const isFilesystemProgram = (program: string): boolean =>
  call('isFilesystemProgram', program);

/** The operands a filesystem command writes (its words after the program), or `undefined`. */
export const filesystemOperands = (
  program: string,
  args: readonly ShellWord[],
): readonly ShellWord[] | undefined => call('filesystemOperands', program, args);
