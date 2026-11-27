import { coreModule } from '../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/skills/folder_line.rs`, docs/RUST.md). */
const call = coreModule('skills/folder-line');

/** The line a skill's text opens with, its folder after it. */
export const BASE_DIRECTORY: string = call('BASE_DIRECTORY');
