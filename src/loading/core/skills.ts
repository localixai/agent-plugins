import { checkCoreSkill } from '../../format/core/skill.ts';
import { error, warning, type PluginIssue } from '../../format/issues.ts';
import { listSorted } from '../../format/listing.ts';
import { locate, type LocateContext } from '../../format/locate.ts';
import type { DirectoryEntry } from '../../ports/index.ts';
import type { PluginTextFile } from '../plugin-model.ts';
import { fixedLocation } from './locations.ts';

/**
 * A core package's skills (core §7.1): each immediate child of `skills/` holding a path named exactly
 * `SKILL.md` that resolves to a regular file is one skill; nothing deeper is searched. A `SKILL.md`
 * outside the plugin root is skipped (core §4.1, boundary 3), and so is one that fails the Agent
 * Skills specification; the others load. A skill is named by its directory, offered as
 * `<package>:<directory>` (Agent Plugins Extended §18.1.5 item 5).
 */

export interface CoreSkillsRead {
  readonly skills: readonly PluginTextFile[];
  readonly issues: readonly PluginIssue[];
}

const DIRECTORY = 'skills';

/** One child of `skills/`: its skill, the issues it gives, or nothing when it holds no `SKILL.md`. */
async function childSkill(
  context: LocateContext,
  entry: DirectoryEntry,
  pluginName: string,
): Promise<CoreSkillsRead> {
  const path = `${DIRECTORY}/${entry.name}/SKILL.md`;
  const found = await locate(context, DIRECTORY, path, true);
  if (found.issue) return { skills: [], issues: [{ ...found.issue, path }] };
  if (found.stat?.isFile !== true) return { skills: [], issues: [] };
  const text = (await context.files.readText(path, context.signal)) ?? '';
  const check = checkCoreSkill(text, entry.name);
  const extra = check.unknownFields.map((field) =>
    warning(path, `Field "${field}" is not part of the Agent Skills specification`),
  );
  if (check.problems.length > 0) {
    return {
      skills: [],
      issues: [
        error(path, `Skill "${entry.name}" is skipped: ${check.problems.join('; ')}`),
        ...extra,
      ],
    };
  }
  const skill: PluginTextFile = {
    path,
    name: entry.name,
    qualifiedName: `${pluginName}:${entry.name}`,
    text,
    directory: `${DIRECTORY}/${entry.name}`,
  };
  return { skills: [skill], issues: extra };
}

/** The skills of `skills/`, in name order. */
export async function readCoreSkills(
  context: LocateContext,
  pluginName: string,
): Promise<CoreSkillsRead> {
  const where = await fixedLocation(context, DIRECTORY, 'directory');
  if ('absent' in where) return { skills: [], issues: [] };
  if ('issue' in where) return { skills: [], issues: [where.issue] };
  const entries = (await listSorted(context.files, DIRECTORY, context.signal)) ?? [];
  const reads = await Promise.all(
    entries.filter((entry) => !entry.isFile).map((entry) => childSkill(context, entry, pluginName)),
  );
  return {
    skills: reads.flatMap((read) => read.skills),
    issues: reads.flatMap((read) => read.issues),
  };
}
