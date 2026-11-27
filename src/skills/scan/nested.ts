/**
 * Skills of a `.claude/skills` below the working directory (skills.md, "Load skills in monorepos and
 * subdirectories"): "Skills in a `.claude/skills/` directory below where you started don't load at
 * startup. They load the first time Claude reads or edits a file in that subdirectory and stay
 * available for the rest of the session."
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/scan/nested.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { ScanContext } from './locations.ts';
import type { ScanOutcome } from './scan-types.ts';

const call = coreModule('skills/scan/nested');

export interface NestedScan extends ScanOutcome {
  /** The directories scanned now: the session keeps them as loaded and never asks again. */
  readonly directories: readonly string[];
}

/**
 * The skills of the directories between the working directory and the file Claude touched, outermost
 * first, leaving out the ones the session already scanned (`loaded`). A file outside the working
 * directory loads nothing: that is `/add-dir`'s.
 */
export const scanNested = (
  context: ScanContext,
  touched: string,
  loaded: ReadonlySet<string> = new Set(),
): Promise<NestedScan> => call.async('scanNested', context, touched, loaded);
