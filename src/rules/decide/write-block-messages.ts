/**
 * The refusals of `permissions.blockWritesOutsideWorkingDirectories`, verbatim (errors.md, "Setting
 * blocks writes outside the working directories" and "Writes to Claude Code settings files are
 * blocked"; settings-reference.md). `decide` answers with the reason (`outside-working-directory`,
 * `settings-file`); the host shows Claude the text the tool call fails with.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/write_block_messages.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/decide/write-block-messages');

/** What a write the block refuses is refused for. */
export type WriteBlockReason = 'outside-working-directory' | 'settings-file';

/** The facts the texts name: the path written, and the working directories it is outside of. */
export interface WriteBlockRefusalInput {
  readonly path: string;
  readonly workingDirectories: readonly string[];
}

/**
 * The text a write the block refuses fails with. The working directories are joined with `, `
 * (judgment, spec/judgments.md: the docs show the placeholder `<dirs>` only).
 */
export const writeBlockRefusal = (
  reason: WriteBlockReason,
  input: WriteBlockRefusalInput,
): string => call('writeBlockRefusal', reason, input);

/** "The refusal can also read" this, for a write outside the working directories. */
export const writeBlockShortRefusal = (path: string): string =>
  call('writeBlockShortRefusal', path);
