/**
 * The paths a manifest names that the validator then looks for on disk (oracle 2.1.319). A path
 * with a `..` segment is refused outright; the rest are claims the caller checks against the files:
 * `Path not found`, or for `skills` a file where a directory is due.
 *
 * Types only: the core runs it (`crates/agent-plugins/src/format/validate/path_claims.rs`,
 * docs/RUST.md), and only the core calls it now.
 */

export interface PathClaim {
  /** Where the oracle puts the report: `skills[0]`, `commands.a.source`, `mcpServers`. */
  readonly path: string;
  readonly value: string;
  /** The entry must be a directory holding `SKILL.md`. */
  readonly directory: boolean;
  /** The loader skips a missing file here without reporting it (an MCP servers file). */
  readonly skippedWhenMissing: boolean;
}
