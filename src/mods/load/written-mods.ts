import type { PermissionMode } from '../../protocol/official.ts';
import { guardOption, flagSetting, type ModsPolicyInput } from './policy-input.ts';

/**
 * A mod Claude writes in a session (Agent Plugins Extended §10.8.4; plugins/mods/create.md, "Ask
 * Claude for a mod", "Sessions that load a mod without asking" and "Sessions where a mod Claude
 * writes can't load"; troubleshoot.md, "Claude Code stops asking to enable hot reloading"): it loads
 * only after the person approves it, in a trusted workspace where mods may run.
 */

/** The three ways the page lists a session where it cannot load. */
export type WrittenModGate = 'no-approver' | 'untrusted' | 'disabled';

export interface WrittenModSession {
  /** False for a session that cannot show a prompt, such as a `claude -p` run. */
  readonly isInteractive: boolean;
  readonly permissionMode?: PermissionMode;
  /**
   * A resumed session: the mods already in its folder were written before, so it is not the `claude
   * -p` run that writes them, and they load without asking (§10.8.4).
   */
  readonly isResumed?: boolean;
}

export interface WrittenModRefusal {
  readonly gate: WrittenModGate;
  /** Wording ours (judgment): the page names the three sessions and no message. */
  readonly reason: string;
}

type Check = (policy: ModsPolicyInput, session: WrittenModSession) => string | undefined;

/**
 * Who can answer the question in each mode, for the `claude -p` run that writes the mod: `dontAsk`
 * answers nothing anywhere; `default` and `acceptEdits` would ask, and a `claude -p` run cannot
 * show the question; `auto` and `bypassPermissions` enable hot reloading without it (§10.8.4).
 */
type Approver = 'nobody' | 'person' | 'unasked';
const APPROVER: { readonly [M in PermissionMode]: Approver } = {
  default: 'person',
  acceptEdits: 'person',
  bypassPermissions: 'unasked',
  plan: 'unasked',
  dontAsk: 'nobody',
  auto: 'unasked',
};

const approverOf = (session: WrittenModSession): Approver =>
  APPROVER[session.permissionMode ?? 'default'];

const noApprover: Check = (_policy, session) => {
  const approver = approverOf(session);
  const writingRunCannotAsk =
    approver === 'person' && !session.isInteractive && session.isResumed !== true;
  return approver === 'nobody' || writingRunCannotAsk
    ? 'nobody is there to approve it: the session cannot show a prompt'
    : undefined;
};

/** The trust prompt is an interactive session's (§10.8.4: "an interactive session's workspace isn't trusted"). */
const untrusted: Check = (policy, session) =>
  session.isInteractive && policy.isTrusted === false
    ? 'the workspace is not trusted: accept the trust prompt first'
    : undefined;

/** `--safe-mode` and `--bare`, `disableAllHooks`, and what managed settings do to mods Claude writes. */
const disabled: Check = (policy) => {
  const reasons: readonly [boolean, string][] = [
    [policy.flags.safeMode === true, '--safe-mode turns mods off'],
    [policy.flags.bare === true, '--bare turns mods off'],
    [flagSetting(policy.user, 'disableAllHooks'), 'disableAllHooks is set in your settings'],
    [flagSetting(policy.managed, 'disableAllHooks'), 'disableAllHooks is set in managed settings'],
    [
      flagSetting(policy.managed, 'disableSideloadFlags'),
      'managed settings set disableSideloadFlags',
    ],
    // Judgment: the page says managed settings "block it" without naming the keys beyond disableSideloadFlags.
    [
      flagSetting(policy.managed, 'allowManagedHooksOnly'),
      'managed settings allow only managed hooks',
    ],
    [
      guardOption(policy, 'allowManagedModsOnly'),
      "managed settings allow only your organization's mods",
    ],
  ];
  return reasons.find(([on]) => on)?.[1];
};

/** In the order a person would read them: who can approve, whether the folder is trusted, whether mods run. */
const GATES: readonly (readonly [WrittenModGate, Check])[] = [
  ['no-approver', noApprover],
  ['untrusted', untrusted],
  ['disabled', disabled],
];

/** Why a mod Claude wrote cannot load in this session, or `undefined` when it can once it is approved. */
export function writtenModRefusal(
  policy: ModsPolicyInput,
  session: WrittenModSession,
): WrittenModRefusal | undefined {
  for (const [gate, check] of GATES) {
    const reason = check(policy, session);
    if (reason !== undefined) return { gate, reason };
  }
  return undefined;
}

/**
 * Whether the client asks before it loads the mods Claude writes in this session: never where they
 * cannot load (§10.8.4: "MUST NOT ask to load it"), never in `auto` and `bypassPermissions` modes,
 * which enable hot reloading without the question, and never in a resumed session, whose mods load
 * without asking. Plan mode writes no file, so it never asks either.
 */
export function writtenModAsks(policy: ModsPolicyInput, session: WrittenModSession): boolean {
  if (writtenModRefusal(policy, session) !== undefined || session.isResumed === true) return false;
  return approverOf(session) === 'person';
}

/** What became of the hot-reloading question (create.md, "Approve the mod"). */
export type HotReloadAnswer = 'enable' | 'not-now' | 'unanswered' | 'dismissed';

/** What a host keeps for the gate, so a resumed session still knows (create.md: "Your answer lasts for the session, including after you resume it"). */
export interface HotReloadState {
  readonly enabled: boolean;
  /** The question ended with no answer picked (a timeout such as `askUserQuestionTimeout`). */
  readonly unanswered: number;
  /** The person picked `Not now`. */
  readonly declined: boolean;
}

export interface HotReloadGate {
  /** Whether Claude Code asks when Claude saves a file in the mods folder. */
  shouldAsk(): boolean;
  /** Whether the mods in the session's mods folder load and reload. */
  enabled(): boolean;
  answered(answer: HotReloadAnswer): void;
  state(): HotReloadState;
}

/** "If the question ends three times without an answer picked, hot reloading stays off." */
const UNANSWERED_LIMIT = 3;

/**
 * The question of whether to enable hot reloading. `Enable for this session` turns it on and ends the
 * question; three questions that end unanswered leave it off and end it; a question the person
 * dismisses counts for nothing. Judgment: `Not now` ends the question for the session too (the page
 * says the mods load "the next time that session starts"), and the counts of unanswered questions
 * add up, an answer picked in between does not reset them.
 */
export function createHotReloadGate(initial?: HotReloadState): HotReloadGate {
  let state: HotReloadState = initial ?? { enabled: false, unanswered: 0, declined: false };
  const next: { readonly [A in HotReloadAnswer]: (from: HotReloadState) => HotReloadState } = {
    enable: (from) => ({ ...from, enabled: true }),
    'not-now': (from) => ({ ...from, declined: true }),
    unanswered: (from) => ({ ...from, unanswered: from.unanswered + 1 }),
    dismissed: (from) => from,
  };
  return {
    shouldAsk: () => !state.enabled && !state.declined && state.unanswered < UNANSWERED_LIMIT,
    enabled: () => state.enabled,
    answered(answer) {
      state = next[answer](state);
    },
    state: () => state,
  };
}
