/**
 * The JSON output of each event: the universal fields and an optional `hookSpecificOutput` that
 * "requires a `hookEventName` field set to the event name" (hooks.md, "JSON output"), so one
 * event's object does not validate on another.
 *
 * Judgment (spec/judgments.md): the official types have no `hookSpecificOutput` member for
 * `ConfigChange`, `PreCompact` and the other events the docs give none; an object for them carries
 * only `hookEventName`, which keeps an event name that matches and drops what follows.
 *
 * The schemas are the Rust core's (`crates/agent-plugins/src/hooks/output/schemas.rs`,
 * docs/RUST.md), where the output is checked (`parse.ts`); this module keeps the type of what they
 * answer.
 */

/**
 * A hook's JSON output after validation: known fields only, typed. `decision` is `approve` only
 * for `PreToolUse`; `hookSpecificOutput` is the event's own, checked against its official type.
 */
export interface CheckedOutput {
  continue?: boolean;
  suppressOutput?: boolean;
  stopReason?: string;
  decision?: 'approve' | 'block';
  systemMessage?: string;
  terminalSequence?: string;
  reason?: string;
  hookSpecificOutput?: object;
}
