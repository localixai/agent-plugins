/**
 * Whether a call a user's mod approved in auto mode skips the classifier (Agent Plugins Extended
 * §14.9.2 item 4, §13.11 item 2; permissions.md of 2.1.330, "Precedence among rules, hooks, and
 * mods"): only without the built-in guard. Where the guard loads, a mod's approval skips neither
 * the classifier nor what `dontAsk` denies, and `allowModsToOverrideDenyRules` does not bring it
 * (Judgment J14.14). Before 2.1.330 the guard kept the classifier only in front of a prompt tool or
 * a `canUseTool` callback.
 */
export interface ModApprovalFacts {
  /** The built-in guard is loaded in this session. */
  readonly guardLoaded: boolean;
}

/** True when the approved call runs without a classifier check. */
export const modApprovalSkipsClassifier = (facts: ModApprovalFacts): boolean => !facts.guardLoaded;
