import { readSkill } from '../format/skill.ts';
import type { InvocableFrontmatter, Skill } from '../format/skill-types.ts';
import type { PluginTextFile } from '../loading/index.ts';

/**
 * A core package's skill as a plugin skill (Agent Plugins Extended §18.1.5 item 5, J18.14): the
 * command `/<package>:<directory>`, listed and invocable by the model and the user. Only the six
 * Agent Skills fields have an effect (`name`, `description`, `license`, `compatibility`, `metadata`,
 * `allowed-tools`): every Claude Code field the file may also carry (`model`, `hooks`, `context`,
 * `disable-model-invocation`, `paths`, `shell`, `arguments`...) takes its default, so a core skill
 * acts the same in every core client. The loader already checked the file against Agent Skills
 * (core §7.1), so the Claude Code reader's findings on other fields are not the package's issues.
 */

/** What every field outside Agent Skills is in a core skill: Claude Code's value for an unset field. */
const CLAUDE_CODE_DEFAULTS: Omit<
  InvocableFrontmatter,
  'description' | 'allowedTools' | 'metadata' | 'license' | 'compatibility' | 'frontmatter'
> = {
  arguments: [],
  disableModelInvocation: false,
  userInvocable: true,
  disallowedTools: [],
  background: false,
  paths: [],
  shell: 'bash',
};

export function coreSkillOf(pluginName: string, file: PluginTextFile): Skill {
  const read = readSkill(file.path ?? `skills/${file.name}/SKILL.md`, file.text, {
    origin: { kind: 'plugin', plugin: pluginName, core: true },
    dirName: file.name,
    fixedName: true,
  });
  const { skill } = read;
  return {
    kind: 'skill',
    name: skill.name,
    command: skill.command,
    path: skill.path,
    dir: skill.dir,
    origin: skill.origin,
    body: skill.body,
    frontmatter: skill.frontmatter,
    allowedTools: skill.allowedTools,
    ...CLAUDE_CODE_DEFAULTS,
    ...(skill.description === undefined ? {} : { description: skill.description }),
    ...(skill.metadata === undefined ? {} : { metadata: skill.metadata }),
    ...(skill.license === undefined ? {} : { license: skill.license }),
    ...(skill.compatibility === undefined ? {} : { compatibility: skill.compatibility }),
  };
}
