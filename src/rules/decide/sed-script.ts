/**
 * Whether a `sed` call only edits the files it names (permission-modes.md, "Auto-approve file edits
 * with acceptEdits mode" lists `sed` as an edit of its operands). A script can also write another
 * file (`w`, `W`, the `s///w` flag), run a command (`e`, the `s///e` flag) or read one (`r`, `R`),
 * and `-f` reads the script from a file the kit does not see: none of those is an edit of the
 * operands, so the call is not approved (spec/judgments.md). The reading is GNU sed's; a script the
 * reader does not know is unsafe.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/sed_script.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { ShellWord } from '../bash/words.ts';

const call = coreModule('rules/decide/sed-script');

/**
 * The files a sed call edits, or `undefined` when its script does more than edit them (a script
 * a variable hides counts as unsafe).
 */
export const sedOperands = (args: readonly ShellWord[]): readonly ShellWord[] | undefined =>
  call('sedOperands', args);
