import { coreModule } from '../wasm/bridge.ts';
import type { AgentFieldName, AgentFieldValue } from './agent-fields.ts';
import type { PluginIssue } from './issues.ts';

/**
 * The agent registry's fields beyond `name` and `description`, read from a frontmatter table.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/agent_field_read.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/agent-field-read');

/** `name` and `description`: `readAgent` reads them itself, as their problems decide if the file loads. */
type OwnFieldName = 'name' | 'description';
export type ReadFieldName = Exclude<AgentFieldName, OwnFieldName>;

type Draft = { -readonly [F in ReadFieldName]?: AgentFieldValue<F> };

/** What the registry read from a file's frontmatter. */
export interface AgentFieldsRead {
  /** The fields that came out usable, typed by name. */
  readonly values: Readonly<Draft>;
  /** Fields in the file that Claude Code does not apply for a plugin agent (components.md). */
  readonly ignored: readonly ReadFieldName[];
  readonly issues: readonly PluginIssue[];
}

/**
 * Reads every field of the registry beyond `name` and `description` from the frontmatter table.
 * A plugin agent skips the fields it ignores (and names them in `ignored` when the file has them).
 */
export const readAgentFields = (
  fields: Readonly<Record<string, unknown>>,
  options: { readonly plugin: boolean; readonly path: string },
): AgentFieldsRead => call('readAgentFields', fields, options);
