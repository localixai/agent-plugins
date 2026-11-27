/**
 * Whether the signals of a `relevance` block match the session (plugins/relevance.md, "Understand
 * how plugin relevance works", "`relevance.signals`", "Working directory matching", "Command name
 * matching", "Manifest dependency matching"): pure, over the facts the host collected. Matching is
 * local; nothing here reports which signal matched. What the docs leave open (the glob dialect, an
 * invalid regular expression, a compound command's separators) is in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/relevance_match.rs`,
 * docs/RUST.md).
 */
import type { RelevanceSignals } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';

export type SignalName = 'cwd' | 'cli' | 'hosts' | 'filesRead' | 'manifestDeps';

export interface ManifestFact {
  readonly path: string;
  readonly content: string;
  /** Byte size when known; the length of `content` otherwise. */
  readonly size?: number;
}

export interface SessionFacts {
  /** The session's working directory, absolute. */
  readonly cwd: string;
  /** The root of the git repository the session is inside, when it is. */
  readonly repositoryRoot?: string;
  /** The shell commands Claude ran this session. */
  readonly commands: readonly string[];
  /** Paths Claude read, wrote or edited this session, and the project's auto-loaded `CLAUDE.md` files. */
  readonly filesTouched: readonly string[];
  /** The package manifests Claude read, written or edited, and their contents. */
  readonly manifests: readonly ManifestFact[];
}

const call = coreModule('marketplace/relevance-match');

/** "Manifest files larger than 512 KB are skipped". */
export const MANIFEST_SIZE_LIMIT: number = call('MANIFEST_SIZE_LIMIT');

/** "Forward-slash normalized and case-insensitive." */
export const relevanceGlobMatches = (glob: string, path: string): boolean =>
  call('relevanceGlobMatches', glob, path);

/**
 * A `cwd` pattern: matched against the absolute working directory and, inside a repository, against
 * its path relative to the root; every pattern "matches the directory itself and everything under it".
 */
export const cwdMatches = (pattern: string, cwd: string, repositoryRoot?: string): boolean =>
  call('cwdMatches', pattern, cwd, repositoryRoot);

/**
 * "One command name for each shell command Claude runs: the first token after any leading environment
 * variable assignments and `sudo`. Compound commands contribute only their leading command", so
 * `cd infra && terraform plan` records `cd`.
 */
export const commandName = (command: string): string | undefined => call('commandName', command);

/** The hostnames of the `http://` and `https://` URLs in a Bash command, lowercased. */
export const commandHosts = (command: string): readonly string[] => call('commandHosts', command);

/** The signals that match, in the docs' order. Empty when none is set: "only if at least one signal is set". */
export const matchingSignals = (
  signals: RelevanceSignals | undefined,
  facts: SessionFacts,
): readonly SignalName[] => call('matchingSignals', signals, facts);

/** `cwd` is "the only signal that can match at session start, before the user sends a first message". */
export const matchesAtSessionStart = (
  signals: RelevanceSignals | undefined,
  cwd: string,
  repositoryRoot?: string,
): boolean => call('matchesAtSessionStart', signals, cwd, repositoryRoot);
