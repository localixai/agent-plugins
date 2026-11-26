/**
 * The credential variables of mcp.md ("Credential variables that read as empty"): in a remote MCP
 * server's `url` and `headers` they read as empty, set or not, and a `:-default` on them is ignored,
 * so a project's `.mcp.json` or a plugin cannot send them to the server it names. The single home of
 * the list: plugin servers (`loading/resolve/surfaces`) and servers of the settings scopes
 * (`mcp-env`) read it here.
 *
 * The page lists names only "such as"; remote-mcp-credentials owns the oracle-proven full set.
 * The five names below are the documented examples, not the complete classifier.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/credential_env.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/credential-env');

export const CREDENTIAL_ENV_NAMES: readonly [
  'ANTHROPIC_API_KEY',
  'ANTHROPIC_AUTH_TOKEN',
  'AWS_BEARER_TOKEN_BEDROCK',
  'HTTPS_PROXY',
  'NPM_TOKEN',
] = call('CREDENTIAL_ENV_NAMES');

/** True for a name that reads as empty in the `url` and `headers` of a remote server. */
export const readsAsEmptyTowardRemote = (name: string): boolean =>
  call('readsAsEmptyTowardRemote', name);

/** The phrase mcp.md tells the reader to search the debug file for. */
export const NOT_EXPANDED_TOWARD_REMOTE: string = call('NOT_EXPANDED_TOWARD_REMOTE');

/**
 * The debug-log line for a covered variable the user has set and a remote server's `url` or
 * `headers` references. The docs give the phrase, not the whole line; the rest is a judgment. It
 * names the variable and never its value.
 */
export const credentialDebugLine = (name: string): string => call('credentialDebugLine', name);

export const FIXED_CREDENTIAL_NAMES: readonly [
  'ANTHROPIC_CUSTOM_HEADERS',
  'OTEL_EXPORTER_OTLP_HEADERS',
  'OTEL_EXPORTER_OTLP_TRACES_HEADERS',
  'OTEL_EXPORTER_OTLP_METRICS_HEADERS',
  'OTEL_EXPORTER_OTLP_LOGS_HEADERS',
] = call('FIXED_CREDENTIAL_NAMES');

/** True for a variable name that looks like a credential, by the pattern or the fixed list. */
export const isCredentialVariable = (name: string): boolean => call('isCredentialVariable', name);

/** `environment` without its credential variables. */
export const withoutCredentials = (
  environment: Readonly<Record<string, string>>,
): Readonly<Record<string, string>> => call('withoutCredentials', environment);

/** Whether `value` holds a URL whose userinfo carries a password: "a value that looks like a credential". */
export const hasUrlPassword = (value: string): boolean => call('hasUrlPassword', value);

/** `value` with the username and password cut from every URL in it. */
export const withoutUserinfo = (value: string): string => call('withoutUserinfo', value);

/** What the scrub does to one variable (env-vars.md, "What the subprocess environment scrub removes"). */
export type ScrubAction = 'remove' | 'keep' | 'cut-userinfo';

/** What the scrub does to the variable `name` holding `value`; a variable no rule names is kept. */
export const scrubAction = (name: string, value: string): ScrubAction =>
  call('scrubAction', name, value);
