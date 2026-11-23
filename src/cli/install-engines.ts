import {
  engineInstallSuffix,
  engineRefusal,
  engineRefuses,
  engineVerdict,
  readEngines,
} from '../format/engines.ts';
import { commandFailed, type CommandOutput } from './result.ts';

/**
 * `engines` at `claude plugin install` (spec §2.4, §17.4): a plugin whose `engines["claude-code"]`
 * range excludes the running client does not install; one read from a local folder installs, its
 * success line continued with the note. The host gives the range the plugin declares (the entry's,
 * else its `plugin.json`'s) and picks the version it offers: "the newest version whose range fits".
 */
export interface InstallEngineFacts {
  /** The `engines` value of the version to install. */
  readonly engines: unknown;
  /** The running client's version, such as `2.1.330`. */
  readonly clientVersion: string;
  /** The plugin is read in place from a marketplace added from a local path. */
  readonly localFolder?: boolean;
}

export interface InstallEngineCheck {
  /** The install's failure, when the range refuses it. */
  readonly refusal?: CommandOutput;
  /** What the success line continues with; empty when nothing is to be said. */
  readonly suffix: string;
}

/**
 * The refusal is `Failed to install plugin "<id>": "<name>" needs Claude Code <range>, this is
 * <version>` with `failureCode` `engine_range`: the spec quotes neither (spec/judgments.md, J2.10).
 */
export function installEngineCheck(
  id: string,
  facts: InstallEngineFacts | undefined,
): InstallEngineCheck {
  if (facts === undefined) return { suffix: '' };
  const name = id.includes('@') ? id.slice(0, id.lastIndexOf('@')) : id;
  const verdict = engineVerdict(readEngines(facts.engines), {
    clientVersion: facts.clientVersion,
    localFolder: facts.localFolder === true,
  });
  const range = verdict.outside;
  if (range === undefined) return { suffix: '' };
  if (engineRefuses(verdict)) {
    const message = engineRefusal(name, range, facts.clientVersion);
    return {
      refusal: commandFailed('install', `Failed to install plugin "${id}": ${message}`, {
        failureCode: 'engine_range',
      }),
      suffix: '',
    };
  }
  return { suffix: engineInstallSuffix(name, range, facts.clientVersion) };
}
