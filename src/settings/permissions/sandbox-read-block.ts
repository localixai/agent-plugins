/** Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/sandbox_read_block.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/permissions/sandbox-read-block');

/**
 * What `permissions.blockReadsOutsideWorkingDirectories` does to sandboxed commands (settings-reference.md,
 * "Sandboxed commands under the block"). Pure over the facts the host reads; the sandbox is the host's.
 */
/** "Your home directory and ... the other roots that hold user files." */
export const BLOCKED_USER_ROOTS: readonly string[] = call('BLOCKED_USER_ROOTS');

export interface SandboxBlockInput {
  /** The block key is `true` in some file. */
  readonly blockReads: boolean;
  /** `sandbox.filesystem.disabled`. */
  readonly filesystemDisabled: boolean;
  /** `allowManagedReadPathsOnly` is set. */
  readonly managedReadPathsOnly: boolean;
  /** "The path of the directory you started Claude Code in contains a glob character". */
  readonly startDirectory: string;
  /** The home directory of the user. */
  readonly home: string;
  /**
   * What the block re-opens: "the working directories, worktrees Claude Code creates in the
   * session, the session temp directory, and the parts of `~/.claude` that commands need". The host
   * knows each; the kit returns them as `reopened`.
   */
  readonly reopened?: readonly string[];
  /**
   * The repository's common `.git` directory when the session's working directory is a linked git
   * worktree, "including one Claude Code entered mid-session"; absent otherwise.
   */
  readonly worktreeCommonGitDir?: string;
}

export interface SandboxBlock {
  /** Whether the block reaches sandboxed commands; Claude's file tools enforce it either way. */
  readonly reachesSandbox: boolean;
  /** Roots a sandboxed command may not read. */
  readonly denyRead: readonly string[];
  /** What a sandboxed command may read again below the denied roots. */
  readonly reopened: readonly string[];
  /** "The repository's common `.git` directory stays readable and writable to sandboxed commands." */
  readonly readWrite: readonly string[];
  /**
   * What stays write-denied inside `readWrite`: "Writes to `hooks/` and `config` inside that
   * directory remain denied" (sandboxing.md, "Filesystem isolation").
   */
  readonly denyWrite: readonly string[];
  /** "`allowRead` and `allowWrite` entries from repository settings don't count" while the block is in force. */
  readonly repositoryAllowEntriesCount: boolean;
}

/** The block's effect on sandboxed commands: it reaches them unless one of the three cases the docs list holds. */
export const sandboxReadBlock = (input: SandboxBlockInput): SandboxBlock =>
  call('sandboxReadBlock', input);

/** "Under the block, Claude Code re-opens your global git configuration files": the paths it names. */
export const gitConfigReopened = (input: {
  readonly home: string;
  readonly xdgConfigHome?: string;
  readonly included: readonly string[];
  /** Files a sandboxed command can write, directly or through a symlink: not re-opened. */
  readonly writable: ReadonlySet<string>;
  /** `sandbox.filesystem.denyRead` entries: they "always take precedence over this re-open". */
  readonly denyRead: readonly string[];
}): readonly string[] => call('gitConfigReopened', input);
