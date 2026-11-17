import type { PluginForm } from '../loading/core/form.ts';

/**
 * What a conversion between the two plugin forms gives (Agent Plugins Extended §18.4): the files to
 * write beside the source, as data, and a note for everything it could not carry over or carried
 * over with a difference (§18.4.1 item 1). The kit writes nothing: a host writes `files` into the
 * plugin root, which makes a dual package (§18.4.1 item 4), and shows `notes`.
 */

/**
 * `stopped`: nothing is written (a name the target refuses, a file already there, output the target
 * would not accept); `not-converted`: a field, entry, file or behaviour the target form does not
 * get; `reported`: carried over, but it acts differently or the target warns about it.
 */
export const CONVERSION_OUTCOMES = ['stopped', 'not-converted', 'reported'] as const;
export type ConversionOutcome = (typeof CONVERSION_OUTCOMES)[number];

export interface ConversionNote {
  readonly outcome: ConversionOutcome;
  /** The file the note is about, relative to the plugin root (`mcp.json`, `commands/deploy.md`). */
  readonly path: string;
  /** Where in the file, dotted as the readers name it (`mcpServers.notes.cwd`); absent for a whole file. */
  readonly field?: string;
  readonly message: string;
}

/** A file to write, relative to the plugin root, `/`-separated. */
export interface ConvertedFile {
  readonly path: string;
  readonly text: string;
}

export interface Conversion {
  /** The form the files describe. */
  readonly to: PluginForm;
  /** A `stopped` note holds: `files` is empty. */
  readonly stopped: boolean;
  readonly files: readonly ConvertedFile[];
  readonly notes: readonly ConversionNote[];
  /** The interpreter a written hook adapter needs on `PATH` (§18.4.4): `node`. */
  readonly runtime?: string;
}

export interface ConvertOptions {
  /**
   * Route every hook handler through a hook adapter (§18.4.4), the default; `false` reports every
   * handler as not converted instead (§18.4.2 item 4, §18.4.3 item 4).
   */
  readonly hookAdapter?: boolean;
}
