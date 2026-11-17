import {
  checkDualManifests,
  CLAUDE_MANIFEST_FILE,
  CORE_MANIFEST_FILE,
} from '../../format/core/dual.ts';
import { readJson } from '../../format/read-json.ts';
import { errorIn, type ValidationIssue } from '../../format/validate/finding.ts';
import type { PluginFiles } from '../../ports/index.ts';
import { hasCoreManifest } from './form.ts';

/**
 * The dual-package check a host's validator runs (Agent Plugins Extended §18.3.1, §18.1.5 item 6):
 * on a root with both manifests, what of the shared manifest fields loads in one form only. It adds
 * to `validatePlugin`, which validates the Claude Code view alone, as `claude plugin validate` does
 * (J1.1). A root that is not a dual package gives `dual: false` and no findings.
 */
export async function checkDualPackage(input: {
  readonly files: PluginFiles;
  readonly signal?: AbortSignal | undefined;
}): Promise<{ readonly dual: boolean; readonly issues: readonly ValidationIssue[] }> {
  const { files, signal } = input;
  const [claudePresent, core] = await Promise.all([
    files.exists(CLAUDE_MANIFEST_FILE, signal),
    hasCoreManifest(files, signal),
  ]);
  if (!claudePresent || !core) return { dual: false, issues: [] };
  const [coreRead, claudeRead] = await Promise.all([
    readJson(files, CORE_MANIFEST_FILE, signal),
    readJson(files, CLAUDE_MANIFEST_FILE, signal),
  ]);
  if (!('value' in claudeRead)) {
    return {
      dual: true,
      issues: [
        errorIn(
          CLAUDE_MANIFEST_FILE,
          '',
          `${CLAUDE_MANIFEST_FILE} is not valid JSON: ${claudeRead.problem}`,
        ),
      ],
    };
  }
  const coreValue = 'value' in coreRead ? coreRead.value : undefined;
  return { dual: true, issues: checkDualManifests(coreValue, claudeRead.value) };
}
