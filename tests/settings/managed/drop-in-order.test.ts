/**
 * managed-settings.md § Split a file-based policy across teams (lists "combine, with duplicates
 * removed"); the order of the combined list is spec 1.1.0 Judgment J14.1, tested outside spec().
 */
import { describe, expect, test } from 'bun:test';
import {
  readFileDocument,
  type DropInFile,
  type ManagedFileRead,
} from '../../../src/settings/managed/index.ts';

const file = (name: string, value: unknown): DropInFile => ({ name, read: { value } });

function split(base: unknown, ...dropIns: DropInFile[]) {
  const read: ManagedFileRead = { base: { value: base }, dropIns: { files: dropIns } };
  return readFileDocument(read);
}

describe('the order of a combined list (Judgment J14.1)', () => {
  test('entries keep the position of their first occurrence in file order; a later duplicate is dropped', () => {
    const merged = split(
      { permissions: { deny: ['Read(./b)', 'Read(./a)'] } },
      file('20-z.json', { permissions: { deny: ['Read(./d)', 'Read(./b)'] } }),
      file('10-y.json', { permissions: { deny: ['Read(./c)', 'Read(./a)'] } }),
    );
    expect(merged.settings.permissions?.deny).toEqual([
      'Read(./b)',
      'Read(./a)',
      'Read(./c)',
      'Read(./d)',
    ]);
  });
});
