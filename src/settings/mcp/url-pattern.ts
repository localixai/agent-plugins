/**
 * A `serverUrl` pattern against a server URL (managed-mcp.md, "How `serverUrl` entries match"):
 * `*` wildcards in the scheme, host, port and path; scheme and host case-insensitive and a trailing
 * FQDN dot ignored; paths case-sensitive; a pattern with no path matches any path. With no port, a
 * host written out in full allows the scheme's default port only (443 for `https`, 80 for `http`),
 * and a host with a `*` in it allows every port; `:*` allows every port and `:8443` that port.
 *
 * Judgments (spec/judgments.md): the server URL is read with the URL parser, so userinfo is never
 * part of the host, the default port written out equals no port and an IPv6 host keeps its brackets;
 * the path compared is the path and the query string (a fragment is never sent); a pattern or URL that
 * does not parse matches nothing; the glob is matched without backtracking blow-up, since the server's
 * URL is text a user controls.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/url_pattern.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/url-pattern');

export const urlPatternMatches = (pattern: string, url: string): boolean =>
  call('urlPatternMatches', pattern, url);

/**
 * Whether a `${VAR}` reference of an entry written as `template` sits in the scheme, host or path: the
 * "expansion that would change a URL entry's scheme, host, or path scope" that makes an allowlist
 * entry ignored (managed-mcp.md, "Environment variables in `serverCommand` and `serverUrl` entries").
 * A reference in the port or anywhere else leaves the scope as written. Judgment (spec/judgments.md):
 * the docs name the three parts without saying how an expansion "would change" one, so a reference
 * in any of them counts, and an entry that does not parse around its references (`${BASE}/*`) counts.
 */
export const referenceInUrlScope = (template: string): boolean =>
  call('referenceInUrlScope', template);
