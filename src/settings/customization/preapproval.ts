/** Implemented by the Rust core (`crates/agent-plugins/src/settings/customization/preapproval.rs`, docs/RUST.md). */

/** Where a skill comes from, as far as `allowed-tools` pre-approval is concerned. */
export type PreapprovalSource =
  | 'user'
  | 'project'
  | 'addDir'
  | 'synced'
  | 'plugin'
  | 'managed'
  | 'bundled';

/** What the host knows about the skill that is about to run. */
export interface PreapprovalSkill {
  readonly source: PreapprovalSource;
  /** The skill's frontmatter declares `hooks`. */
  readonly declaresHooks: boolean;
  /** The skill runs in a subagent through `context: fork`. */
  readonly contextFork: boolean;
}
