/**
 * How each target's hooks file names a hook adapter (Agent Plugins Extended §18.4.4), and how a
 * converter tells its own adapter apart when it converts back: an adapter registration of exactly
 * this shape is unwrapped to the handler it runs (judgment, spec/judgments.md), so a converted
 * package converts back to the hooks it had instead of an adapter around an adapter.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/adapters/registration.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('convert/adapters/registration');

export const RUN_CLAUDE_CODE_HANDLER: string = call('RUN_CLAUDE_CODE_HANDLER');
