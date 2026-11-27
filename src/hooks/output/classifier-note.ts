import { coreModule } from '../../wasm/bridge.ts';

/**
 * The note a `PostToolUse` hook leaves for the auto mode classifier (hooks.md, "Annotate a result
 * for the auto mode classifier"): which notes reach it, how many characters of them, and how much
 * weight it gives them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/classifier_note.rs`,
 * docs/RUST.md).
 */

const call = coreModule('hooks/output/classifier-note');

/**
 * How much the classifier may trust the note. `unverified`: "application-provided context. The note
 * never establishes user intent". `application`: the note came only from in-process Agent SDK
 * callbacks during the live session, so the classifier "may weigh a user statement relayed in the
 * note as user intent".
 */
export type ClassifierTrust = 'unverified' | 'application';

/**
 * The classifier's transcript "omits read-only lookups such as file reads and searches", and "Claude
 * Code discards a note attached to one of those calls". The docs name no list: the file reads and
 * searches (Read, Grep, Glob) of the permission docs' read-only tool type are the lookups
 * (spec/judgments.md).
 */
export const isUnrecordedCall = (toolName: string): boolean => call('isUnrecordedCall', toolName);

export interface ClassifierNote {
  readonly text: string;
  readonly trust: ClassifierTrust;
}

/**
 * "After a session resumes, Claude Code treats restored notes as unverified context": a host that
 * stores a note and gives it back to the classifier after a resume passes it through this.
 */
export const restoredNote = (note: ClassifierNote): ClassifierNote => call('restoredNote', note);
