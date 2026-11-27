import { coreModule } from '../wasm/bridge.ts';

/**
 * What counts as plugin content at the top level of a directory a source delivers: the command
 * source's printed directory and an archive's root (plugins/marketplace-reference.md, "Output that
 * fails the install or update").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/plugin_content.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/plugin-content');

/** "Plugin content at its top level, such as a `.claude-plugin/` directory or a `skills/`, `commands/`, `agents/`, or `hooks/` directory." */
export const PLUGIN_CONTENT_DIRECTORIES: readonly string[] = call('PLUGIN_CONTENT_DIRECTORIES');
