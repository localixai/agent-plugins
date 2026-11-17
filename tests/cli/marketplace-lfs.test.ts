/**
 * Spec §14.8.4, the `git` row of the marketplace source types (plugins/host-marketplace.md, "Keep
 * plugin files out of Git LFS": "The clone never downloads LFS content, so LFS-tracked files arrive
 * as pointer files"): "the add or update output reports how many". The line's wording is outside
 * `spec(...)` and in spec/judgments.md.
 */
import { describe, expect, test } from 'bun:test';
import { lfsPointerLine } from '../../src/cli/lfs-pointer-line.ts';
import { concluded, fetched, stdoutOf } from '../support/marketplace-add.ts';
import { spec } from '../support/spec.ts';

spec('policy.reference.Marketplace source types.git', () => {
  test('marketplace add reports how many LFS files arrived as pointer files', () => {
    const done = concluded({ source: 'acme/tools' }, fetched('acme-tools', { lfsPointers: 3 }));
    const out = stdoutOf(done.output);
    expect(out[0]).toStartWith('Successfully added marketplace: acme-tools');
    expect(out.join('\n')).toContain('3 Git LFS files');
  });

  test('with no pointer file nothing is added to the output', () => {
    const done = concluded({ source: 'acme/tools' }, fetched('acme-tools', { lfsPointers: 0 }));
    expect(stdoutOf(done.output)).toEqual([
      'Successfully added marketplace: acme-tools (declared in user settings)',
    ]);
    expect(lfsPointerLine(0)).toBeUndefined();
  });
});

describe('the pointer-file line (judgment)', () => {
  test('one and several', () => {
    expect(lfsPointerLine(1)).toBe(
      '1 Git LFS file arrived as a pointer file: the clone never downloads LFS content',
    );
    expect(lfsPointerLine(2)).toBe(
      '2 Git LFS files arrived as pointer files: the clone never downloads LFS content',
    );
  });
});
