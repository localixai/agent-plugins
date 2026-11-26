import { coreModule } from '../wasm/bridge.ts';

/**
 * How Claude Code clones with the credentials already on the machine (plugins/install.md, "Add a
 * private marketplace"; env-vars.md, `CLAUDE_CODE_PLUGIN_PREFER_HTTPS`): pure, the host runs git. The
 * variable names that switch git's prompts off are the kit's choice (spec/judgments.md); the docs say
 * only that "Git's prompts and askpass programs are off, including one set in `GIT_ASKPASS`".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/git_transport.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/git-transport');

export type CloneTransport = 'https' | 'ssh';

/** Facts about the machine the host reads. */
export interface ShorthandFacts {
  /** `CLAUDE_CODE_PLUGIN_PREFER_HTTPS=1`: "skip that check and always clone over HTTPS". */
  readonly preferHttps: boolean;
  /** Whether the user's SSH key authenticates to `github.com`; consulted only without `preferHttps`. */
  readonly sshAuthenticates: () => boolean;
}

/**
 * A GitHub `owner/repo` shorthand: "Claude Code checks whether your SSH key authenticates to
 * `github.com`, then clones over SSH if it does and over HTTPS if it doesn't"; `PREFER_HTTPS` skips
 * the check.
 */
export const shorthandTransport = (facts: ShorthandFacts): CloneTransport =>
  call('shorthandTransport', facts);

/** Environment settings that make git fail instead of asking. They replace what the user's environment holds. */
export const NO_PROMPT_ENVIRONMENT: Readonly<Record<string, string>> =
  call('NO_PROMPT_ENVIRONMENT');

/** Where the user's git setup names an SSH program; the host reads these three. */
export interface SshProgramFacts {
  readonly gitSshCommand?: string;
  readonly gitSsh?: string;
  /** `core.sshCommand` of the git configuration. */
  readonly coreSshCommand?: string;
}

export interface ClonePlan {
  /** Set on top of the inherited environment. */
  readonly environment: Readonly<Record<string, string>>;
  /** Runs the program the user's git setup names; otherwise `ssh` with `BatchMode`, which never asks for a passphrase. */
  readonly sshProgram: 'user-configured' | 'ssh-batch-mode';
  /**
   * The options the client's own `ssh` runs with: `BatchMode=yes` and `StrictHostKeyChecking=yes`,
   * so a host not already in `known_hosts` is refused, never accepted (plugins/troubleshooting.md,
   * "SSH host key is not in your known_hosts file"). Empty for a user-named program, whose command
   * line is not rewritten (J15.14).
   */
  readonly sshOptions: readonly string[];
}

/**
 * The environment and SSH program of a clone, for an HTTPS credential helper (prompts off), an SSH
 * key that "must work without a passphrase prompt", and "if your git setup names an SSH program ...
 * Claude Code runs that program".
 */
export const clonePlan = (ssh: SshProgramFacts = {}): ClonePlan => call('clonePlan', ssh);

/**
 * The transports tried in order for a GitHub `owner/repo` (plugins/host-marketplace.md, "Grant access
 * to a private marketplace"): `ssh -T git@github.com` first, SSH when it succeeds; "if the probe
 * fails, or the SSH clone itself fails, it clones over HTTPS". `CLAUDE_CODE_PLUGIN_PREFER_HTTPS=1` skips
 * the probe.
 */
export const shorthandAttempts = (facts: ShorthandFacts): readonly CloneTransport[] =>
  call('shorthandAttempts', facts);
