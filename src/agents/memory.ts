/**
 * The persistent memory of a subagent (sub-agents.md, "Enable persistent memory"). The kit plans
 * the directory, the part of `MEMORY.md` to include and the tools to add; reading and writing the
 * files in the system prompt are the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/memory.rs`, docs/RUST.md).
 */
import type { AgentMemoryScope } from '../format/agent-fields.ts';
import type { Locations, TextFileReader } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('agents/memory');

/**
 * The part of a `MEMORY.md` index that the subagent's system prompt includes: "the first 200 lines
 * or 25KB ..., whichever comes first". Frontmatter and block-level HTML comments are not loaded, so
 * they do not count (errors.md). `truncated` says content was cut, so the host adds the instruction
 * to curate the index. The cut at the byte limit is made between characters, not between lines
 * (judgment).
 */
export const truncateMemoryIndex = (text: string): { text: string; truncated: boolean } =>
  call('truncateMemoryIndex', text);

export interface MemoryPlan {
  /** The memory directory: the subagent's own, created by the host when absent. */
  readonly directory: string;
  /** What of `MEMORY.md` the system prompt includes (empty when the file does not exist). */
  readonly index: string;
  /** The index was cut at 200 lines or 25KB. */
  readonly truncated: boolean;
  /** "Read, Write, and Edit tools are automatically enabled so the subagent can manage its memory". */
  readonly addTools: readonly ['Read', 'Write', 'Edit'];
}

export interface MemoryContext {
  /** Auto memory is on: `autoMemoryEnabled` is not false and `CLAUDE_CODE_DISABLE_AUTO_MEMORY` is unset. */
  readonly autoMemoryEnabled: boolean;
  readonly locations: Pick<Locations, 'homeDir' | 'projectDir' | 'joinPath'>;
  /** `CLAUDE_CONFIG_DIR` as the session's environment gives it: the user scope's memory lives below it when set. */
  readonly configDir?: string | undefined;
  readonly files: TextFileReader;
  readonly signal?: AbortSignal;
}

/**
 * The persistent memory of a subagent from its `memory` scope: the directory (`~/.claude/agent-memory/<name>/`,
 * `.claude/agent-memory/<name>/` or `.claude/agent-memory-local/<name>/`), the part of its `MEMORY.md`
 * to include and the tools to add. With no scope, or with auto memory off ("the `memory` field has
 * no effect and the subagent launches without the memory instructions or the memory tool access"),
 * there is no plan. The instruction text for the system prompt is not quoted by the docs and is the
 * host's.
 */
export const planAgentMemory = (
  scope: AgentMemoryScope | undefined,
  agentType: string,
  ctx: MemoryContext,
): Promise<MemoryPlan | undefined> => call.async('planAgentMemory', scope, agentType, ctx);
