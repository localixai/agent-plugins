/**
 * The remote fields of a core MCP server (core §7.2.1, "Streamable HTTP and legacy HTTP+SSE"): the
 * `url` is an absolute HTTP or HTTPS URL with no user information and no fragment, HTTPS unless the
 * host is exactly `localhost` or a loopback IP literal; `headers` are valid HTTP fields, no name
 * twice under different casing. Neither is expanded: `${...}` in them is literal text.
 *
 * Judgments (spec/judgments.md): the loopback literals are `127.0.0.0/8` and `[::1]`; a header name
 * is an RFC 9110 token, a value holds visible ASCII, obs-text, spaces and tabs, but not at either end.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/remote.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/core/remote');

/** Why a `url` breaks core §7.2.1, or `undefined` when it is valid. */
export const urlProblem = (value: unknown): string | undefined => call('urlProblem', value);

/** Why `headers` breaks core §7.2.1, or `undefined` when every entry is a valid HTTP field. */
export const headersProblem = (value: unknown): string | undefined => call('headersProblem', value);
