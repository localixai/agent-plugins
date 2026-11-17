import { checkCoreSkill } from '../../format/core/skill.ts';
import type { PluginTextFile } from '../../loading/plugin-model.ts';
import { notConverted, reported } from '../notes.ts';
import { expandedFeatures } from '../skill-body.ts';
import type { ConversionNote } from '../types.ts';

/**
 * A Claude Code plugin's skills in a core client (Agent Plugins Extended §18.4.3 item 2). `skills/`
 * is shared, so nothing is written: a skill converts when it is `skills/<name>/SKILL.md` and conforms
 * to Agent Skills (core §7.1). A skill elsewhere (a manifest `skills` path, a root `SKILL.md`) is
 * reported as not converted; so is one a core client would skip, which the converter must never
 * offer as converted. A skill's Claude Code fields and its §4.8 body features are reported: a core
 * client ignores the fields and shows the text as written. Judgment (spec/judgments.md): skills are
 * not copied under `skills/` and fields are not stripped, since either would rewrite the source form.
 */

const SKILLS = 'skills/';

function skillNotes(skill: PluginTextFile): ConversionNote[] {
  const path = skill.path ?? `${skill.directory ?? skill.name}/SKILL.md`;
  const home = `${SKILLS}${skill.name}`;
  if (skill.alone === true || skill.directory !== home || skill.path !== `${home}/SKILL.md`) {
    return [
      notConverted(
        path,
        undefined,
        `skill "${skill.name}" is not under skills/, the only place a core client reads`,
      ),
    ];
  }
  const check = checkCoreSkill(skill.text, skill.name);
  if (check.problems.length > 0) {
    return [
      notConverted(
        path,
        undefined,
        `a core client skips skill "${skill.name}": ${check.problems.join('; ')}`,
      ),
    ];
  }
  const features = expandedFeatures(skill.text);
  return [
    ...(check.unknownFields.length === 0
      ? []
      : [
          reported(
            path,
            check.unknownFields.join(', '),
            `skill "${skill.name}": a core client ignores ${check.unknownFields.join(', ')}, outside the six Agent Skills fields`,
          ),
        ]),
    ...(features.length === 0
      ? []
      : [
          reported(
            path,
            undefined,
            `skill "${skill.name}" holds ${features.join('; ')}, which a core client shows as written`,
          ),
        ]),
  ];
}

export const coreSkillNotes = (skills: readonly PluginTextFile[]): ConversionNote[] =>
  skills.flatMap(skillNotes);
