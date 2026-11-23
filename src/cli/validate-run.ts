import type { ValidationIssue } from '../format/validate/finding.ts';
import type { ExitCode } from './result.ts';
import type { ValidationHosts, ValidationProvides } from './validate-lists.ts';

/**
 * What `claude plugin validate` reports and how it exits (plugins/cli-reference.md, "plugin
 * validate", "Output and exit codes", "Advice on the README"; plugins/manifest-reference.md,
 * "Validate the manifest"). The kit validates a folder through `validatePlugin`; this turns its
 * findings into the verdict line, the exit code and the `--json` report. Pure: a host prints.
 */

/** The manifest a target is validated through: the report's `manifest`, or none for a bare run. */
const MANIFEST_FILES: Readonly<Record<string, string | undefined>> = {
  marketplace: '.claude-plugin/marketplace.json',
  plugin: '.claude-plugin/plugin.json',
  components: undefined,
  none: undefined,
};

export interface ValidationVerdict {
  /** `true` when the run exits `0`. */
  readonly success: boolean;
  readonly exitCode: ExitCode;
  /** `Validation passed`, `Validation passed with warnings`, `Validation failed` ... */
  readonly line: string;
}

/**
 * The verdict: an error fails the run; a warning fails it only under `--strict`; "Advice doesn't
 * count, with or without `--strict`", so it is not among the findings.
 */
export function validationVerdict(
  issues: readonly Pick<ValidationIssue, 'severity'>[],
  strict: boolean,
): ValidationVerdict {
  const errors = issues.some((issue) => issue.severity === 'error');
  const warnings = issues.some((issue) => issue.severity === 'warning');
  if (errors) return { success: false, exitCode: 1, line: 'Validation failed' };
  if (warnings && strict) {
    return {
      success: false,
      exitCode: 1,
      line: 'Validation failed (--strict treats warnings as errors)',
    };
  }
  return {
    success: true,
    exitCode: 0,
    line: warnings ? 'Validation passed with warnings' : 'Validation passed',
  };
}

/** Exit `2`: "the validator itself failed, such as on an unreadable path"; the message goes to stderr, stdout stays empty. */
export const unexpectedValidationError = (
  reason: string,
): { readonly exitCode: 2; readonly stderr: string } => ({
  exitCode: 2,
  stderr: `Unexpected error during validation: ${reason}`,
});

/**
 * A mod hook that can refuse an action (such as `tool.call`), and whether it has a `.catch` handler
 * (J17.15): `module` as `modules` writes it, `pattern` the event pattern given to `on`, `hook` that
 * pattern with its matcher in braces (`tool.call{tool=Bash}`).
 */
export interface GatingHook {
  readonly module: string;
  readonly pattern: string;
  readonly hook: string;
  readonly hasCatch: boolean;
}

/** One entry of `errors` or `warnings` (J17.15); a `notes` entry is a plain string. */
export interface ValidationEntry {
  readonly path: string;
  readonly message: string;
}

export interface ValidationFileResult {
  readonly file: string;
  readonly errors: readonly ValidationEntry[];
  readonly warnings: readonly ValidationEntry[];
  readonly notes: readonly string[];
  readonly gatingHooks: readonly GatingHook[];
}

/** One entry of `advice`: the README, the message and the line to paste. */
export interface ValidationAdvice {
  readonly file: string;
  readonly message: string;
  readonly installLine: string;
}

export interface ValidationJson {
  readonly success: boolean;
  readonly strict: boolean;
  readonly target: string;
  readonly manifest: ValidationFileResult | null;
  readonly contents: readonly ValidationFileResult[];
  readonly advice: readonly ValidationAdvice[];
  /** The plugin's components by kind; `null` when the run cannot list them (spec §17.13.4). */
  readonly provides: ValidationProvides | null;
  /** The hosts the plugin can reach; `null` when the run cannot list them (spec §17.13.4). */
  readonly hosts: ValidationHosts | null;
}

function resultOf(
  file: string,
  issues: readonly ValidationIssue[],
  extra: {
    readonly notes: readonly string[] | undefined;
    readonly gatingHooks: readonly GatingHook[] | undefined;
  },
): ValidationFileResult {
  const pick = (severity: ValidationIssue['severity']) =>
    issues
      .filter((issue) => issue.severity === severity)
      .map(({ path, message }) => ({ path, message }));
  const { notes = [], gatingHooks = [] } = extra;
  return { file, errors: pick('error'), warnings: pick('warning'), notes, gatingHooks };
}

