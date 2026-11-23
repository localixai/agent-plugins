import { EXIT2_RULES } from './exit2.ts';
import { handlerLabel, hookError, hookName } from './notices.ts';
import { routedBlock, shownWith } from './route.ts';
import type { Subject } from './subject.ts';
import type { HandlerVerdict } from './types.ts';

/**
 * A hook that starts a background process without redirecting its output (hooks.md, "Hooks that
 * start a background process"): "If your script prints an allow, then starts a background process
 * without redirecting that process's stdout and stderr, Claude Code blocks the tool call. The hook
 * error includes `refusing to honor the suspect capture`." A decision printed after "Claude Code has
 * stopped reading" never counts: the run is what the process printed up to its exit.
 *
 * Judgments (spec/judgments.md): the docs show an allow; any exit but 2 whose capture is held open
 * is refused, since the same late write could be a deny. The refusal blocks where exit 2 does (and
 * where a failure denies, `PermissionRequest`), not on the events that keep Claude working
 * (`Stop`, `SubagentStop`, `TaskCompleted`, `TeammateIdle`), where blocking would loop.
 */

/**
 * 2.1.330: "a hook error that includes `stdio went quiet` and `refusing to honor the suspect
 * capture`" (Agent Plugins Extended §7.4.2 item 5). Judgment (spec/judgments.md): the two joined
 * by "; " after the hook's own process exited.
 */
const SUSPECT_CAPTURE =
  "stdio went quiet after the hook's process exited, but a background process still holds it open; refusing to honor the suspect capture";

function blocksHere(subject: Subject): boolean {
  const rule = EXIT2_RULES[subject.event];
  return (
    (rule.canBlock || rule.failureDenies === true) &&
    rule.ignoresOnFailure !== true &&
    rule.cannotBlockWhen?.(subject.facts) !== true
  );
}

export function suspectCapture(subject: Subject): HandlerVerdict {
  const message = hookError(
    hookName(subject.event, subject.toolName),
    handlerLabel(subject.entry.handler),
    SUSPECT_CAPTURE,
  );
  const base = { handler: subject.entry, succeeded: false, issues: [] };
  if (!blocksHere(subject)) {
    return {
      ...base,
      status: 'non-blocking-error',
      notice: { handlerId: subject.entry.id, message },
    };
  }
  const block = routedBlock(subject.event, 'failure', { reason: SUSPECT_CAPTURE, message });
  return { ...base, status: 'blocked', block, ...shownWith(subject.entry.id, block) };
}
