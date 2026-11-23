import { isRecord } from '../../protocol/is-record.ts';
import { sameData } from './spec.ts';

/**
 * The refusal of an event passed to `next` with a pinned field changed (spec §10.16: `tool.check`'s
 * `isCeilingUnread` and `op`, `tool.describe`'s `mcpServer`, every field of `tool.run`). `'every'`
 * pins every field the event was fired with, and one a hook adds.
 */
export function pinnedFields(
  event: string,
  pinned: readonly string[] | 'every',
): (original: unknown, changed: unknown) => string | undefined {
  return (original, changed) => {
    if (!isRecord(original) || !isRecord(changed)) return undefined;
    const names =
      pinned === 'every' ? new Set([...Object.keys(original), ...Object.keys(changed)]) : pinned;
    const field = [...names].find((name) => !sameData(original[name], changed[name]));
    if (field === undefined) return undefined;
    return pinned === 'every'
      ? `${event}: every field of the call is pinned; "${field}" was changed`
      : `${event}: "${field}" is pinned; a hook cannot change it`;
  };
}
