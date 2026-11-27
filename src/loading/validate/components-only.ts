/**
 * A folder with no manifest still validates when it holds components Claude Code loads without one
 * (oracle 2.1.319): a skills tree (`skills/<name>/SKILL.md`), a project's `.claude/skills`,
 * `.claude/commands` or `.claude/agents`, or a folder that is itself named `skills`, `commands` or
 * `agents`. Anything else has no manifest to find.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/components_only.rs`, docs/RUST.md).
 */
import type { ValidationIssue } from '../../format/validate/finding.ts';
import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/validate/components-only');

export interface BareInput {
  readonly files: PluginFiles;
  readonly label: string;
  readonly signal?: AbortSignal;
}

/** The components of a folder with no manifest, or `undefined` when it holds none the validator knows. */
export const componentsOnly = (input: BareInput): Promise<ValidationIssue[] | undefined> =>
  call.async('componentsOnly', input);
