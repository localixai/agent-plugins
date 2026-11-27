import { coreModule } from '../../wasm/bridge.ts';
import type { AgentCatalog } from './resolve.ts';
import type { CatalogAgent } from './types.ts';

/**
 * Naming a subagent (sub-agents.md, "Invoke subagents explicitly"). Three patterns "escalate from a
 * one-off suggestion to a session-wide default": a name in the prompt (Claude decides, nothing to
 * parse), an @-mention (the subagent runs for that task) and the whole session as the subagent
 * (`--agent`, the `agent` setting). Pure over the catalog; the typeahead and the session start are the
 * host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/catalog/invoke.rs`, docs/RUST.md).
 */
const call = coreModule('agents/catalog/invoke');

export interface TypeaheadEntry {
  readonly label: string;
  /** What picking it writes into the prompt: `@"code-reviewer (agent)"`. */
  readonly insert: string;
  readonly agent: CatalogAgent;
}

/**
 * The subagents to offer after `@`. Judgment (spec/judgments.md): a subagent a deny rule switched off is
 * not offered (nor mentioned); the docs say only that the rule stops Claude using it.
 */
export const agentTypeahead = (catalog: AgentCatalog): readonly TypeaheadEntry[] =>
  call('agentTypeahead', catalog);

export interface AgentMention {
  readonly agent: CatalogAgent;
  /** The mention as written, `@agent-test-runner`. */
  readonly text: string;
}

export interface MentionsFound {
  /** Each subagent once, in the order first mentioned. */
  readonly mentions: readonly AgentMention[];
  /**
   * The message Claude receives: "Your full message still goes to Claude, which writes the subagent's
   * task prompt ... The @-mention controls which subagent Claude invokes, not what prompt it receives."
   */
  readonly message: string;
}

/**
 * The subagents a message mentions, from both spellings ("You can also type the mention manually without
 * using the picker: `@agent-<name>` for local subagents, or `@agent-` followed by the scoped name for
 * plugin subagents"). A name that is no subagent of the catalog selects nothing. Natural language is
 * not a mention: "Use the test-runner subagent" leaves the choice to Claude.
 */
export const agentMentions = (message: string, catalog: AgentCatalog): MentionsFound =>
  call('agentMentions', message, catalog);

/**
 * Where the choice of the session agent came from: `--agent` ("The CLI flag overrides the setting"),
 * the `agent` key of a settings source, or a plugin's default `agent` (plugins/components.md, "Default
 * settings": the lowest layer).
 */
export type SessionAgentSource = 'flag' | 'setting' | 'plugin';

/** The `agent` setting as the settings resolved it: the name, and the plugin when it is a plugin default. */
export interface AgentSetting {
  readonly name: string;
  /** The id of the plugin whose `settings.json` (or manifest `settings`) set it; absent for a settings source. */
  readonly plugin?: string;
}

export type SessionAgentChoice =
  | {
      readonly ok: true;
      readonly agent: CatalogAgent | undefined;
      readonly from?: SessionAgentSource;
      /** With `from: 'plugin'`: the id of the plugin whose default chose it. */
      readonly plugin?: string;
    }
  | {
      readonly ok: false;
      readonly message: string;
      /**
       * The agents the name could mean, in code-unit order of their scoped names: two or more for a
       * bare name several plugins provide, none for a name no agent has.
       */
      readonly candidates: readonly CatalogAgent[];
    };

/**
 * The subagent a session runs as: `--agent <name>` over the `agent` setting ("`--agent` takes
 * precedence over this key for one session"), the setting as `valueWithPluginDefaults` resolved it (a
 * settings source over the plugins' defaults). Oracle 2.1.322 (spec/judgments.md): a plugin's default
 * names an agent the way the setting does, so a user agent of that exact name wins over the plugin's own.
 * Judgment (spec/judgments.md): the docs name no failure for an unknown name, so the message is the
 * kit's, and a deny rule `Agent(name)` does not stop the person choosing it as the main thread.
 */
export const selectSessionAgent = (
  catalog: AgentCatalog,
  choice: { readonly flag?: string | undefined; readonly setting?: AgentSetting | undefined },
): SessionAgentChoice => call('selectSessionAgent', catalog, choice);

/** The startup header's mark: "The agent name appears as `@<name>`". */
export const sessionAgentLabel = (agent: CatalogAgent): string => call('sessionAgentLabel', agent);

export interface ResumedSessionAgent {
  readonly agent: CatalogAgent | undefined;
  /** Shown in the resumed conversation when the agent is gone. */
  readonly warning?: string;
}

/**
 * The agent of a resumed session (sub-agents.md: "the choice persists when you resume the session";
 * errors.md, "Session agent no longer available"). A missing one lets the session go on with the
 * default tools and a warning that names it and the directories searched.
 */
export const resumeSessionAgent = (
  catalog: AgentCatalog,
  name: string,
  searched: readonly string[],
): ResumedSessionAgent => call('resumeSessionAgent', catalog, name, searched);
