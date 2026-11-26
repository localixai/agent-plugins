import type { Environment } from '../ports/environment.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Background auto-update of a private marketplace (plugins/host-marketplace.md, "What background
 * auto-update does with credentials"; plugins/install.md, "Auto-update from a private
 * marketplace"). The check "uses the user's configured git credential helpers and never prompts".
 * Pure: the host runs git; the kit says whether the check can authenticate and what follows it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/updates/private_marketplace.rs`,
 * docs/RUST.md).
 */
const call = coreModule('updates/private-marketplace');

/** What authenticates a remote in the background. */
export type Credentials = (
  | { readonly remote: 'ssh'; readonly keyInAgent: boolean }
  | {
      readonly remote: 'https';
      /** `supplies-stored-credential`: Git Credential Manager, the macOS Keychain helper or `git-credential-store` holding a credential; `must-prompt` cannot answer unattended. */
      readonly helper: 'supplies-stored-credential' | 'must-prompt' | 'none';
    }
) & {
  /** `GITHUB_TOKEN`, `GH_TOKEN` or another provider token is set in the environment. It never counts toward authenticating. */
  readonly providerTokenInEnvironment?: boolean;
};

/**
 * Whether the unattended check authenticates. "If a user sets `GITHUB_TOKEN` or another provider
 * token in the environment, that alone doesn't authenticate the background check": the function
 * reads only the key in the agent and the helper, so a token in the environment cannot count. An SSH remote with no
 * key in the agent and an HTTPS remote with no helper are judgments (spec/judgments.md).
 */
export const backgroundCheckAuthenticates = (credentials: Credentials): boolean =>
  call('backgroundCheckAuthenticates', credentials);

/** What the host sees of the remote when the check runs. */
export interface RemoteState {
  /** The remote answers at all. */
  readonly reachable: boolean;
  /** Its default branch has commits the checkout lacks; only meaningful once the check authenticated. */
  readonly hasNewCommits: boolean;
}

/** What the check found. */
export type CheckResult = 'up-to-date' | 'new-commits' | 'cannot-reach-or-authenticate';

export type CheckFollowUp = 'leave' | 'reclone' | 'keep-existing-checkout';

/**
 * What the unattended check finds: a remote it cannot reach, or cannot authenticate to (a helper
 * that must prompt "fails quietly"), is `cannot-reach-or-authenticate`; otherwise whether the remote
 * has new commits.
 */
export const backgroundCheckResult = (credentials: Credentials, remote: RemoteState): CheckResult =>
  call('backgroundCheckResult', credentials, remote);

/**
 * "After the check, Claude Code does one of the following." `checkoutHoldsCatalog` is whether the
 * existing checkout contains `.claude-plugin/marketplace.json`: the variable "skips the re-clone
 * only for a checkout that already contains" it, so a marketplace never cloned, or whose clone
 * stopped partway, still gets the clone attempt (plugins/troubleshooting.md, "Marketplace updates
 * keep failing offline").
 */
export const afterBackgroundCheck = (
  result: CheckResult,
  env: Environment,
  checkoutHoldsCatalog: boolean,
): CheckFollowUp => call('afterBackgroundCheck', result, env, checkoutHoldsCatalog);

/** "If that clone fails, the existing checkout stays in place." */
export const afterReclone = (
  cloneSucceeded: boolean,
): 'replace-checkout' | 'keep-existing-checkout' => call('afterReclone', cloneSucceeded);
