/**
 * env-vars.md, "Variables — CLAUDE_CODE_FORK_SUBAGENT" and "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS";
 * agent-teams.md, "Use subagent definitions for teammates"; errors.md, "Teammate's agent definition
 * was not restored" (Agent Plugins Extended §5.12.3, §5.12.5, §5.12.6). Docs paragraphs beyond the
 * requirement snapshot, so outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import {
  agentTeamsOn,
  DEFINITION_NOT_RESTORED,
  forkModeOn,
  restoreTeammate,
  runsInBackground,
  teammateSetup,
} from '../../src/entry/agents.ts';
import type { AgentDefinition } from '../../src/protocol/official.ts';

describe('fork mode', () => {
  test('is on by default and off for an off spelling', () => {
    expect(forkModeOn(undefined)).toBe(true);
    expect(forkModeOn('1')).toBe(true);
    for (const off of ['0', 'false', 'OFF']) expect(forkModeOn(off)).toBe(false);
  });

  test('while on, an interactive main conversation cannot wait: the subagent starts in the background', () => {
    expect(runsInBackground({ claudeWillWait: true, interactive: true })).toBe(true);
    expect(runsInBackground({ claudeWillWait: true, interactive: true, forkSubagent: '0' })).toBe(
      false,
    );
  });

  test('-p and the SDK can wait; CLAUDE_CODE_DISABLE_BACKGROUND_TASKS still runs it in the foreground', () => {
    expect(runsInBackground({ claudeWillWait: true, interactive: false })).toBe(false);
    expect(
      runsInBackground({ claudeWillWait: true, interactive: true, backgroundDisabled: true }),
    ).toBe(false);
  });
});

describe('agent teams', () => {
  test('are off by default and on for an on spelling of CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS', () => {
    expect(agentTeamsOn(undefined)).toBe(false);
    expect(agentTeamsOn('0')).toBe(false);
    expect(agentTeamsOn('1')).toBe(true);
    expect(agentTeamsOn('true')).toBe(true);
  });
});

const definition: AgentDefinition = {
  description: 'Reviews code',
  prompt: 'Review carefully.',
  tools: ['Read', 'Grep'],
  disallowedTools: ['Bash', 'SendMessage'],
  model: 'sonnet',
  effort: 'high',
  autoCompactWindow: 100_000,
  skills: ['lint'],
  mcpServers: ['github'],
};

describe('a teammate spawned from a definition', () => {
  test('in process: the list plus SendMessage and the Task tools, the body appended', () => {
    const setup = teammateSetup({
      definition,
      display: 'in-process',
      taskTools: true,
      source: 'project',
    });
    expect(setup).toEqual({
      tools: ['Read', 'Grep', 'SendMessage', 'TaskCreate', 'TaskGet', 'TaskList', 'TaskUpdate'],
      disallowedTools: ['Bash'],
      model: 'sonnet',
      effort: 'high',
      autoCompactWindow: 100_000,
      prompt: { placement: 'append', text: 'Review carefully.' },
    });
  });

  test('in a split pane: the list only, the body in place of the prompt, MCP servers applied', () => {
    const setup = teammateSetup({
      definition,
      display: 'split-pane',
      taskTools: true,
      source: 'user',
      requestedModel: 'opus',
    });
    expect(setup).toEqual({
      tools: ['Read', 'Grep'],
      mcpServers: ['github'],
      model: 'opus',
      prompt: { placement: 'replace', text: 'Review carefully.' },
    });
  });

  test("a plugin agent's mcpServers stay ignored; without the Task tools only SendMessage is added", () => {
    expect(
      teammateSetup({ definition, display: 'split-pane', taskTools: false, source: 'plugin' })
        .mcpServers,
    ).toBeUndefined();
    expect(
      teammateSetup({ definition, display: 'in-process', taskTools: false, source: 'plugin' })
        .tools,
    ).toEqual(['Read', 'Grep', 'SendMessage']);
  });
});

describe('bringing a stopped teammate back', () => {
  test('a project definition from a folder that is not trusted is not re-applied, with the notice', () => {
    expect(restoreTeammate({ source: 'project', folderTrusted: false })).toEqual({
      reapplied: false,
      notice: DEFINITION_NOT_RESTORED,
    });
    expect(DEFINITION_NOT_RESTORED).toStartWith(
      'Its agent definition was not restored: the folder its definition file came from is not trusted (source: projectSettings)',
    );
  });

  test('a trusted folder, a plugin agent and a user agent are re-applied', () => {
    expect(restoreTeammate({ source: 'project', folderTrusted: true }).reapplied).toBe(true);
    expect(restoreTeammate({ source: 'plugin', folderTrusted: false }).reapplied).toBe(true);
    expect(restoreTeammate({ source: 'user', folderTrusted: false }).reapplied).toBe(true);
  });
});
