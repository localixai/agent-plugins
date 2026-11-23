/**
 * Spec §15.5.3 ("Display fields", the paragraph on invalid entries; Judgment J15.15) and §15.4
 * (`engines`, with §2.4's messages): an entry's display field of the wrong type, or a `homepage`
 * that is not a URL, makes that entry invalid to install while the marketplace and its other
 * entries are unaffected; a malformed `engines` is ignored and the entry installs.
 */
import { describe, expect, test } from 'bun:test';
import { invalidEntryMessage } from '../../../src/format/marketplace/entry-display.ts';
import { rawEntry, readOne, slotAt } from '../../support/marketplace-entries.ts';
import { errorsOf } from '../../support/marketplace-fixtures.ts';
import { spec } from '../../support/spec.ts';

const validating = slotAt(0, { mode: 'validate' });

describe('judgment J15.15: a malformed display field makes the entry invalid to install', () => {
  test('the entry is kept at load, without the field, and carries the problems in schema wording', () => {
    const { entry, issues } = readOne(rawEntry({ description: 5, homepage: 'x' }));
    expect(errorsOf(issues)).toEqual([]);
    expect(entry?.description).toBeUndefined();
    expect(entry?.display.homepage).toBeUndefined();
    expect(entry?.invalid).toEqual([
      'description: Invalid input: expected string, received number',
      'homepage: Invalid URL',
    ]);
  });

  test('installing it fails with the problems joined by ", "', () => {
    const { entry } = readOne(rawEntry({ author: 'A', repository: { url: 'x' }, keywords: 'a' }));
    expect(entry === undefined ? undefined : invalidEntryMessage(entry)).toBe(
      "This plugin's marketplace entry is invalid: author: Invalid input: expected object, received string, repository: Invalid input: expected string, received object, keywords: Invalid input: expected array, received string",
    );
    const sound = readOne(rawEntry({ description: 'fine' })).entry;
    expect(sound === undefined ? 'missing' : invalidEntryMessage(sound)).toBeUndefined();
  });

  test('a validator still reports each display problem as an error at the field', () => {
    const { issues } = readOne(rawEntry({ description: 5, homepage: 'x' }), validating);
    expect(errorsOf(issues).map((found) => [found.path, found.message])).toEqual([
      ['plugins[0].description', 'Invalid input: expected string, received number'],
      ['plugins[0].homepage', 'Invalid URL'],
    ]);
  });
});

spec('marketplace.entry.engines', () => {
  test('a valid range is carried on the entry', () => {
    const { entry, issues } = readOne(rawEntry({ engines: { 'claude-code': '>=2.1.400' } }));
    expect(issues).toEqual([]);
    expect(entry?.engines).toEqual({ range: '>=2.1.400' });
  });

  test('a value that is not a range is a validator error at plugins[<i>].engines.claude-code', () => {
    const { issues } = readOne(rawEntry({ engines: { 'claude-code': 'banana' } }), validating);
    expect(issues).toEqual([
      {
        severity: 'error',
        path: 'plugins[0].engines.claude-code',
        message:
          'Must be a version range such as ">=2.1.0"; got "banana". It will be ignored at load time.',
      },
    ]);
  });

  test('an engines that is not an object is a warning at plugins[<i>].engines', () => {
    const { issues } = readOne(rawEntry({ engines: 5 }), validating);
    expect(issues).toEqual([
      {
        severity: 'warning',
        path: 'plugins[0].engines',
        message:
          '\'engines\' must be an object such as { "claude-code": ">=2.1.0" }; got number. It will be ignored at load time.',
      },
    ]);
  });

  test('a malformed engines is ignored at load: the entry installs (J15.15)', () => {
    for (const engines of [5, { 'claude-code': 'banana' }]) {
      const { entry } = readOne(rawEntry({ engines }));
      expect(entry?.name).toBe('my-plugin');
      expect(entry?.invalid).toBeUndefined();
    }
  });
});