/**
 * The `--json` report: `success` is the verdict the exit code gives, `strict` whether warnings
 * failed the run, `target` the resolved path validated, `manifest` the manifest's own result (null
 * with no manifest), `contents` one result per other file, only for files with something to report,
 * and `advice`. Each result carries `gatingHooks`, the mod hooks that can refuse an action. For a directory with `marketplace.json` and `plugin.json`, `manifest` is the
 * marketplace result and the plugin's go in `contents`. The shape inside a result is a judgment
 * (spec/judgments.md): the docs name `file`, `errors`, `warnings` and `notes`, not what an entry holds.
 */
/** `provides` and `hosts` of a run that listed them, else `null` for both. */
const listedOf = (
  lists: { readonly provides: ValidationProvides; readonly hosts: ValidationHosts } | undefined,
): Pick<ValidationJson, 'provides' | 'hosts'> =>
  lists === undefined ? { provides: null, hosts: null } : lists;

export function validationJson(input: {
  readonly target: 'marketplace' | 'plugin' | 'components' | 'none';
  /** The resolved path Claude Code validated. */
  readonly path: string;
  readonly issues: readonly ValidationIssue[];
  readonly strict: boolean;
  readonly advice?: readonly ValidationAdvice[];
  /** The gating hooks the mods validator found, by the file that declares them. */
  readonly gatingHooks?: Readonly<Record<string, readonly GatingHook[]>>;
  /** The mods validator's lines, by the file they belong to. */
  readonly notes?: Readonly<Record<string, readonly string[]>>;
  /**
   * What a plugin run lists (`pluginProvides`, `pluginHosts` over the plugin `loadPlugin` gives);
   * absent for a run that cannot list them, whose report then carries `null`.
   */
  readonly lists?: { readonly provides: ValidationProvides; readonly hosts: ValidationHosts };
}): ValidationJson {
  const manifestFile = MANIFEST_FILES[input.target];
  const gates = input.gatingHooks ?? {};
  const notes = input.notes ?? {};
  const byFile = new Map<string, ValidationIssue[]>();
  for (const issue of input.issues) {
    byFile.set(issue.file, [...(byFile.get(issue.file) ?? []), issue]);
  }
  // Judgment: a file whose only report is a gating hook or a note still has a result (spec/judgments.md).
  for (const file of [...Object.keys(gates), ...Object.keys(notes)]) {
    if (!byFile.has(file)) byFile.set(file, []);
  }
  const resultFor = (file: string): ValidationFileResult =>
    resultOf(file, byFile.get(file) ?? [], { notes: notes[file], gatingHooks: gates[file] });
  const contents = [...byFile.keys()].filter((file) => file !== manifestFile).map(resultFor);
  return {
    success: validationVerdict(input.issues, input.strict).success,
    strict: input.strict,
    target: input.path,
    manifest: manifestFile === undefined ? null : resultFor(manifestFile),
    contents,
    advice: input.advice ?? [],
    ...listedOf(input.lists),
  };
}

/**
 * Judgment (spec/judgments.md): the fields only `marketplace.json` defines and the ones only
 * `plugin.json` defines, for a file the docs let the validator classify by its fields ("such as
 * `owner`", "such as `author`"). The marketplace set is `owner` and `plugins`; the plugin set is
 * the manifest's own component and identity keys.
 */
const MARKETPLACE_ONLY: readonly string[] = ['owner', 'plugins'];
const PLUGIN_ONLY: readonly string[] = [
  'author',
  'commands',
  'agents',
  'skills',
  'hooks',
  'mcpServers',
  'lspServers',
  'userConfig',
  'channels',
  'monitors',
  'dependencies',
  'settings',
  'defaultEnabled',
];

/**
 * What a manifest file is validated as (plugins/cli-reference.md, "Validate a manifest file"):
 * `plugin.json` and `marketplace.json` by name; any other file in a `.claude-plugin` directory is a
 * plugin manifest; elsewhere a marketplace when `plugins` is an array, when `plugins` and `owner` are
 * both present, or when it has a marketplace-only field and no plugin-only one; otherwise a plugin.
 */
export function manifestFileKind(input: {
  readonly name: string;
  readonly inClaudePluginDirectory: boolean;
  readonly value: Readonly<Record<string, unknown>>;
}): 'marketplace' | 'plugin' {
  if (input.name === 'marketplace.json') return 'marketplace';
  if (input.name === 'plugin.json' || input.inClaudePluginDirectory) return 'plugin';
  const has = (key: string): boolean => key in input.value;
  if (Array.isArray(input.value['plugins'])) return 'marketplace';
  if (has('plugins') && has('owner')) return 'marketplace';
  return MARKETPLACE_ONLY.some(has) && !PLUGIN_ONLY.some(has) ? 'marketplace' : 'plugin';
}
