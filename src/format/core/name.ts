/**
 * A core package's `name` (core §5.5): 1–64 characters of `a-z`, `0-9`, `-` and `.`, alphanumeric at
 * both ends, no `--` and no `..`. The checks are the schema's (`schemas/1.1.0/plugin.schema.json`,
 * `name`), each with its own reason so a report says which constraint failed (core §5.3).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/name.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/core/name');

/** Why a name breaks core §5.5, each failed constraint once; empty when it is valid. */
export const coreNameProblems = (name: string): readonly string[] => call('coreNameProblems', name);
