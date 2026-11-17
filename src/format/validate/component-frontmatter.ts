import { isRecord } from '../../protocol/is-record.ts';
import { parseFrontmatter, type Frontmatter, type FrontmatterState } from '../frontmatter.ts';
import { errorAt, warningAt, type Finding } from './finding.ts';
import { kindOf } from './received.ts';

/**
 * The frontmatter checks of `claude plugin validate` on a skill, agent or command file (oracle
 * 2.1.319, 2.1.321 corpus and 2.1.325; spec §4.10, §5.4.4): no block, a block that does not parse, a block that is not a mapping, a missing
 * description and the few fields whose type it reads. Anything else in the block is left alone.
 */
export const COMPONENT_KINDS = ['skill', 'agent', 'command'] as const;
export type ComponentKind = (typeof COMPONENT_KINDS)[number];

const EMPTY_METADATA = 'empty metadata (all frontmatter fields silently dropped).';
/** What a block that does not parse costs at runtime, by kind. */
const WHEN_UNPARSED: { readonly [K in ComponentKind]: string } = {
  skill: EMPTY_METADATA,
  agent: 'its name taken from the filename and every other frontmatter field silently dropped.',
  command: EMPTY_METADATA,
};

/**
 * What a broken block costs an agent of a project's `.claude/agents/` (spec §2.11 table, J2.4,
 * oracle 2.1.319 and 2.1.322): it does not load, where a plugin's agent loads under its file name.
 */
const PROJECT_AGENT_UNPARSED =
  'does not load at all — with no frontmatter name it is treated as a co-located reference document and skipped.';

/** Where the component file sits: a plugin's folder, or a project's `.claude/` folder. */
export type ComponentPlace = 'plugin' | 'project';

const IN_PLUGIN: { readonly [K in ComponentKind]: string } = {
  skill: `loads with ${WHEN_UNPARSED.skill}`,
  agent: `loads with ${WHEN_UNPARSED.agent}`,
  command: `loads with ${WHEN_UNPARSED.command}`,
};

/** What a broken block costs, by place and kind. */
const UNPARSED_COST: {
  readonly [P in ComponentPlace]: { readonly [K in ComponentKind]: string };
} = {
  plugin: IN_PLUGIN,
  project: { ...IN_PLUGIN, agent: PROJECT_AGENT_UNPARSED },
};

const NO_BLOCK =
  'No frontmatter block found. Add YAML frontmatter between --- delimiters at the top of the file to set description and other metadata.';

/**
 * The parser's complaint, in the validator's words. The validator's YAML parser names two causes;
 * this one (`yaml`) names more, so only an unterminated quote is told apart (spec/judgments.md).
 */
const yamlComplaint = (problem: string | undefined): string =>
  /missing closing/i.test(problem ?? '') ? 'Unexpected EOF' : 'Unexpected token';

const sentence = (value: unknown): string => (Array.isArray(value) ? 'an array' : kindOf(value));

/** `name` and `description` name a list `object` like a map, `description` names it `array`. */
const nameKind = (value: unknown): string => (Array.isArray(value) ? 'object' : kindOf(value));

type Check = (fields: Readonly<Record<string, unknown>>, kind: ComponentKind) => Finding[];

const nameCheck: Check = (fields) => {
  const name = present(fields.name);
  return name === undefined || typeof name === 'string'
    ? []
    : [errorAt('name', `name must be a string, got ${nameKind(name)}.`)];
};

/** A list or a map; a number or a boolean is read as text and not reported. */
const descriptionCheck: Check = (fields, kind) => {
  const { description } = fields;
  if (description === undefined) {
    return [
      warningAt(
        'description',
        `No description in frontmatter. A description helps users and Claude understand when to use this ${kind}.`,
      ),
    ];
  }
  return typeof description === 'object' && description !== null
    ? [
        errorAt(
          'description',
          `description must be a string, got ${kindOf(description)}. At runtime this value is dropped.`,
        ),
      ]
    : [];
};

const toolsCheck: Check = (fields) => {
  const tools = present(fields['allowed-tools']);
  if (tools === undefined || typeof tools === 'string') return [];
  if (!Array.isArray(tools)) {
    return [
      errorAt(
        'allowed-tools',
        `allowed-tools must be a string or array of strings, got ${kindOf(tools)}.`,
      ),
    ];
  }
  return tools.every((entry) => typeof entry === 'string')
    ? []
    : [errorAt('allowed-tools', 'allowed-tools array must contain only strings.')];
};

const SHELLS: ReadonlySet<string> = new Set(['bash', 'powershell']);

/** `shell`: text naming one of the two shells, in any letter case (`Bash` passes). */
const shellCheck: Check = (fields) => {
  const shell = present(fields.shell);
  if (shell === undefined) return [];
  if (typeof shell !== 'string') {
    return [errorAt('shell', `shell must be a string, got ${kindOf(shell)}.`)];
  }
  return SHELLS.has(shell.toLowerCase())
    ? []
    : [errorAt('shell', `shell must be 'bash' or 'powershell', got '${shell}'.`)];
};

const metadataCheck: Check = (fields) => {
  const metadata = present(fields.metadata);
  return metadata === undefined || isRecord(metadata)
    ? []
    : [
        warningAt(
          'metadata',
          `'metadata' must be a mapping (key: value pairs); got ${kindOf(metadata)}. It is dropped at load time.`,
        ),
      ];
};

/**
 * The fields whose type the validator reads, per kind, in its order (spec §4.10 for skills and
 * commands, §5.4.4 for agents: `name` and `description` only).
 */
const CHECKS: { readonly [K in ComponentKind]: readonly Check[] } = {
  skill: [nameCheck, toolsCheck, shellCheck, metadataCheck, descriptionCheck],
  command: [nameCheck, toolsCheck, shellCheck, metadataCheck, descriptionCheck],
  agent: [nameCheck, descriptionCheck],
};

const fieldFindings = (kind: ComponentKind, fields: Readonly<Record<string, unknown>>): Finding[] =>
  CHECKS[kind].flatMap((check) => check(fields, kind));

/** A key written with no value (`allowed-tools:`) is null: the validator treats it as absent. */
const present = (value: unknown): unknown => (value === null ? undefined : value);

/** What each state of the block gives: the missing block is advised on, a broken one is an error. */
const BLOCKS: {
  readonly [S in FrontmatterState]: (
    block: Frontmatter,
    kind: ComponentKind,
    place: ComponentPlace,
  ) => Finding[];
} = {
  none: () => [warningAt('frontmatter', NO_BLOCK)],
  ok: (block, kind) => fieldFindings(kind, block.fields),
  'invalid-yaml': (block, kind, place) => [
    errorAt(
      'frontmatter',
      `YAML frontmatter failed to parse: YAML Parse error: ${yamlComplaint(block.problem)}. At runtime this ${kind} ${UNPARSED_COST[place][kind]}`,
    ),
  ],
  'not-a-table': (block) => [
    errorAt(
      'frontmatter',
      `Frontmatter must be a YAML mapping (key: value pairs), got ${sentence(block.value)}.`,
    ),
  ],
};

/** Findings of one component file's text, with the paths the oracle gives them. */
export function frontmatterFindings(
  kind: ComponentKind,
  text: string,
  place: ComponentPlace = 'plugin',
): Finding[] {
  const block = parseFrontmatter(text);
  return BLOCKS[block.state](block, kind, place);
}
