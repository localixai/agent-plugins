import type { PluginFiles } from '../../../ports/index.ts';
import type { ModCompiler } from '../../ports.ts';

/** What `validateMod` reads and what it reports (plugins/cli-reference.md, "Output and exit codes"). */

export interface ValidateInput {
  readonly files: PluginFiles;
  /** The path as the user typed it: the report names it as `target`. */
  readonly target: string;
  /**
   * The plugin directory's absolute path: a refusal of the static analysis names the module's
   * absolute file and the directory's name (Agent Plugins Extended §10.4.2, J10.2). Absent, `target`.
   */
  readonly root?: string;
  /** TypeScript, TSX and JSX modules need it; a JavaScript module is read as written. */
  readonly compiler?: ModCompiler;
  readonly signal?: AbortSignal;
}

/** An error or a warning of a `--json` result (Agent Plugins Extended J17.15). */
export interface ReportEntry {
  readonly path: string;
  readonly message: string;
}

export interface FileReport {
  readonly file: string;
  readonly errors: readonly ReportEntry[];
  readonly warnings: readonly ReportEntry[];
  readonly notes: readonly string[];
  /** Every result carries it, empty when the file gates nothing (Agent Plugins Extended J17.15). */
  readonly gatingHooks: readonly {
    readonly module: string;
    /** The pattern `on` was given; `hook` adds its matcher in braces, as the `hooks:` line does. */
    readonly pattern: string;
    readonly hook: string;
    readonly hasCatch: boolean;
  }[];
}

/** The `--json` object (cli-reference.md, "Output and exit codes"). */
export interface ValidateReport {
  readonly success: boolean;
  readonly strict: boolean;
  readonly target: string;
  readonly manifest: FileReport | null;
  readonly contents: readonly FileReport[];
  readonly advice: readonly never[];
  /** What the command prints, line by line. */
  readonly lines: readonly string[];
  /** 0 passed, 1 failed (an error, or a warning under `--strict`). */
  readonly exitCode: 0 | 1;
}
