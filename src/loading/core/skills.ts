/**
 * A core package's skills (core §7.1): each immediate child of `skills/` holding a path named exactly
 * `SKILL.md` that resolves to a regular file is one skill; nothing deeper is searched. A `SKILL.md`
 * outside the plugin root is skipped (core §4.1, boundary 3), and so is one that fails the Agent
 * Skills specification; the others load. A skill is named by its directory, offered as
 * `<package>:<directory>` (Agent Plugins Extended §18.1.5 item 5).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/skills.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { LocateContext } from '../../format/locate.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginTextFile } from '../plugin-model.ts';

const call = coreModule('loading/core/skills');

export interface CoreSkillsRead {
  readonly skills: readonly PluginTextFile[];
  readonly issues: readonly PluginIssue[];
}

/** The skills of `skills/`, in name order. */
export const readCoreSkills = (
  context: LocateContext,
  pluginName: string,
): Promise<CoreSkillsRead> => call.async('readCoreSkills', context, pluginName);
