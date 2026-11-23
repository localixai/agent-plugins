import type { BuiltinToolName } from './official-tools.ts';
import type { Assert, Equal } from './type-equality.ts';

/**
 * The tools of tools-reference.md ("Tool" column of the first table), in the table's order. The
 * docs say which tools exist and what each is called; a rule or a hook matcher uses these
 * canonical names, never the label a tool shows in the transcript (permissions.md, "Match all
 * uses of a tool").
 */
export const DOCUMENTED_TOOLS = [
  'Agent',
  'Artifact',
  'AskUserQuestion',
  'Bash',
  'CronCreate',
  'CronDelete',
  'CronList',
  'Edit',
  'EndConversation',
  'EnterPlanMode',
  'EnterWorktree',
  'ExitPlanMode',
  'ExitWorktree',
  'Glob',
  'Grep',
  'ListAgents',
  'ListMcpResourcesTool',
  'ListPlugins',
  'ListSkills',
  'LSP',
  'Monitor',
  'NotebookEdit',
  'PowerShell',
  'propose_skills',
  'PushNotification',
  'Read',
  'ReadMcpResourceTool',
  'RemoteTrigger',
  'ReportFindings',
  'ScheduleWakeup',
  'SearchPlugins',
  'SearchSkills',
  'SendFeedback',
  'SendMessage',
  'SendUserFile',
  'ShareOnboardingGuide',
  'Skill',
  'SubagentHandback',
  'SuggestPluginInstall',
  'SuggestSkills',
  'TaskCreate',
  'TaskGet',
  'TaskList',
  'TaskOutput',
  'TaskStop',
  'TaskUpdate',
  'TodoWrite',
  'ToolSearch',
  'WaitForMcpServers',
  'WebFetch',
  'WebSearch',
  'Workflow',
  'Write',
] as const;

export type DocumentedTool = (typeof DOCUMENTED_TOOLS)[number];

/**
 * Tools the SDK's `BUILTIN_TOOL_NAMES` lists that the tools-reference table does not: the docs
 * are silent on them, so the kit names them but claims nothing about their behaviour.
 */
export const SDK_ONLY_TOOLS = [
  'ClaudeDesign',
  'OfferChromeSetup',
  'Projects',
  'ReadMcpResourceDirTool',
  'ReadNotifications',
  'SendUserMessage',
  'SetGoal',
  'ShowOnboardingRolePicker',
] as const;

export type SdkOnlyTool = (typeof SDK_ONLY_TOOLS)[number];

/**
 * Tools the tools-reference table lists that the SDK's `BUILTIN_TOOL_NAMES` does not: the claude.ai
 * account tools, offered "only in a cloud session or the desktop app" (tools-reference.md,
 * `ListPlugins`). They are documented names a rule or a hook may match like any other.
 */
export const DOCS_ONLY_TOOLS = [
  'ListPlugins',
  'ListSkills',
  'SearchPlugins',
  'SearchSkills',
  'SuggestPluginInstall',
  'SuggestSkills',
] as const satisfies readonly DocumentedTool[];

export type DocsOnlyTool = (typeof DOCS_ONLY_TOOLS)[number];

/**
 * Tools Claude Code removed but a rule may still name: the table says `TaskOutput` was "Removed in
 * v2.1.277", and permissions.md ("Tool name wildcards") exempts removed tools from the
 * matches-no-tool warning.
 */
export const REMOVED_TOOLS = ['TaskOutput'] as const satisfies readonly DocumentedTool[];

export type RemovedTool = (typeof REMOVED_TOOLS)[number];

/**
 * @public A compile-time drift guard: a tool the SDK adds or drops, or the table adds or drops,
 * fails the typecheck until the lists are updated.
 */
export type ToolNamesMatchSdk = Assert<
  Equal<Exclude<DocumentedTool, RemovedTool | DocsOnlyTool> | SdkOnlyTool, BuiltinToolName>
>;
