import type { Environment } from '../../../ports/index.ts';
import { coreModule } from '../../../wasm/bridge.ts';

/**
 * Environment variables in the headers of an `http` hook (hooks.md, "HTTP hook fields"): `headers`
 * values "support environment variable interpolation using `$VAR_NAME` or `${VAR_NAME}` syntax.
 * Only variables listed in `allowedEnvVars` are resolved", "References to unlisted variables are
 * replaced with empty strings. Required for any env var interpolation to work."
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/http_env.rs`,
 * docs/RUST.md).
 */

const call = coreModule('hooks/handlers/remote/http-env');

/** Every header of the hook with its value interpolated; names are lower case (the port's rule). */
export const interpolateHeaders = (
  headers: Readonly<Record<string, string>> | undefined,
  allowedEnvVars: readonly string[] | undefined,
  environment: Environment,
): Record<string, string> => call('interpolateHeaders', headers, allowedEnvVars, environment);
