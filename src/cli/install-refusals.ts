/** The refusals of `claude plugin install` before anything is written: bad `--config` values, a dependency closure that cannot be met. */
import type { ConfigOutcome } from './install-config.ts';
import { commandFailed, type CommandOutput } from './result.ts';

export function configRefusal(config: ConfigOutcome): CommandOutput {
  const lines = config.refused.map((problem) => `--config ${problem.key}: ${problem.message}`);
  return commandFailed('install', ['Failed to install plugin:', ...lines].join('\n'), {
    failureCode: 'invalid_config',
    fields: { invalidKeys: config.refused.map((problem) => problem.key) },
  });
}

export function closureRefusal(id: string, messages: readonly string[]): CommandOutput {
  return commandFailed('install', [`Failed to install plugin "${id}":`, ...messages].join('\n'), {
    failureCode: 'dependency_error',
  });
}
