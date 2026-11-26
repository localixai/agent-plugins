/**
 * A Claude Code plugin's skills in a core client (Agent Plugins Extended §18.4.3 item 2). `skills/`
 * is shared, so nothing is written: a skill converts when it is `skills/<name>/SKILL.md` and conforms
 * to Agent Skills (core §7.1). A skill elsewhere (a manifest `skills` path, a root `SKILL.md`) is
 * reported as not converted; so is one a core client would skip, which the converter must never
 * offer as converted. A skill's Claude Code fields and its §4.8 body features are reported: a core
 * client ignores the fields and shows the text as written. Judgment (spec/judgments.md): skills are
 * not copied under `skills/` and fields are not stripped, since either would rewrite the source form.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_core/skills.rs`, docs/RUST.md).
 */
import type { PluginTextFile } from '../../loading/plugin-model.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ConversionNote } from '../types.ts';

const call = coreModule('convert/to-core/skills');

export const coreSkillNotes = (skills: readonly PluginTextFile[]): ConversionNote[] =>
  call('coreSkillNotes', skills);
