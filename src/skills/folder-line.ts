import { coreModule } from '../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/skills/folder_line.rs`, docs/RUST.md). */
const call = coreModule('skills/folder-line');

/** The line a skill's text opens with, its folder after it. */
export const BASE_DIRECTORY: string = call('BASE_DIRECTORY');

/**
 * The text the model receives: a skill's opens with its folder and a blank line, a command's (no
 * folder) is the body alone (Claude Code 2.1.319, the spec's model-content oracle; the docs do not
 * show the line).
 */
export const withFolderLine = (text: string, dir: string | undefined): string =>
  call('withFolderLine', text, dir);
