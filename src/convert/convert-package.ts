import type { PluginIssue } from '../format/issues.ts';
import { detectPluginForm, type PluginForm } from '../loading/core/form.ts';
import { loadCorePackage } from '../loading/core/load-core.ts';
import { loadPlugin } from '../loading/load-plugin.ts';
import { validatePlugin } from '../loading/validate/validate-plugin.ts';
import type { PluginFiles } from '../ports/index.ts';
import { claudeCodeExtras, coreOutputProblems } from './judge.ts';
import { declaresModules, extensionNamespaces } from './namespaces.ts';
import { conversion, notConverted, stop } from './notes.ts';
import { overlayFiles } from './overlay.ts';
import { coreToClaudeCode } from './to-claude-code/index.ts';
import { claudeCodeToCore } from './to-core/index.ts';
import type { Conversion, ConversionNote, ConvertOptions } from './types.ts';

export interface ConvertPackageInput extends ConvertOptions {
  /** Read access to the plugin root; nothing is written through it. */
  readonly files: PluginFiles;
  readonly signal?: AbortSignal;
}

/**
 * Converts a plugin root into the other form (Agent Plugins Extended §18.4), reading it through the
 * files port and writing nothing: the form is decided (§1.3), the root loaded by that form's loader
 * (a core package with every extension namespace it carries, so each is reported), the pure
 * converter run, and its output judged before a host writes it. A file the output would overwrite
 * stops the conversion (§18.4.1 item 4: the source form is never touched), and so does output the
 * target does not accept (item 3): `claude plugin validate` errors for a Claude Code plugin, a core
 * component the core loader would skip. A dual package has both forms already and is not converted.
 * Never throws.
 */
export async function convertPluginPackage(input: ConvertPackageInput): Promise<Conversion> {
  const decision = await detectPluginForm({ files: input.files, signal: input.signal });
  if (decision.dual) {
    return conversion(
      'core',
      [],
      [stop('plugin.json', undefined, 'the root is already a dual package: both forms are there')],
    );
  }
  return CONVERTERS[decision.form](input);
}

const loadStops = (to: PluginForm, issues: readonly PluginIssue[]): Conversion =>
  conversion(
    to,
    [],
    issues
      .filter((found) => found.severity === 'error')
      .map((found) => stop(found.path, undefined, `the source does not load: ${found.message}`)),
  );

/** What the source's own loader skipped: the conversion cannot carry it, and says so. */
const skipped = (issues: readonly PluginIssue[]): ConversionNote[] =>
  issues
    .filter((found) => found.severity === 'error')
    .map((found) =>
      notConverted(found.path, undefined, `the source form skips it: ${found.message}`),
    );

const withNotes = (converted: Conversion, notes: readonly ConversionNote[]): Conversion =>
  conversion(converted.to, converted.files, [...notes, ...converted.notes], converted.runtime);

/** One strategy per source form (K1). */
const CONVERTERS: {
  readonly [F in PluginForm]: (input: ConvertPackageInput) => Promise<Conversion>;
} = {
  core: async (input) => {
    const { files, signal } = input;
    const extensions = await extensionNamespaces(files, signal);
    const loaded = await loadCorePackage({ files, extensions, ...(signal && { signal }) });
    if (loaded.package === undefined) return loadStops('claude-code', loaded.issues);
    const converted = withNotes(coreToClaudeCode(loaded.package, input), skipped(loaded.issues));
    return judged(input, converted, async (overlay) => {
      const report = await validatePlugin({ files: overlay, ...(signal && { signal }) });
      const errors = report.issues
        .filter((found) => found.severity === 'error')
        .map((found) => stop(found.file, found.path, `claude plugin validate: ${found.message}`));
      const loadedPlugin = await loadPlugin({ files: overlay, ...(signal && { signal }) });
      return [...errors, ...claudeCodeExtras(loadedPlugin.plugin)];
    });
  },
  'claude-code': async (input) => {
    const { files, signal } = input;
    const loaded = await loadPlugin({ files, ...(signal && { signal }) });
    if (loaded.plugin === undefined) return loadStops('core', loaded.issues);
    const mods = await declaresModules(files, signal);
    const converted = claudeCodeToCore(loaded.plugin, { ...input, mods });
    return judged(input, withNotes(converted, skipped(loaded.issues)), (overlay) =>
      coreOutputProblems(overlay, signal),
    );
  },
};

/** The conversion, stopped by a file it would overwrite or by what the target makes of its output. */
async function judged(
  input: ConvertPackageInput,
  converted: Conversion,
  judge: (overlay: PluginFiles) => Promise<readonly ConversionNote[]>,
): Promise<Conversion> {
  if (converted.stopped) return converted;
  const { files, signal } = input;
  const present = await Promise.all(converted.files.map((file) => files.exists(file.path, signal)));
  const occupied = converted.files
    .filter((_file, index) => present[index] === true)
    .map((file) =>
      stop(file.path, undefined, 'the file is already there, and a converter never overwrites'),
    );
  const notes = occupied.length > 0 ? occupied : await judge(overlayFiles(files, converted.files));
  return conversion(
    converted.to,
    converted.files,
    [...converted.notes, ...notes],
    converted.runtime,
  );
}
