import { error, type PluginIssue } from '../format/issues.ts';
import type { PluginManifest } from '../format/manifest-schema.ts';
import {
  engineNotes,
  engineRefuses,
  engineRefusal,
  engineVerdict,
  readEngines,
} from '../format/engines.ts';
import { note, type PluginNote } from '../format/plugin-notes.ts';
import type { Plugin } from './plugin-model.ts';

/** What the engines check reads from the load's input. */
export interface EngineInput {
  readonly clientVersion?: string;
  readonly localFolder?: boolean;
}

/**
 * The refusal of a plugin whose `engines["claude-code"]` range excludes the running client (spec
 * §2.4): it does not load, unless it is read from a local folder. Without a client version there is
 * nothing to check.
 */
export function engineGate(
  manifest: PluginManifest | undefined,
  name: string,
  input: EngineInput,
): PluginIssue | undefined {
  const { clientVersion } = input;
  if (clientVersion === undefined) return undefined;
  const verdict = engineVerdict(readEngines(manifest?.engines), {
    clientVersion,
    localFolder: input.localFolder === true,
  });
  return engineRefuses(verdict) && verdict.outside !== undefined
    ? error('engines.claude-code', engineRefusal(name, verdict.outside, clientVersion))
    : undefined;
}

/** Spec §2.4, Judgment J2.9: the note `claude plugin list` adds while a plugin sets `agent`. */
const AGENT_DEPRECATED =
  "A plugin's `agent` setting is deprecated and will stop applying in a future release.";

/** The notes of a plugin that loaded: the manifest's, the `engines` ones and the `agent` one. */
export function loadNotes(
  step: { readonly manifest: PluginManifest | undefined; readonly notes: readonly PluginNote[] },
  plugin: Plugin,
  input: EngineInput,
): readonly PluginNote[] {
  const { clientVersion } = input;
  const engines = engineNotes(
    readEngines(step.manifest?.engines),
    clientVersion === undefined
      ? undefined
      : { clientVersion, localFolder: input.localFolder === true },
  );
  const agent =
    plugin.settings.agent === undefined
      ? []
      : [note('plugin-agent-setting-deprecated', AGENT_DEPRECATED)];
  return [...step.notes, ...engines, ...agent];
}
