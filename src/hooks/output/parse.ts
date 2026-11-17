import type { HookEvent } from '../../protocol/official.ts';
import type { ParseOutput } from '../verdict/types.ts';
import { hasText } from './fields.ts';
import { outputSchema, type CheckedOutput } from './schemas.ts';

/**
 * Reading one JSON value as the output of a hook (hooks.md, "JSON output", "Exit code 0"): "JSON
 * that can't be parsed or fails schema validation" is a non-blocking error, and the message names
 * what is wrong so the author can fix it. An unknown key is dropped; a value of the wrong type fails.
 */

export interface CheckedResult {
  readonly data?: CheckedOutput;
  readonly failure?: string;
}

/** Events whose `block` needs its explanation: "`reason` | Required when `decision` is `\"block\"`". */
const REASON_REQUIRED: ReadonlySet<HookEvent> = new Set<HookEvent>(['Stop', 'SubagentStop']);

const pathOf = (segments: readonly PropertyKey[]): string =>
  segments.reduce<string>(
    (path, segment) =>
      typeof segment === 'number'
        ? `${path}[${segment}]`
        : path === ''
          ? String(segment)
          : `${path}.${String(segment)}`,
    '',
  );

const missingReason = (event: HookEvent, data: CheckedOutput): boolean =>
  REASON_REQUIRED.has(event) && data.decision === BLOCK && !hasText(data.reason);

const BLOCK = 'block';

export function checkOutput(event: HookEvent, json: unknown): CheckedResult {
  const parsed = outputSchema(event).safeParse(json);
  if (!parsed.success) {
    const found = parsed.error.issues.map((issue) => {
      const where = pathOf(issue.path);
      return where === '' ? issue.message : `${where}: ${issue.message}`;
    });
    return { failure: found.join('; ') };
  }
  if (missingReason(event, parsed.data)) {
    return { failure: `reason: Required when decision is "${BLOCK}" on ${event}` };
  }
  return { data: parsed.data };
}

const keysOf = (value: unknown): readonly string[] =>
  typeof value === 'object' && value !== null && !Array.isArray(value) ? Object.keys(value) : [];

const SPECIFIC = 'hookSpecificOutput';

/**
 * The keys of `json` the schema dropped: at the top level, and inside a `hookSpecificOutput` the
 * schema kept (named `hookSpecificOutput.<key>`). One that is gone whole is one key.
 */
function unrecognizedKeys(json: unknown, data: CheckedOutput): readonly string[] {
  const kept = new Set(Object.keys(data));
  const top = keysOf(json).filter((key) => !kept.has(key));
  if (!kept.has(SPECIFIC) || typeof json !== 'object' || json === null || !(SPECIFIC in json)) {
    return top;
  }
  const keptInside = new Set(keysOf(data.hookSpecificOutput));
  const inside = keysOf(json.hookSpecificOutput).filter((key) => !keptInside.has(key));
  return [...top, ...inside.map((key) => `${SPECIFIC}.${key}`)];
}

/**
 * The output stage's `parse`: the checked output, or why it failed. A key the schema does not name
 * is ignored and listed in `unrecognizedKeys`, for the debug line `Hook JSON output had
 * unrecognized keys` (Agent Plugins Extended §7.4.5.1 item 1, Judgment J7.30).
 */
export const parseHookOutput: ParseOutput = (event, json) => {
  const checked = checkOutput(event, json);
  if (checked.data === undefined) return { failure: checked.failure ?? 'the output is not valid' };
  const unknown = unrecognizedKeys(json, checked.data);
  return { output: checked.data, ...(unknown.length === 0 ? {} : { unrecognizedKeys: unknown }) };
};
