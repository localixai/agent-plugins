/**
 * Substitution of `${...}` in one string. One pass over the original text: a substituted value is
 * never scanned again (skills.md, "Available string substitutions"), so a saved option holding
 * `${CLAUDE_PLUGIN_ROOT}` stays that text.
 *
 * Grammar (judgment: the docs name the forms, not the edges): a reference is `${` name `}` with no
 * braces inside; a `${VAR:-default}` default ends at the first `}` and nests nothing; there is no
 * escape (a backslash does not prevent a `${CLAUDE_*}` variable, skills.md); anything that is not
 * a known reference is left as written.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/substitute.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { FieldPolicy, SubstitutionScope } from './scope.ts';
import type { PluginVariable } from './variables.ts';

const call = coreModule('loading/resolve/substitute');

/** What a template referenced, in order of first appearance, once each. */
export type TokenReference =
  | { readonly kind: 'path-variable'; readonly name: PluginVariable }
  | { readonly kind: 'user-config'; readonly key: string }
  | { readonly kind: 'environment'; readonly name: string }
  | { readonly kind: 'extra'; readonly name: string };

export interface Substituted {
  readonly text: string;
  /** Warnings only: an unresolved reference stays as written and is reported. */
  readonly issues: readonly PluginIssue[];
  readonly referenced: readonly TokenReference[];
  /**
   * What stayed as written because nothing answered it: the names of `${VAR}` references with no
   * value and no default, and the keys of `${user_config.KEY}` references with no value or no
   * declaration. A caller that treats them as an error (an MCP server that cannot start) reads them here
   * instead of from the warning text.
   */
  readonly unresolved: Unresolved;
}

export interface Unresolved {
  readonly variables: readonly string[];
  readonly options: readonly string[];
}

/**
 * What stands for a sensitive value in skill, command and agent content, where "a sensitive value
 * there becomes a placeholder" (plugins/manifest-reference.md). The docs give no text; Claude Code
 * 2.1.319 sends the model this one, a command's body included, saved value or not
 * (the spec's model-content oracle).
 */
export const SENSITIVE_PLACEHOLDER = (key: string): string => call('SENSITIVE_PLACEHOLDER', key);

export type Answers = ReadonlyMap<PluginVariable, string>;

/** The substitution once the variables the template needs are known: no port is asked here. */
export const runWith = (
  template: string,
  policy: FieldPolicy,
  path: string,
  base: { scope: SubstitutionScope; answers: Answers },
): Substituted => call('runWith', template, policy, path, base);

/** The environment variables among `referenced`, each once, in order. */
export const environmentNames = (referenced: readonly TokenReference[]): string[] =>
  call('environmentNames', referenced);

/** The path variables whose value the template writes, under the policy's rendering. */
export const neededVariables = (
  template: string,
  scope: SubstitutionScope,
  policy: FieldPolicy,
): PluginVariable[] => call('neededVariables', template, scope, policy);
