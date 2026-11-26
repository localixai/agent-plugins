/**
 * What a Read call returns at most, and the files it refuses before it reads a byte
 * (tools-reference.md, "Read tool behavior", "Large files").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/read_limits.rs`,
 * docs/RUST.md).
 */
import type { Environment } from '../../../ports/environment.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/read-limits');

/**
 * What one Read call returns at most, in tokens: 25,000, or `CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS`
 * (env-vars.md: "Override the default token limit for file reads"). A value that is not a positive
 * integer counts as unset (spec/judgments.md).
 */
export const readTokenLimit = (env: Pick<Environment, 'get'>): number =>
  call('readTokenLimit', env);

export interface ReadTarget {
  readonly path: string;
  readonly bytes: number;
}

export interface ReadRefusal {
  readonly reason: 'binary-extension' | 'image-too-large' | 'notebook-too-large';
  readonly message: string;
}

/**
 * Whether Read refuses a file before it reads a byte (tools-reference.md, "Read tool behavior"): an
 * extension it treats as binary, an image over 128 MB, a notebook over 100 MB. Megabytes are
 * 1024 x 1024 bytes and only the four extensions the docs name are modelled (spec/judgments.md);
 * the wording of the refusals is a judgment too.
 */
export const classifyReadTarget = (target: ReadTarget): ReadRefusal | undefined =>
  call('classifyReadTarget', target);

export interface ReadSizing {
  /** The size of the text file on disk. */
  readonly bytes: number;
  /** What the text to return comes to, in tokens: the whole file, or the `offset` and `limit` window. */
  readonly tokens: number;
  /** The call passes `offset` or `limit`. */
  readonly windowed: boolean;
  /** The call passes `allow_large: true`. */
  readonly allowLarge: boolean;
  /** The room left in the context window, in tokens; `allow_large` is sized against it. */
  readonly contextRoomTokens?: number | undefined;
}

/** `whole`: all of it; `partial`: the first page and a `PARTIAL view` notice; `refused`: an error. */
export type ReadOutcome = 'whole' | 'partial' | 'refused';

/**
 * What a text read returns (tools-reference.md, "Large files"). Over the default limits: a whole
 * file over 256 KB is refused, a whole file over the token limit gives its first page with a
 * `PARTIAL view` notice, and an `offset` or `limit` read over the token limit is refused. With
 * `allow_large` the read "is sized against the room left in the session's context window instead of
 * the default limits"; over that room it is refused (spec/judgments.md: the docs do not say what
 * a read over the room gets). Without a measured room `allow_large` has no effect.
 */
export const classifyReadSize = (sizing: ReadSizing, env: Pick<Environment, 'get'>): ReadOutcome =>
  call('classifyReadSize', sizing, env);

/**
 * The notice of a partial view: "how much of the file it received and how to read more with
 * `offset` and `limit`". It always starts with `PARTIAL view`; the rest is a judgment.
 */
export const partialViewNotice = (shownLines: number, totalLines: number): string =>
  call('partialViewNotice', shownLines, totalLines);

/** The error of a read that is too large: a portion with `offset` and `limit`, or Grep (wording is a judgment). */
export const READ_TOO_LARGE_MESSAGE: string = call('READ_TOO_LARGE_MESSAGE');
