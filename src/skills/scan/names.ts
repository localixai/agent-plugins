/**
 * Whether a host path is inside another.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/scan/names.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('skills/scan/names');

/** Whether `path` is inside `parent`, and not `parent` itself. */
export const isStrictlyBelow = (parent: string, path: string): boolean =>
  call('isStrictlyBelow', parent, path);
