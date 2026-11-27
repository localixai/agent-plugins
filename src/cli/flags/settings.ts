import { coreModule } from '../../wasm/bridge.ts';

/**
 * `--setting-sources` and `--settings` (cli-reference.md, "CLI flags"). `--setting-sources` is "a
 * comma-separated list of setting sources to load (`user`, `project`, `local`)": managed policy is
 * read whatever the list says (agent-sdk/claude-code-features.md, "Control filesystem settings with
 * settingSources"). `--settings` is "a path to a settings JSON file or an inline JSON string": its
 * values "override the same keys in your `settings.json` files ... Keys you omit keep their
 * file-based values"; "the file must be a regular file no larger than 2 MiB". The refusals of the
 * file are errors.md's, "Settings file exceeds the 2MiB limit". Wording the docs do not give is in
 * spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/settings.rs`, docs/RUST.md); the
 * session plan reads the two flags there, so only the limit crosses.
 */

const call = coreModule('cli/flags/settings');

/** "The file must be a regular file no larger than 2 MiB." */
export const SETTINGS_FILE_LIMIT: number = call('SETTINGS_FILE_LIMIT');
