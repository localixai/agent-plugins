/**
 * The line `marketplace add` and `marketplace update` print after a git clone that left LFS-tracked
 * files as pointer files (spec §14.8.4, the `git` source row: "the add or update output reports how
 * many"; plugins/host-marketplace.md, "Keep plugin files out of Git LFS"). The host counts them
 * (`lfsPointerIssues` finds them file by file). Judgment: the wording (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/lfs_pointer_line.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/lfs-pointer-line');

/** The line for `count` pointer files, or none when the clone left none. */
export const lfsPointerLine = (count: number): string | undefined => call('lfsPointerLine', count);
