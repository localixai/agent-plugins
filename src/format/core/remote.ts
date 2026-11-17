import { isRecord } from '../../protocol/is-record.ts';

/**
 * The remote fields of a core MCP server (core §7.2.1, "Streamable HTTP and legacy HTTP+SSE"): the
 * `url` is an absolute HTTP or HTTPS URL with no user information and no fragment, HTTPS unless the
 * host is exactly `localhost` or a loopback IP literal; `headers` are valid HTTP fields, no name
 * twice under different casing. Neither is expanded: `${...}` in them is literal text.
 *
 * Judgments (spec/judgments.md): the loopback literals are `127.0.0.0/8` and `[::1]`; a header name
 * is an RFC 9110 token, a value holds visible ASCII, obs-text, spaces and tabs, but not at either end.
 */

const IPV4_LOOPBACK = /^127\.\d{1,3}\.\d{1,3}\.\d{1,3}$/;

const isLoopback = (hostname: string): boolean =>
  hostname === 'localhost' || hostname === '[::1]' || IPV4_LOOPBACK.test(hostname);

const SCHEMES: Readonly<Record<string, boolean>> = { 'https:': true, 'http:': false };

function parsed(text: string): URL | undefined {
  try {
    return new URL(text);
  } catch {
    return undefined;
  }
}

/** User information, even an empty one (`https://@host`), which `URL` would not show. */
const holdsUserInfo = (url: URL, text: string): boolean =>
  url.username !== '' || url.password !== '' || /^[a-z][a-z0-9+.-]*:\/\/[^/?#]*@/i.test(text);

/** Why a `url` breaks core §7.2.1, or `undefined` when it is valid. */
export function urlProblem(value: unknown): string | undefined {
  if (typeof value !== 'string' || value.length === 0) return 'must be a non-empty string';
  const url = parsed(value);
  if (url === undefined) return 'must be an absolute URL';
  const secure = SCHEMES[url.protocol];
  if (secure === undefined) return 'must be an http or https URL';
  if (holdsUserInfo(url, value)) return 'must not hold user information';
  if (value.includes('#')) return 'must not hold a fragment';
  if (!secure && !isLoopback(url.hostname)) {
    return 'must use https unless the host is localhost or a loopback address';
  }
  return undefined;
}

const TOKEN = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;
const FIELD_VALUE = /^(?:[\x21-\x7e\x80-\xff](?:[\t\x20-\x7e\x80-\xff]*[\x21-\x7e\x80-\xff])?)?$/;

function headerProblem(name: string, value: unknown): string | undefined {
  if (!TOKEN.test(name)) return `header name "${name}" is not a valid HTTP field name`;
  if (typeof value !== 'string') return `header "${name}" must be a string`;
  return FIELD_VALUE.test(value)
    ? undefined
    : `header "${name}" has a value that is not a valid HTTP field value`;
}

/** Why `headers` breaks core §7.2.1, or `undefined` when every entry is a valid HTTP field. */
export function headersProblem(value: unknown): string | undefined {
  if (!isRecord(value)) return 'must be an object of strings';
  const seen = new Set<string>();
  for (const [name, member] of Object.entries(value)) {
    const problem = headerProblem(name, member);
    if (problem !== undefined) return problem;
    const folded = name.toLowerCase();
    if (seen.has(folded)) return `header "${name}" is given twice under different casing`;
    seen.add(folded);
  }
  return undefined;
}
