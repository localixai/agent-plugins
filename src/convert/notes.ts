import type { Conversion, ConversionNote, ConvertedFile } from './types.ts';
import type { PluginForm } from '../loading/core/form.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/notes.rs`, docs/RUST.md).
 */

const call = coreModule('convert/notes');

export const stop = (path: string, field: string | undefined, message: string): ConversionNote =>
  call('stop', path, field, message);

export const notConverted = (
  path: string,
  field: string | undefined,
  message: string,
): ConversionNote => call('notConverted', path, field, message);

export const reported = (
  path: string,
  field: string | undefined,
  message: string,
): ConversionNote => call('reported', path, field, message);

/** The conversion of the parts: no files when any note stops it. */
export const conversion = (
  to: PluginForm,
  files: readonly ConvertedFile[],
  notes: readonly ConversionNote[],
  runtime?: string,
): Conversion => call('conversion', to, files, notes, runtime);
