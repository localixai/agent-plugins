/**
 * How a skill runs once it is invoked (skills.md, "Run skills in a subagent", "Frontmatter
 * reference", "How injected commands run"; model-config.md, "Restrict model selection" and "Adjust
 * effort level"). Pure over the skill and what the host knows of the session; the host reads its
 * environment through `readExecutionEnvironment`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/execution.rs`, docs/RUST.md).
 */
import type { InvocableFrontmatter, SkillShell } from '../format/skill-types.ts';
import type { Environment } from '../ports/index.ts';
import type { EffortLevel } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('skills/execution');

type Runnable = Pick<
  InvocableFrontmatter,
  'agent' | 'background' | 'context' | 'effort' | 'model' | 'shell' | 'shellDeclared'
>;

/** What fires a skill; the last two always wait for a forked skill's result. */
export type Trigger = 'user' | 'model' | 'scheduled-task' | 'workflow-agent';

export interface ExecutionContext {
  /** False with `-p` or through the Agent SDK. */
  readonly interactive: boolean;
  /** `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` is `1`. */
  readonly backgroundTasksDisabled: boolean;
  readonly trigger: Trigger;
  /** The commands of forked skills still running. */
  readonly runningForks?: ReadonlySet<string>;
}

/** Why a forked skill waits for its result instead of running in the background. */
export type WaitReason =
  | 'background-false'
  | 'non-interactive'
  | 'background-tasks-disabled'
  | 'already-running'
  | 'scheduled-task'
  | 'workflow-agent';

export type Execution =
  | { readonly mode: 'inline' }
  | {
      readonly mode: 'fork';
      /** The subagent type that runs the skill. */
      readonly agent: string;
      /** The subagent runs while the user keeps working; its result arrives when it completes. */
      readonly background: boolean;
      readonly waitsFor: readonly WaitReason[];
      /** "A backgrounded fork also runs with the narrower tool set": waiting keeps the full one. */
      readonly fullToolSet: boolean;
      /** The subagent "doesn't see your conversation history". */
      readonly seesConversation: false;
    };

/** Inline, or a forked subagent and whether the caller waits for it. */
export const resolveExecution = (
  skill: Runnable & { readonly command: string },
  ctx: ExecutionContext,
): Execution => call('resolveExecution', skill, ctx);

/** The two variables a skill's execution reads (env-vars.md). */
export interface ExecutionEnvironment {
  readonly backgroundTasksDisabled: boolean;
  /** `CLAUDE_CODE_EFFORT_LEVEL`: a level, or `none` for `auto`/`unset` (no level is sent). */
  readonly effort?: EffortLevel | 'none';
}

export const readExecutionEnvironment = (environment: Environment): ExecutionEnvironment =>
  call('readExecutionEnvironment', environment);

/** What the host knows about which models may run (`availableModels`, the permission mode). */
export interface ModelAvailability {
  /** The organization's `availableModels` allowlist permits the model. */
  readonly allowed: (model: string) => boolean;
  /** Auto mode (and plan mode while the classifier reviews commands) supports the model. Absent: supported. */
  readonly supportedInMode?: (model: string) => boolean;
  /** For a forked skill: the newest permitted version of a blocked family alias. Absent: none. */
  readonly substitute?: (model: string) => string | undefined;
}

export interface TurnSession {
  readonly models?: ModelAvailability;
  /** `CLAUDE_CODE_EFFORT_LEVEL`, from `readExecutionEnvironment`. */
  readonly effortEnvironment?: EffortLevel | 'none' | undefined;
  /** `maxEffortLevel` or an organization cap: "still limits the level the skill ... runs at". */
  readonly maxEffort?: EffortLevel;
  /**
   * The levels the active model supports; an unsupported level "falls back to the highest supported
   * level at or below" (`xhigh` runs as `high`). Absent: all. Judgment: none at or below sends no level.
   */
  readonly supportedEfforts?: readonly EffortLevel[];
}

export type ModelKept = 'excluded' | 'unsupported-in-mode';

export interface EffortChoice {
  /** Absent when the environment says to send no level, or the model supports none at or below it. */
  readonly level?: EffortLevel;
  readonly source: 'skill' | 'environment';
}

export interface TurnOverrides {
  /** The rest of the turn, or the forked subagent. */
  readonly target: 'turn' | 'subagent';
  /** The model to run on; absent keeps the session's (or, for a fork, the inherited) model. */
  readonly model?: string;
  /** Why the skill's `model` was not used. */
  readonly modelKept?: ModelKept;
  /** A forked skill's blocked family alias was replaced by a permitted version. */
  readonly modelSubstituted?: true;
  readonly effort?: EffortChoice;
}

/**
 * The model and effort a skill sets for the rest of the turn, or for its forked subagent. Absent
 * fields override nothing: the session's own resolution order applies.
 */
export const resolveTurnOverrides = (skill: Runnable, session: TurnSession): TurnOverrides =>
  call('resolveTurnOverrides', skill, session);

export interface ShellAvailability {
  /** Bash is available (not Windows without Git Bash). */
  readonly bash: boolean;
  /** The PowerShell tool is enabled. */
  readonly powershell: boolean;
}

export type ShellResolution =
  | { readonly ok: true; readonly tool: SkillShell }
  | { readonly ok: false; readonly message: string };

/**
 * Which tool runs a skill's `` !`command` `` blocks, or the one failure that stops the invocation.
 * Judgment (spec/judgments.md): with neither tool the docs name no message; the invocation fails
 * naming the skill.
 */
export const resolveShell = (
  skill: Pick<Runnable, 'shell' | 'shellDeclared'> & { readonly name: string },
  tools: ShellAvailability,
): ShellResolution => call('resolveShell', skill, tools);
