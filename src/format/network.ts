import { isRecord } from '../protocol/is-record.ts';
import { warningAt, type Finding } from './validate/finding.ts';

/**
 * `network` of `plugin.json` (spec §2.4, Judgment J2.10): the hosts a plugin says it reaches,
 * `{ "hosts": [ … ] }`, each in `sandbox.network.allowedDomains` syntax. It neither allows nor
 * blocks a host; `claude plugin validate` lists them (§17.13.4). Every problem is a warning and the
 * value, or the one host, is ignored at load.
 */

const NOT_A_LIST =
  '\'network\' must be an object with a list of hosts, such as { "hosts": ["api.example.com"] }. It will be ignored at load time.';

/**
 * The spec quotes the start of the host warning only; the rest is the kit's (spec/judgments.md,
 * J2.10).
 */
const NOT_A_HOST =
  'Must be a host as sandbox.network.allowedDomains takes it, such as "api.example.com" or "*.example.com", without a scheme or path. It will be ignored at load time.';

/**
 * A host as `allowedDomains` takes it: plain ASCII labels, an optional leading `*.` wildcard, no
 * scheme, port or path (the kit's reading of the syntax, spec/judgments.md).
 */
const HOST =
  /^(?:\*\.)?[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?(?:\.[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?)*$/;

const isHost = (value: unknown): value is string => typeof value === 'string' && HOST.test(value);

/** The declared hosts that load: the well-formed ones, in order, each once. */
export function declaredHosts(value: unknown): readonly string[] {
  if (!isRecord(value) || !Array.isArray(value.hosts)) return [];
  return [...new Set(value.hosts.filter(isHost))];
}

/** The validator's warnings for a `network` value. */
export function networkFindings(value: unknown): Finding[] {
  if (!isRecord(value) || !Array.isArray(value.hosts)) return [warningAt('network', NOT_A_LIST)];
  const unknown = Object.keys(value)
    .filter((key) => key !== 'hosts')
    .map((key) =>
      warningAt(`network.${key}`, `Unknown field '${key}'. Claude Code ignores it at load time.`),
    );
  const hosts = value.hosts.flatMap((host: unknown, index) =>
    isHost(host) ? [] : [warningAt(`network.hosts[${index}]`, NOT_A_HOST)],
  );
  return [...hosts, ...unknown];
}
