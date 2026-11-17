import { warning, type PluginIssue } from '../../format/issues.ts';
import type { HandlerVerdict } from './types.ts';
import type { Subject } from './subject.ts';

/**
 * The debug-log lines the docs quote for how a hook's output was read, as warnings of the run: the
 * kit writes no log, the host does (judgment, spec/judgments.md; Agent Plugins Extended §7.4.12).
 */

/** hooks-guide.md, "Shell profile output before the JSON" (§7.4.1.1 item 5). */
const PLAIN_TEXT_LINE = 'Hook output does not start with {, treating as plain text';

/** hooks-guide.md, "Fields at the wrong level" (§7.4.5.1 item 1, Judgment J7.30). */
const UNRECOGNIZED_KEYS_LINE = 'Hook JSON output had unrecognized keys';

const pathOf = (subject: Subject): string => `hooks.${subject.event}.${subject.entry.id}`;

/** The line for stdout read as plain text because it does not start with `{`; none for empty stdout. */
export const plainTextNote = (subject: Subject, text: string): readonly PluginIssue[] =>
  text === '' || text.startsWith('{') ? [] : [warning(pathOf(subject), PLAIN_TEXT_LINE)];

/** The line for the keys a valid output had that the schema does not name, the keys after it. */
export const unrecognizedKeysNote = (
  subject: Subject,
  keys: readonly string[] | undefined,
): readonly PluginIssue[] =>
  keys === undefined || keys.length === 0
    ? []
    : [warning(pathOf(subject), `${UNRECOGNIZED_KEYS_LINE}: ${keys.join(', ')}`)];

/** `verdict` with `notes` added to its issues. */
export const withNotes = (verdict: HandlerVerdict, notes: readonly PluginIssue[]): HandlerVerdict =>
  notes.length === 0 ? verdict : { ...verdict, issues: [...verdict.issues, ...notes] };
