/**
 * What `claude plugin validate` reports about one file (the oracle of docs/SPEC.md): the file it is
 * about, relative to the folder validated, the place in it (`skills[0]`, `userConfig.key.title`,
 * `frontmatter`), and the message as the official CLI of the same release prints it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/finding.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { Severity } from '../issues.ts';

export interface ValidationIssue {
  readonly severity: Severity;
  readonly file: string;
  readonly path: string;
  readonly message: string;
}

/** A finding before it is attached to a file: what each checker of one file's content returns. */
export interface Finding {
  readonly severity: Severity;
  readonly path: string;
  readonly message: string;
}

const call = coreModule('format/validate/finding');

export const errorIn = (file: string, path: string, message: string): ValidationIssue =>
  call('errorIn', file, path, message);
