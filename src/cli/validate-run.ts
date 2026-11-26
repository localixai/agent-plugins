import type { ValidationIssue } from '../format/validate/finding.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ExitCode } from './result.ts';

const call = coreModule('cli/validate-run');

/**
 * What `claude plugin validate` reports and how it exits (plugins/cli-reference.md, "plugin
 * validate", "Output and exit codes", "Advice on the README"; plugins/manifest-reference.md,
 * "Validate the manifest"). The kit validates a folder through `validatePlugin`; this turns its
 * findings into the verdict line, the exit code and the `--json` report. Pure: a host prints.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/validate_run.rs`, docs/RUST.md).
 */

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
export const validationVerdict = (
  issues: readonly Pick<ValidationIssue, 'severity'>[],
  strict: boolean,
): ValidationVerdict => call('validationVerdict', issues, strict);

/** Exit `2`: "the validator itself failed, such as on an unreadable path"; the message goes to stderr, stdout stays empty. */
export const unexpectedValidationError = (
  reason: string,
): { readonly exitCode: 2; readonly stderr: string } => call('unexpectedValidationError', reason);

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
}

/**
 * The `--json` report: `success` is the verdict the exit code gives, `strict` whether warnings
 * failed the run, `target` the resolved path validated, `manifest` the manifest's own result (null
 * with no manifest), `contents` one result per other file, only for files with something to report,
 * and `advice`. Each result carries `gatingHooks`, the mod hooks that can refuse an action. For a directory with `marketplace.json` and `plugin.json`, `manifest` is the
 * marketplace result and the plugin's go in `contents`. The shape inside a result is a judgment
 * (spec/judgments.md): the docs name `file`, `errors`, `warnings` and `notes`, not what an entry holds.
 */
export const validationJson = (input: {
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
}): ValidationJson => call('validationJson', input);

/**
 * What a manifest file is validated as (plugins/cli-reference.md, "Validate a manifest file"):
 * `plugin.json` and `marketplace.json` by name; any other file in a `.claude-plugin` directory is a
 * plugin manifest; elsewhere a marketplace when `plugins` is an array, when `plugins` and `owner` are
 * both present, or when it has a marketplace-only field and no plugin-only one; otherwise a plugin.
 */
export const manifestFileKind = (input: {
  readonly name: string;
  readonly inClaudePluginDirectory: boolean;
  readonly value: Readonly<Record<string, unknown>>;
}): 'marketplace' | 'plugin' => call('manifestFileKind', input);
