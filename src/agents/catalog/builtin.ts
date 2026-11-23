import type { ModelAlias } from '../model-value.ts';
import type { CatalogAgent, StartContext } from './types.ts';

/**
 * The built-in subagents (sub-agents.md, "Built-in subagents"): the three the page gives a tab each,
 * the "Other" table, and `web-fetch` where the session reads web pages through it (§5.12.4). A registry typed over the names, so a built-in added to the list fails the
 * typecheck until it says what it is. The docs publish no prompt (a built-in's is Claude Code's own),
 * so `prompt` is empty; a description is the page's own sentence for it (spec/judgments.md).
 */
export const BUILTIN_AGENT_NAMES = [
  'Explore',
  'Plan',
  'general-purpose',
  'claude',
  'statusline-setup',
  'claude-code-guide',
  'web-fetch',
] as const;
export type BuiltinAgentName = (typeof BUILTIN_AGENT_NAMES)[number];

/** The subagent a call without `subagent_type` runs: "a `general-purpose` subagent to fall back on". */
export const FALLBACK_AGENT: BuiltinAgentName = 'general-purpose';

interface BuiltinSpec {
  readonly description: string;
  /** "Tools: read-only tools; Write and Edit are denied". Absent: every tool available to subagents. */
  readonly denied?: readonly string[];
  /** Its own model; absent, the model order of "Choose a model" decides. */
  readonly model?: ModelAlias;
  readonly startsWith: StartContext;
  /** Its tool list; absent, the pool of "Available tools". */
  readonly tools?: readonly string[];
  /** The two `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS` removes. */
  readonly exploreOrPlan: boolean;
  /** Registered only in a session that reads web pages through it (`BuiltinPolicy.webFetch`). */
  readonly webFetch?: boolean;
}

const FULL: StartContext = { claudeMd: true, gitStatus: true };
/** "Explore and Plan start without your CLAUDE.md files and the git status snapshot". */
const LEAN: StartContext = { claudeMd: false, gitStatus: false };
const READ_ONLY = ['Write', 'Edit'] as const;
/** `web-fetch` "starts without CLAUDE.md"; the table names no other omission (§5.12.4). */
const WITHOUT_CLAUDE_MD: StartContext = { claudeMd: false, gitStatus: true };

const BUILTINS: { readonly [N in BuiltinAgentName]: BuiltinSpec } = {
  Explore: {
    description: 'A fast, read-only agent optimized for searching and analyzing codebases.',
    denied: READ_ONLY,
    startsWith: LEAN,
    exploreOrPlan: true,
  },
  Plan: {
    description:
      'A research agent used during plan mode to gather context before presenting a plan.',
    denied: READ_ONLY,
    startsWith: LEAN,
    exploreOrPlan: true,
  },
  'general-purpose': {
    description:
      'A capable agent for complex, multi-step tasks that require both exploration and action.',
    startsWith: FULL,
    exploreOrPlan: false,
  },
  claude: {
    description:
      "A catch-all for a task that doesn't fit a more specialized agent, with every tool available to subagents.",
    startsWith: FULL,
    exploreOrPlan: false,
  },
  'statusline-setup': {
    description: 'Configures your status line when you run /statusline.',
    model: 'sonnet',
    startsWith: FULL,
    exploreOrPlan: false,
  },
  'claude-code-guide': {
    description: 'Answers questions about Claude Code features.',
    model: 'haiku',
    startsWith: FULL,
    exploreOrPlan: false,
  },
  'web-fetch': {
    description: 'Reads a web page for Claude.',
    model: 'haiku',
    tools: ['WebFetch'],
    startsWith: WITHOUT_CLAUDE_MD,
    exploreOrPlan: false,
    webFetch: true,
  },
};

/** What switches the built-ins off. */
export interface BuiltinPolicy {
  /** False with `-p` or through the Agent SDK. */
  readonly interactive: boolean;
  /** `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS=1`. */
  readonly disableExplorePlan: boolean;
  /** `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS=1`; it acts in non-interactive mode only. */
  readonly disableAll: boolean;
  /**
   * The session reads web pages through the built-in `web-fetch` agent (§5.12.4): signed in with a
   * claude.ai account, feature flags fetched, no LLM gateway. The host decides; absent, it does not
   * and `web-fetch` is not registered. The deny rule `Agent(web-fetch)` removes it as any built-in.
   */
  readonly webFetch?: boolean;
}

/** Whether the built-in is registered under the policy: "registered by default in interactive sessions". */
function registered(spec: BuiltinSpec, policy: BuiltinPolicy): boolean {
  if (policy.disableAll && !policy.interactive) return false;
  if (spec.webFetch === true && policy.webFetch !== true) return false;
  return !(policy.disableExplorePlan && spec.exploreOrPlan);
}

function catalogEntry(name: BuiltinAgentName): CatalogAgent {
  const spec = BUILTINS[name];
  return {
    name,
    ownName: name,
    source: { kind: 'builtin' },
    definition: {
      description: spec.description,
      prompt: '',
      ...(spec.tools === undefined ? {} : { tools: [...spec.tools] }),
      ...(spec.denied === undefined ? {} : { disallowedTools: [...spec.denied] }),
      ...(spec.model === undefined ? {} : { model: spec.model }),
    },
    extras: {},
    hooks: undefined,
    path: undefined,
    startsWith: spec.startsWith,
  };
}

/** The built-in subagents a session registers, in the page's order. */
export function builtinAgents(policy: BuiltinPolicy): readonly CatalogAgent[] {
  return BUILTIN_AGENT_NAMES.filter((name) => registered(BUILTINS[name], policy)).map(catalogEntry);
}
