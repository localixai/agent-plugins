/**
 * The table of "Exit code output" (hooks.md): what a hook's stdout and its exit code come to
 * together. The rows are the kinds of stdout, the columns the exit codes; each cell is what the
 * doc says, as data.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/exit_codes.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/verdict/exit-codes');

export type ExitClass = 'zero' | 'two' | 'other';

/** "JSON object that passes schema validation", "JSON that can't be parsed or fails schema validation", "Plain text, or nothing". */
export type StdoutClass = 'json-valid' | 'json-invalid' | 'plain';

export const exitClassOf = (exitCode: number): ExitClass => call('exitClassOf', exitCode);

export type Result =
  /** "Success". */
  | 'success'
  /** "Blocking error". */
  | 'blocking'
  /** "Non-blocking error". */
  | 'non-blocking-error';

/** Whether the JSON fields count. */
export type Fields =
  /** "The fields apply". */
  | 'apply'
  /** "Claude Code still reads the fields, but they can't override the block". */
  | 'read-no-override'
  /** "Claude Code ignores the exit code, and the fields alone decide". */
  | 'decide-alone'
  | 'none';

/** What the message of the outcome is made of. */
export type MessageFrom =
  /** "Your stderr is the reason". */
  | 'stderr'
  /** "The notice carries the parse or validation message". */
  | 'validation'
  /** "The notice carries the first line of your stderr". */
  | 'first-stderr-line'
  | 'none';

export interface Cell {
  readonly result: Result;
  readonly fields: Fields;
  readonly message: MessageFrom;
  /**
   * For a hook "whose failure blocks the action": the run counts as a failure. Doc: "Success. Claude
   * Code ignores the exit code ... For hooks whose failure blocks the action, the run counts as a
   * failure instead"; and "Plain-text stdout from a command hook isn't a failure" (exit 0).
   */
  readonly failure: boolean;
}

export const EXIT_TABLE: {
  readonly [X in ExitClass]: { readonly [S in StdoutClass]: Cell };
} = call('EXIT_TABLE');
