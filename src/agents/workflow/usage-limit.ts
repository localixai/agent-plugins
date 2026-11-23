/**
 * When a run hits your usage limit (workflows.md, "When a run hits your usage limit"; §11.9): the run
 * pauses instead of failing the agent, when all of the conditions hold; when one does not, the
 * affected agent fails, and the run is finished by relaunching it after the reset (§11.8, a saved
 * `failed-at-limit` agent). Pure over facts the host holds.
 */

/**
 * `interactive` is an interactive terminal session; `desktop-code-tab` a local session started in
 * the desktop app's Code tab; `desktop-other` a desktop session that is not one (SSH, WSL, Dispatch,
 * a scheduled task, one started from Remote Control or through a corporate launcher).
 */
export const SESSION_KINDS = [
  'interactive',
  'desktop-code-tab',
  'desktop-other',
  'non-interactive',
  'background',
  'remote-control',
  'agent-team-teammate',
] as const;
export type SessionKind = (typeof SESSION_KINDS)[number];

/** §11.9 item 1: only an interactive terminal session or a local Code tab session pauses. */
const SESSION_PAUSES: { readonly [K in SessionKind]: boolean } = {
  interactive: true,
  'desktop-code-tab': true,
  'desktop-other': false,
  'non-interactive': false,
  background: false,
  'remote-control': false,
  'agent-team-teammate': false,
};

/** The Code tab, whose own rules are item 1's traffic switch and item 3's auto-continue choice. */
const CODE_TAB: { readonly [K in SessionKind]: boolean } = {
  interactive: false,
  'desktop-code-tab': true,
  'desktop-other': false,
  'non-interactive': false,
  background: false,
  'remote-control': false,
  'agent-team-teammate': false,
};

export const NO_WAIT_CAUSES = [
  'session-not-interactive',
  'no-subscription',
  'usage-based-billing',
  'nonessential-traffic-off',
  'auto-continue-off',
  'code-tab-auto-continue-off',
  'reset-too-far',
  'waited-twice',
  'old-release',
] as const;
export type NoWaitCause = (typeof NO_WAIT_CAUSES)[number];

/** A day, in milliseconds: "The limit resets within 24 hours". */
const DAY_MS = 24 * 60 * 60 * 1000;

/** "The run hasn't already waited twice. When it hits the limit a third time, the agent fails." */
const MAX_USAGE_WAITS = 2;

export interface UsageLimitFacts {
  readonly session: SessionKind;
  /** Signed in with a claude.ai subscription. */
  readonly subscription: boolean;
  /** The subscription has usage-based billing: the run does not pause (§11.9 item 1). */
  readonly usageBasedBilling?: boolean;
  /** `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is set: a Code tab session does not pause. */
  readonly nonessentialTrafficDisabled?: boolean;
  /** `autoContinueAtUsageLimit` is on. */
  readonly autoContinue: boolean;
  /** The Code tab's session-limit card leaves auto-continue on (the default; item 3). */
  readonly codeTabAutoContinue?: boolean;
  /** Managed settings set `autoContinueAtUsageLimit` to `true`, over the Code tab's choice. */
  readonly managedAutoContinue?: boolean;
  /** Milliseconds until the limit resets. A weekly limit can reset further out. */
  readonly msUntilReset: number;
  /** How many times the run has already waited. */
  readonly waitsSoFar: number;
  /** The release has the wait (Claude Code v2.1.271 or later); "on earlier versions, the affected agents fail". */
  readonly releaseHasWait: boolean;
}

export type UsageLimitDecision =
  | {
      /** The agents that hit the limit wait for the reset, and "no new agents start" while it lasts. */
      readonly pauses: true;
      /** Shown on the progress line and in the `/workflows` header. */
      readonly msUntilReset: number;
    }
  | { readonly pauses: false; readonly failsAgent: true; readonly causes: readonly NoWaitCause[] };

const CONDITIONS: { readonly [C in NoWaitCause]: (facts: UsageLimitFacts) => boolean } = {
  'session-not-interactive': (facts) => !SESSION_PAUSES[facts.session],
  'no-subscription': (facts) => !facts.subscription,
  'usage-based-billing': (facts) => facts.usageBasedBilling === true,
  'nonessential-traffic-off': (facts) =>
    CODE_TAB[facts.session] && facts.nonessentialTrafficDisabled === true,
  'auto-continue-off': (facts) => !facts.autoContinue,
  'code-tab-auto-continue-off': (facts) =>
    CODE_TAB[facts.session] &&
    facts.codeTabAutoContinue === false &&
    facts.managedAutoContinue !== true,
  'reset-too-far': (facts) => facts.msUntilReset > DAY_MS,
  'waited-twice': (facts) => facts.waitsSoFar >= MAX_USAGE_WAITS,
  'old-release': (facts) => !facts.releaseHasWait,
};

/**
 * Whether the run waits for the reset or the agent that hit the limit fails. Asked again during a
 * wait with the current facts: "If you turn [`autoContinueAtUsageLimit`] off during a wait, the wait
 * ends and the waiting agents fail."
 */
export function usageLimitDecision(facts: UsageLimitFacts): UsageLimitDecision {
  const causes = NO_WAIT_CAUSES.filter((cause) => CONDITIONS[cause](facts));
  if (causes.length > 0) return { pauses: false, failsAgent: true, causes };
  return { pauses: true, msUntilReset: facts.msUntilReset };
}
