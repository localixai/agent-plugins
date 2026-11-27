/**
 * A core package's skill as a plugin skill (Agent Plugins Extended §18.1.5 item 5, J18.14): the
 * command `/<package>:<directory>`, listed and invocable by the model and the user. Only the six
 * Agent Skills fields have an effect (`name`, `description`, `license`, `compatibility`, `metadata`,
 * `allowed-tools`): every Claude Code field the file may also carry (`model`, `hooks`, `context`,
 * `disable-model-invocation`, `paths`, `shell`, `arguments`...) takes its default, so a core skill
 * acts the same in every core client. The loader already checked the file against Agent Skills
 * (core §7.1), so the Claude Code reader's findings on other fields are not the package's issues.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/core_skills.rs`, docs/RUST.md).
 */
import type { Skill } from '../format/skill-types.ts';
import type { PluginTextFile } from '../loading/index.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('host/core-skills');

export const coreSkillOf = (pluginName: string, file: PluginTextFile): Skill =>
  call('coreSkillOf', pluginName, file);
