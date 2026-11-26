import { coreModule } from '../wasm/bridge.ts';
import type { ValidationAdvice } from './validate-run.ts';

const call = coreModule('cli/validate-advice');

/**
 * The advice on a README (plugins/cli-reference.md, "Advice on the README"): a plugin's README with
 * no line that installs the plugin from a marketplace gets the line to add. It is neither an error
 * nor a warning and never changes the verdict.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/validate_advice.rs`, docs/RUST.md).
 */

/** Whether the README already holds a line that installs `plugin` from a marketplace. */
export const readmeInstallsPlugin = (readme: string, plugin: string): boolean =>
  call('readmeInstallsPlugin', readme, plugin);

/**
 * The advice for a README that lacks the line, or none. `source` is the marketplace's source
 * (`owner/repo`); without one the line holds `<owner>/<repo>`, which "pasting it unchanged doesn't
 * clear".
 */
export const readmeAdvice = (input: {
  readonly file: string;
  readonly plugin: string;
  readonly readme: string;
  readonly source?: string;
}): ValidationAdvice | undefined => call('readmeAdvice', input);

/** Whether an advice line still holds the placeholder, so pasting it does not clear the advice. */
export const adviceNeedsSource = (advice: ValidationAdvice): boolean =>
  call('adviceNeedsSource', advice);

/** The `Advice` block as the page prints it, or nothing when there is none. */
export const renderAdvice = (advice: readonly ValidationAdvice[]): readonly string[] =>
  call('renderAdvice', advice);

/**
 * The marketplace source Claude Code fills in from "the `origin` remote of the `.git` directory at the
 * top of" the validated directory; `undefined` when the remote is not one it can use (judgment,
 * spec/judgments.md: a GitHub remote is the one the install line's `owner/repo` form names).
 */
export const sourceFromOrigin = (remote: string | undefined): string | undefined =>
  call('sourceFromOrigin', remote);
