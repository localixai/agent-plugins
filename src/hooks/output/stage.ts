/**
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/stage.rs`, docs/RUST.md).
 */
import type { ScratchFiles } from '../../ports/scratch-files.ts';
import type { OutputStage } from '../verdict/types.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/stage');

export interface OutputStageOptions {
  /** Where text over the 10,000-character cap is saved; without it such text is cut at the cap. */
  readonly scratch?: ScratchFiles;
}

/** The real output stage the engine takes: parse, cap, merge (hooks.md, "JSON output"). */
export const createOutputStage = (options: OutputStageOptions = {}): OutputStage =>
  call('createOutputStage', options);
