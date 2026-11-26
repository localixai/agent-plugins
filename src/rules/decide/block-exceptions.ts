import { coreModule } from '../../wasm/bridge.ts';
import type { PathContext } from '../types.ts';
import type { DecideContext } from './context.ts';
import type { WriteBlockReason } from './write-block-messages.ts';

/**
 * The read and write blocks (settings-reference.md, `permissions.blockReadsOutsideWorkingDirectories`
 * and `permissions.blockWritesOutsideWorkingDirectories`): what they refuse and the files Claude
 * Code itself needs, which stay open.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/block_exceptions.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/decide/block-exceptions');

/**
 * The working directories as the blocks count them: a directory that only repository settings add
 * "doesn't count" (settings-reference.md, `permissions.blockReadsOutsideWorkingDirectories`).
 */
export const blockPaths = (ctx: DecideContext): PathContext => call('blockPaths', ctx);

/** Why the write block refuses `paths`, or `undefined` when it lets them through. */
export const writeBlockReason = (
  paths: readonly string[],
  ctx: DecideContext,
): WriteBlockReason | undefined => call('writeBlockReason', paths, ctx);

/** Whether the read block refuses a read of `paths`. */
export const readBlocked = (paths: readonly string[], ctx: DecideContext): boolean =>
  call('readBlocked', paths, ctx);
