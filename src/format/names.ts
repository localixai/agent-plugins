/**
 * Plugin names (plugins/manifest-reference.md, "`name`"). Every component is namespaced under the
 * name, so it may not be empty or hold a space, `@`, `:`, a path separator, a control character
 * or a bidirectional-formatting character: such a name never loads. The rest is the validator's;
 * its wording is the oracle's (docs/SPEC.md, "Sources of truth").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/names.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

const call = coreModule('format/names');

/**
 * A control (C0 or C1) or bidirectional-formatting character (`Bidi_Control`: the embeddings,
 * overrides and isolates, and LRM, RLM and ALM): the one class the loader and the validator refuse
 * in a plugin or marketplace name.
 */
export const CONTROL_OR_BIDI: RegExp = call('CONTROL_OR_BIDI');

/** Whether a plugin of this name can load at all. */
export const nameLoadIssues = (name: string): PluginIssue[] => call('nameLoadIssues', name);

/** Where a name is reported: the manifest's own `name`, or a marketplace entry's (shorter advice). */
export const NAME_CONTEXTS: readonly ['manifest', 'entry'] = call('NAME_CONTEXTS');
export type NameContext = (typeof NAME_CONTEXTS)[number];

/**
 * The reserved-name report of `claude plugin validate`: an error for a name that passes as
 * Anthropic's own, a warning for one that reads as such (oracle 2.1.319).
 */
export const nameReservedIssues = (
  name: string,
  context: NameContext = 'manifest',
): PluginIssue[] => call('nameReservedIssues', name, context);

/**
 * What `claude plugin validate` adds about a name that loads. The reserved-name report is left out
 * for a plugin an official marketplace lists (Anthropic's own plugins carry such names) and for one
 * its marketplace entry already reported.
 */
export const nameValidationIssues = (
  name: string,
  skipReserved = false,
  context: NameContext = 'manifest',
): PluginIssue[] => call('nameValidationIssues', name, skipReserved, context);
