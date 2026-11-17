import type { Conversion, ConversionNote, ConversionOutcome, ConvertedFile } from './types.ts';
import type { PluginForm } from '../loading/core/form.ts';

/** One note of a conversion (Agent Plugins Extended §18.4.1 item 1). */
const note =
  (outcome: ConversionOutcome) =>
  (path: string, field: string | undefined, message: string): ConversionNote => ({
    outcome,
    path,
    ...(field === undefined ? {} : { field }),
    message,
  });

export const stop = note('stopped');
export const notConverted = note('not-converted');
export const reported = note('reported');

/** Which outcomes keep a conversion from writing anything. */
const STOPS: { readonly [O in ConversionOutcome]: boolean } = {
  stopped: true,
  'not-converted': false,
  reported: false,
};

/** The conversion of the parts: no files when any note stops it. */
export function conversion(
  to: PluginForm,
  files: readonly ConvertedFile[],
  notes: readonly ConversionNote[],
  runtime?: string,
): Conversion {
  const stopped = notes.some((found) => STOPS[found.outcome]);
  return {
    to,
    stopped,
    files: stopped ? [] : files,
    notes,
    ...(runtime === undefined || stopped ? {} : { runtime }),
  };
}

/** A JSON file as the converter writes it: two-space indent and a final newline, so output is stable (§18.4.1 item 5). */
export const jsonFile = (path: string, value: unknown): ConvertedFile => ({
  path,
  text: `${JSON.stringify(value, null, 2)}\n`,
});
