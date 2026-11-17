import { errorAt, warningAt, type Finding } from './finding.ts';
import { isRecord } from '../../protocol/is-record.ts';

/**
 * The checks of `claude plugin validate` on a remote server's `url` and `headers` (plugins/
 * manifest-reference.md, "Validate the manifest"; spec §8 item 3): an error for a `url` that is not
 * a valid absolute URL once its references are set aside, warnings for an `http://` or `ws://` URL
 * to a host that is not loopback and for a header value that looks like a literal credential. The
 * wording and the credential shapes are oracle 2.1.325's (spec/judgments.md).
 */

const NOT_ABSOLUTE =
  'url is not a valid absolute URL (for example "https://host/path"). The plugin loader marks this server as invalid config and it never connects.';

const cleartext = (scheme: string, secure: string): string =>
  `url uses ${scheme}// to a non-loopback host — requests, headers and any credentials travel in cleartext. Use ${secure}//; plugin directories may reject insecure remote server urls.`;

/** The cleartext schemes and the secure scheme each message names. */
const CLEARTEXT_SCHEMES: Readonly<Record<string, string>> = { 'http:': 'https:', 'ws:': 'wss:' };

const CREDENTIAL =
  'header value looks like a literal credential. Everything shipped in a plugin is readable by everyone who installs it; reference a sensitive userConfig option (${user_config.KEY}) or an environment variable (${VAR}) instead of committing the value.';

const LOOPBACK = new Set(['localhost', '127.0.0.1', '[::1]', '::1']);

/** A `${...}` reference: a value that holds one is not a literal and is checked without it. */
const REFERENCE = '${';

function urlFindings(path: string, url: unknown): Finding[] {
  // An empty url is a placeholder the server shows as `not configured` (spec §8.2, §8.8 item 6).
  if (typeof url !== 'string' || url === '' || url.includes(REFERENCE)) return [];
  if (!URL.canParse(url)) return [errorAt(`${path}.url`, NOT_ABSOLUTE)];
  const parsed = new URL(url);
  const secure = CLEARTEXT_SCHEMES[parsed.protocol];
  if (secure === undefined || LOOPBACK.has(parsed.hostname)) return [];
  return [warningAt(`${path}.url`, cleartext(parsed.protocol, secure))];
}

/** Header names whose value is a secret by their name (oracle 2.1.325: not `Cookie`). */
const SECRET_NAME = /authorization|api[-_]?key|token|secret|passw(?:or)?d|credential/i;
const AUTH_SCHEME = /^(?:bearer|basic|token)\s+/i;
const LONG_SECRET = /^[A-Za-z0-9._~+/=-]{20,}$/;

/** The shapes of well-known tokens, under any header name. */
const TOKEN_SHAPES: readonly RegExp[] = [
  /\bsk-[A-Za-z0-9_-]{20,}/,
  /\bgh[pousr]_[A-Za-z0-9]{36,}/,
  /\bglpat-[A-Za-z0-9_-]{20,}/,
  /\bxox[abprs]-[A-Za-z0-9-]{10,}/,
  /\bAKIA[A-Z0-9]{16}\b/,
  /\beyJ[\w-]{10,}\.eyJ[\w-]{10,}\.[\w-]+/,
];

/** The documentation placeholders of those shapes (`AKIAIOSFODNN7EXAMPLE`). */
const PLACEHOLDER = 'EXAMPLE';

function looksLikeCredential(name: string, value: string): boolean {
  if (value.includes(REFERENCE) || value.includes(PLACEHOLDER)) return false;
  if (TOKEN_SHAPES.some((shape) => shape.test(value))) return true;
  return SECRET_NAME.test(name) && LONG_SECRET.test(value.replace(AUTH_SCHEME, ''));
}

function headerFindings(path: string, headers: unknown): Finding[] {
  if (!isRecord(headers)) return [];
  return Object.entries(headers).flatMap(([name, value]) =>
    typeof value === 'string' && looksLikeCredential(name, value)
      ? [warningAt(`${path}.headers.${name}`, CREDENTIAL)]
      : [],
  );
}

/** The url and header findings of one server at `path`. */
export const remoteFindings = (
  path: string,
  server: Readonly<Record<string, unknown>>,
): Finding[] => [...urlFindings(path, server['url']), ...headerFindings(path, server['headers'])];
