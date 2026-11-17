/**
 * The worked example of Agent Plugins Extended §18.3.5, `release-guard`, rebuilt as file texts: the
 * core half (`plugin.json`, `mcp.json`, `hooks.json`), the Claude Code half
 * (`.claude-plugin/plugin.json`, `.mcp.json`, `hooks/hooks.json`) and the files both share (the
 * skill, the handler, a stub server). The conversion tests turn one half into the other.
 */
import { convertPluginPackage, type Conversion } from '../../src/entry/core.ts';
import { memoryFiles } from './ports.ts';

const schema = (document: string) =>
  `https://agent-plugins.org/schemas/1.1.0/${document}.schema.json`;

const pretty = (value: unknown): string => `${JSON.stringify(value, null, 2)}\n`;

const MANIFEST = {
  name: 'release-guard',
  displayName: 'Release Guard',
  version: '1.0.0',
  description: 'Release notes from the changelog, and no publishing by the agent.',
  author: { name: 'Example Team', url: 'https://example.com' },
  homepage: 'https://example.com/release-guard',
  license: 'MIT',
  keywords: ['release', 'changelog'],
};

export const CORE_MANIFEST = { $schema: schema('plugin'), ...MANIFEST };
export const CLAUDE_MANIFEST = MANIFEST;

export const CORE_MCP = {
  $schema: schema('mcp'),
  mcpServers: {
    notes: {
      type: 'stdio',
      command: 'node',
      args: ['${PLUGIN_ROOT}/server/notes.mjs', '--state', '${PLUGIN_DATA}/notes'],
    },
  },
};

export const CLAUDE_MCP = {
  mcpServers: {
    notes: {
      type: 'stdio',
      command: 'node',
      args: ['${CLAUDE_PLUGIN_ROOT}/server/notes.mjs', '--state', '${CLAUDE_PLUGIN_DATA}/notes'],
    },
  },
};

export const CORE_HOOKS = {
  $schema: schema('hooks'),
  hooks: {
    PreToolUse: [
      { command: 'node', args: ['${PLUGIN_ROOT}/hooks/check-publish.mjs', '--form', 'core'] },
    ],
  },
};

export const CLAUDE_HOOKS = {
  hooks: {
    PreToolUse: [
      {
        matcher: 'Bash|PowerShell',
        hooks: [
          {
            type: 'command',
            command: 'node',
            args: ['${CLAUDE_PLUGIN_ROOT}/hooks/check-publish.mjs', '--form', 'claude-code'],
          },
        ],
      },
    ],
  },
};

const SKILL = [
  '---',
  'name: release-notes',
  'description: Drafts release notes from CHANGELOG.md for the version the user names. Use when asked for release notes or a release summary.',
  '---',
  '',
  'Read `CHANGELOG.md` at the repository root. Take the section of the requested version, group its',
  'entries under Added, Changed and Fixed, and write the notes in plain prose. Use the `latest_tag`',
  "tool of this plugin's `notes` server to find the previous release when the user names none.",
  '',
].join('\n');

/** `hooks/check-publish.mjs` of §18.3.5, as printed there. */
const CHECK_PUBLISH = [
  '// Denies `npm publish`, answering in the form named by --form (core or claude-code).',
  "const form = process.argv[process.argv.indexOf('--form') + 1]",
  "let text = ''",
  "process.stdin.on('data', (chunk) => (text += chunk))",
  "process.stdin.on('end', () => {",
  '  const input = JSON.parse(text)',
  '  const command =',
  "    form === 'core'",
  "      ? input.tool_profile?.name === 'shell' ? input.tool_profile.input.command : undefined",
  '      : input.tool_input?.command',
  "  if (typeof command !== 'string' || !/\\bnpm\\s+publish\\b/.test(command)) return",
  "  const reason = 'Packages are published by the release workflow, not by the agent.'",
  '  const output =',
  "    form === 'core'",
  "      ? { decision: 'deny', reason }",
  '      : {',
  '          hookSpecificOutput: {',
  "            hookEventName: 'PreToolUse',",
  "            permissionDecision: 'deny',",
  '            permissionDecisionReason: reason,',
  '          },',
  '        }',
  '  process.stdout.write(JSON.stringify(output))',
  '})',
  '',
].join('\n');

export const DENIAL = 'Packages are published by the release workflow, not by the agent.';

/** What both halves share. */
export const SHARED_FILES: Readonly<Record<string, string>> = {
  'skills/release-notes/SKILL.md': SKILL,
  'hooks/check-publish.mjs': CHECK_PUBLISH,
  'server/notes.mjs': '// The MCP stdio server of the example (not shown in §18.3.5).\n',
};

export const CORE_HALF: Readonly<Record<string, string>> = {
  ...SHARED_FILES,
  'plugin.json': pretty(CORE_MANIFEST),
  'mcp.json': pretty(CORE_MCP),
  'hooks.json': pretty(CORE_HOOKS),
};

export const CLAUDE_HALF: Readonly<Record<string, string>> = {
  ...SHARED_FILES,
  '.claude-plugin/plugin.json': pretty(CLAUDE_MANIFEST),
  '.mcp.json': pretty(CLAUDE_MCP),
  'hooks/hooks.json': pretty(CLAUDE_HOOKS),
};

/** A tree with a conversion's files written into it. */
export const written = (
  tree: Readonly<Record<string, string>>,
  converted: Conversion,
): Record<string, string> => ({
  ...tree,
  ...Object.fromEntries(converted.files.map((file) => [file.path, file.text])),
});

/** A parsed JSON file of a conversion's output. */
export function outputOf(converted: Conversion, path: string): unknown {
  const file = converted.files.find((one) => one.path === path);
  if (file === undefined) throw new Error(`the conversion wrote no ${path}`);
  return JSON.parse(file.text);
}

const fileOf = (converted: Conversion, path: string): string => {
  const file = converted.files.find((one) => one.path === path);
  if (file === undefined) throw new Error(`the conversion wrote no ${path}`);
  return file.text;
};

/** The server names of an MCP file a conversion wrote. */
export function serverNamesOf(converted: Conversion, path: string): string[] {
  const parsed: { mcpServers: Record<string, unknown> } = JSON.parse(fileOf(converted, path));
  return Object.keys(parsed.mcpServers);
}

/** The servers of an MCP file a conversion wrote. */
export function serversOf(converted: Conversion, path: string): Record<string, unknown> {
  const parsed: { mcpServers: Record<string, unknown> } = JSON.parse(fileOf(converted, path));
  return parsed.mcpServers;
}

/** The core handlers of a core hooks file a conversion wrote, by event. */
export function coreHandlersOf(converted: Conversion): Record<string, { args: string[] }[]> {
  const parsed: { hooks: Record<string, { args: string[] }[]> } = JSON.parse(
    fileOf(converted, 'hooks.json'),
  );
  return parsed.hooks;
}

/** The matcher groups of a Claude Code hooks file a conversion wrote, by event. */
export function claudeGroupsOf(converted: Conversion): Record<string, unknown[]> {
  const parsed: { hooks: Record<string, unknown[]> } = JSON.parse(
    fileOf(converted, 'hooks/hooks.json'),
  );
  return parsed.hooks;
}

/** Converts a tree with the files-based converter. */
export const convertTree = (tree: Readonly<Record<string, string>>): Promise<Conversion> =>
  convertPluginPackage({ files: memoryFiles(tree) });
