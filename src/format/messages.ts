/**
 * The load messages of a plugin's component paths, as Claude Code shows them in the `/plugin`
 * Errors tab and `claude plugin list` (plugins/manifest-reference.md, "Containment and existence"
 * and "How each key combines with its default location"; errors.md, "Path escapes plugin
 * directory"). `component` is the field that declared the path (`commands`, `hooks`, ...).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/messages.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/messages');

/** `<component> path not found: <path>`. */
export const pathNotFound = (component: string, path: string): string =>
  call('pathNotFound', component, path);
