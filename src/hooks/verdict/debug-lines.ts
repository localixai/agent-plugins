import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HandlerVerdict } from './types.ts';
import type { Subject } from './subject.ts';

/**
 * The debug-log lines the docs quote for how a hook's output was read, as warnings of the run: the
 * kit writes no log, the host does (judgment, spec/judgments.md; Agent Plugins Extended §7.4.12).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/debug_lines.rs`,
 * docs/RUST.md).
 */
const call = coreModule('hooks/verdict/debug-lines');

/** The line for stdout read as plain text because it does not start with `{`; none for empty stdout. */
export const plainTextNote = (subject: Subject, text: string): readonly PluginIssue[] =>
  call('plainTextNote', subject, text);

/** The line for the keys a valid output had that the schema does not name, the keys after it. */
export const unrecognizedKeysNote = (
  subject: Subject,
  keys: readonly string[] | undefined,
): readonly PluginIssue[] => call('unrecognizedKeysNote', subject, keys);

/** `verdict` with `notes` added to its issues. */
export const withNotes = (verdict: HandlerVerdict, notes: readonly PluginIssue[]): HandlerVerdict =>
  call('withNotes', verdict, notes);
