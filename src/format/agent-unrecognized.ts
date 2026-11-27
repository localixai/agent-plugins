import { coreModule } from '../wasm/bridge.ts';
import type { UnrecognizedField } from './agent-record.ts';

/**
 * Which top-level fields of an agent file Claude Code does not recognize, each with the supported
 * field it most likely means.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/agent_unrecognized.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/agent-unrecognized');

/**
 * The top-level fields of an agent file that Claude Code does not recognize (sub-agents.md,
 * "Frontmatter reference": it "ignores a field it doesn't recognize without reporting an error";
 * the debug log names up to five, with the likely intended name). In file order.
 */
export const unrecognizedFields = (
  fields: Readonly<Record<string, unknown>>,
): readonly UnrecognizedField[] => call('unrecognizedFields', fields);
