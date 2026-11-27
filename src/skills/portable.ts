import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Which frontmatter fields each distribution path takes (skills.md, "Using skill frontmatter outside
 * Claude Code"): "Claude Code accepts every field in the table above. Outside Claude Code, you can use
 * only the fields in the Agent Skills spec", and a field the spec doesn't allow makes packaging or
 * upload fail "with a hard error instead of ignoring the field". The kit reads the file; the host (a
 * packaging step, an uploader) shows the error.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/portable.rs`, docs/RUST.md).
 */

const call = coreModule('skills/portable');

/** The six fields of the Agent Skills spec: "name, description, license, compatibility, metadata, allowed-tools". */
export const AGENT_SKILLS_FIELDS: readonly [
  'name',
  'description',
  'license',
  'compatibility',
  'metadata',
  'allowed-tools',
] = call('AGENT_SKILLS_FIELDS');

export const DISTRIBUTION_PATHS: readonly [
  'claude-code',
  'claude-ai-upload',
  'skills-api',
  'package-skill',
] = call('DISTRIBUTION_PATHS');
export type DistributionPath = (typeof DISTRIBUTION_PATHS)[number];

/**
 * The hard error of an upload or packaging for a `SKILL.md`, or none. The message is the docs':
 * `Unexpected key(s) in SKILL.md frontmatter: argument-hint. Allowed properties are: allowed-tools,
 * compatibility, description, license, metadata, name`. Judgment (spec/judgments.md): several
 * unexpected keys are listed in the order they appear, comma separated.
 */
export const frontmatterIssues = (text: string, path: DistributionPath): readonly PluginIssue[] =>
  call('frontmatterIssues', text, path);
