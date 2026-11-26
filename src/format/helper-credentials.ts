/**
 * The variables Claude Code removes from the environment of a `headersHelper` that a project
 * `.mcp.json` or a plugin supplies (mcp.md, "Which variables a helper can read": "every variable
 * ... whose name looks like a credential, such as a name with `TOKEN`, `SECRET`, `PASSWORD`, `KEY`,
 * or `AUTH` in it in either letter case", Git's `GIT_CONFIG_KEY_<n>` apart, and "a fixed list of
 * credential variables whose names don't follow that pattern"). The page gives the rule only by
 * example; this is Claude Code's, observed with the spec's headers-helper-env oracle
 * (spec/judgments.md, "headersHelper credential names") and checked against the recorded result by
 * tests/loading/helper-credentials.test.ts.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/helper_credentials.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/helper-credentials');

/** Whether a supplied `headersHelper` runs without `name` in its environment. */
export const removedFromHeadersHelper = (name: string): boolean =>
  call('removedFromHeadersHelper', name);
