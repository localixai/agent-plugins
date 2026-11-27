/**
 * The keys whose Scope is `Global config` in the settings index (settings-reference.md, "Settings
 * index"): they "apply only from `~/.claude.json`", so a settings file, `--settings` included,
 * cannot set them. The index of Claude Code 2.1.319 has these twelve; `permissionExplainerEnabled`
 * is kept because the index still lists it as removed rather than unknown.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/global_config.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/general/global-config');

export const GLOBAL_CONFIG_KEYS: readonly string[] = call('GLOBAL_CONFIG_KEYS');
