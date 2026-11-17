/**
 * The request-routing and client-identity header names a catalog entry may not set
 * (plugins/host-marketplace.md, "When Claude Code skips a headersHelper command or drops its
 * output": "Claude Code drops request-routing and client-identity names such as `Host`, `Cookie`,
 * and `X-Forwarded-*` ... and keeps authentication names such as `Authorization`").
 *
 * The docs say "such as": the set is the one `claude plugin validate` 2.1.325 warns for, the spec's
 * Judgment J15.12 (spec/judgments.md). The one copy: the catalog validator, the archive download
 * and the settings-file filter all read it.
 */
const EXACT = new Set([
  'host',
  'cookie',
  'forwarded',
  'x-real-ip',
  'x-client-ip',
  'true-client-ip',
  'client-ip',
  'cf-connecting-ip',
  'fastly-client-ip',
  'x-originating-ip',
  'x-remote-ip',
  'x-remote-addr',
  'x-cluster-client-ip',
  'connection',
  'upgrade',
  'transfer-encoding',
  'content-length',
  'te',
  'trailer',
  'expect',
  'via',
]);
const PREFIXES: readonly string[] = ['x-forwarded-', 'x-original-', 'proxy-'];

/** True for a routing or identity name, lowercased with `_` read as `-` (J15.12). */
export function isRoutingHeader(name: string): boolean {
  const folded = name.toLowerCase().replaceAll('_', '-');
  return EXACT.has(folded) || PREFIXES.some((prefix) => folded.startsWith(prefix));
}
