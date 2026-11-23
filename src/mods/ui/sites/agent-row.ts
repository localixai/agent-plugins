import { isRecord } from '../../../protocol/is-record.ts';
import { sameValue } from './band-list.ts';

/** The `AgentRow` props a hook may change: "read-only but for `label`, `body` ... and `isHidden`". */
const WRITABLE: ReadonlySet<string> = new Set(['label', 'body', 'isHidden']);

const agentRowProps = (e: unknown): Readonly<Record<string, unknown>> | undefined =>
  isRecord(e) && e.component === 'AgentRow' && isRecord(e.props) ? e.props : undefined;

/**
 * Why the `AgentRow` event a hook passes to `next` is refused (Agent Plugins Extended §10.9 item 5,
 * claude-code/index.d.ts:13702 of 2.1.330): a prop other than `label`, `body` and `isHidden` changed
 * or dropped. The wording is the kit's.
 */
export function agentRowChangeProblem(original: unknown, changed: unknown): string | undefined {
  const given = agentRowProps(original);
  if (given === undefined) return undefined;
  const passed = agentRowProps(changed) ?? {};
  const keys = new Set([...Object.keys(given), ...Object.keys(passed)]);
  const moved = [...keys].find((key) => !WRITABLE.has(key) && !sameValue(given[key], passed[key]));
  return moved === undefined
    ? undefined
    : `AgentRow prop "${moved}" is read-only: a hook changes label, body and isHidden only`;
}
