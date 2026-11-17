import type { PluginTextFile } from '../../loading/plugin-model.ts';
import { reported } from '../notes.ts';
import { expandedFeatures } from '../skill-body.ts';
import type { ConversionNote } from '../types.ts';

/**
 * A core package's skills in Claude Code (Agent Plugins Extended §18.4.2 item 2): `skills/` is used
 * as it is, nothing is written; a skill whose body holds text Claude Code expands or runs is
 * reported, since Claude Code will replace or run what a core client shows as written.
 */
export function claudeCodeSkillNotes(skills: readonly PluginTextFile[]): ConversionNote[] {
  return skills.flatMap((skill) => {
    const features = expandedFeatures(skill.text);
    const path = skill.path ?? `skills/${skill.name}/SKILL.md`;
    return features.length === 0
      ? []
      : [
          reported(
            path,
            undefined,
            `skill "${skill.name}" holds ${features.join('; ')}, which Claude Code expands or runs and a core client shows as written`,
          ),
        ];
  });
}
