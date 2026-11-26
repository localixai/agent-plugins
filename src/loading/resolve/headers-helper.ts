/**
 * What to do with a server's `headersHelper` each time the server connects (mcp.md, "Use dynamic
 * headers for custom authentication").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/headers_helper.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type {
  DirectoryKind,
  HeadersHelperOrigin,
  HelperContext,
} from './headers-helper-origins.ts';
import type { HelperNotice, HelperTrust } from './headers-helper-trust.ts';

const call = coreModule('loading/resolve/headers-helper');

/** The server a helper belongs to: the name Claude Code shows, its URL as expanded, and the command. */
export interface HelperServer {
  readonly name: string;
  readonly url: string;
  /**
   * The environment variables the expansion of `url` substituted (the `environment` entries of
   * `Substituted.referenced`). Only their live values are redacted from the URL the helper
   * receives; a value that merely occurs in the URL is not.
   */
  readonly expandedVariables: readonly string[];
  readonly headersHelper?: string | undefined;
  /**
   * The path variables `headersHelper` references, for the helper's environment: with the default
   * `SubstitutionScope.shellPaths` the command holds `${CLAUDE_PLUGIN_DATA}` and the like rather than
   * the path, so these give the references a value (judgment, spec/judgments.md). Absent or empty
   * for a pasted command and for a server no plugin supplies.
   */
  readonly paths?: Readonly<Record<string, string>>;
}

/** mcp.md: "gives up on it after 10 seconds". */
export const HEADERS_HELPER_TIMEOUT_MS: number = call('HEADERS_HELPER_TIMEOUT_MS');

export interface HelperRun {
  readonly run: true;
  readonly loaded: true;
  readonly headersOnly: false;
  readonly command: string;
  readonly workingDirectory: DirectoryKind;
  readonly cwd: string;
  readonly environment: Readonly<Record<string, string>>;
  /** Credential variables were taken out of `environment`. */
  readonly removesCredentials: boolean;
  readonly timeoutMs: number;
}

/** The server connects with its static `headers` alone; `notice` is the refusal a trust gate prints. */
export interface HelperSkipped {
  readonly run: false;
  readonly loaded: true;
  readonly headersOnly: true;
  readonly notice?: HelperNotice;
}

/** An agent file's server in a folder that is not trusted: not loaded, so nothing connects. */
export interface HelperNotLoaded {
  readonly run: false;
  readonly loaded: false;
}

export type HeadersHelperPlan = HelperRun | HelperSkipped | HelperNotLoaded;

/**
 * What to do with a server's `headersHelper` each time the server connects (mcp.md, "Use dynamic
 * headers for custom authentication"): where to run it, with what environment, or that it does not
 * run. Pure; the host runs it through `runHeadersHelper` and the plan is made again on every
 * connection ("the helper fresh on each connection ... doesn't cache the result").
 */
export const planHeadersHelper = (
  server: HelperServer,
  origin: HeadersHelperOrigin,
  trust: HelperTrust,
  context: HelperContext,
): HeadersHelperPlan => call('planHeadersHelper', server, origin, trust, context);
