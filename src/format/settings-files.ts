import { coreModule } from '../wasm/bridge.ts';

/**
 * Where the three settings files a person edits live, as settings.md writes them ("Settings files"):
 * the user file under the configuration folder, the project and local files under `.claude/` of the
 * project. One spelling for every message, label and path the kit builds from them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/settings_files.rs`, docs/RUST.md).
 */
const call = coreModule('format/settings-files');

export const SETTINGS_FILE: {
  /** The user file's name inside the configuration folder (`~/.claude` or `CLAUDE_CONFIG_DIR`). */
  readonly userName: 'settings.json';
  /** The user file as the docs write it. */
  readonly user: '~/.claude/settings.json';
  /** The project file, relative to the project folder. */
  readonly project: '.claude/settings.json';
  /** The local file, relative to the folder it stays in. */
  readonly local: '.claude/settings.local.json';
} = call('SETTINGS_FILE');
