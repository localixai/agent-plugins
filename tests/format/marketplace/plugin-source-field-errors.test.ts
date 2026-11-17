/**
 * marketplace-reference.md § Plugin sources (`sha`: "a full 40-character lowercase commit SHA"), §
 * archive plugin source, § command plugin source, § Invalid input on a source. The docs say a
 * known type "with a required field missing or of the wrong type" is `Invalid input`; the wording
 * of a field that is present, of the right type and breaks its rule is oracle 2.1.325 (spec
 * J15.8, §15.6.5, §15.6.7; spec/judgments.md), so these tests sit outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { parsePluginSource } from '../../../src/format/marketplace/plugin-source.ts';
import { slotAt } from '../../support/marketplace-entries.ts';

const PATH = 'plugins[0].source';
const SHA = 'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0';
const SHA_FORM = 'Must be a full 40-character lowercase git commit SHA';
const ARCHIVE_URL =
  'Archive URLs must use https:// and must not point at a loopback, link-local, or cloud-metadata host';
const COMMAND_TEXT =
  'command must be printable ASCII (letters, digits, punctuation, single spaces) with no runs of 4 or more spaces';

const COMMAND_LENGTH = 'command must not be longer than the install consent UI can display';

const parse = (raw: unknown) => parsePluginSource(raw, slotAt());
const errorsOf = (raw: unknown) => {
  const result = parse(raw);
  expect(result.source).toBeUndefined();
  return result.issues.map((found) => `${found.path}: ${found.message}`);
};

describe('a git pin names its sha field (oracle 2.1.325)', () => {
  test('a 40-character sha that is not lowercase hex draws the SHA message alone', () => {
    expect(errorsOf({ source: 'github', repo: 'a/b', sha: SHA.toUpperCase() })).toEqual([
      `${PATH}.sha: ${SHA_FORM}`,
    ]);
  });

  test('a sha of another length is also reported as too short or too long', () => {
    expect(errorsOf({ source: 'url', url: 'https://x.example/r.git', sha: 'abc' })).toEqual([
      `${PATH}.sha: Too small: expected string to have exactly 40 characters`,
      `${PATH}.sha: ${SHA_FORM}`,
    ]);
    expect(errorsOf({ source: 'git-subdir', url: 'o/r', path: 'p', sha: `${SHA}0` })).toEqual([
      `${PATH}.sha: Too big: expected string to have exactly 40 characters`,
      `${PATH}.sha: ${SHA_FORM}`,
    ]);
  });

  test('a missing required field wins: only Invalid input on the source', () => {
    expect(errorsOf({ source: 'github', sha: 'x' })).toEqual([`${PATH}: Invalid input`]);
  });
});

describe('an archive source names its url and sha256 fields (oracle 2.1.325)', () => {
  test('a sha256 that is not 64 hex characters', () => {
    const raw = { source: 'archive', url: 'https://a.example/f.zip', sha256: 'xyz' };
    expect(errorsOf(raw)).toEqual([`${PATH}.sha256: Must be a 64-character hex SHA-256 digest`]);
  });

  test('a url that is not https, or points at a loopback, link-local or metadata host', () => {
    for (const url of [
      'http://e.example/a.zip',
      'ftp://e.example/a.zip',
      'https://127.0.0.1/a.zip',
      'https://localhost/a.zip',
      'https://[::1]/a.zip',
      'https://169.254.169.254/a.zip',
      'https://metadata.google.internal/a.zip',
    ]) {
      expect(errorsOf({ source: 'archive', url })).toEqual([`${PATH}.url: ${ARCHIVE_URL}`]);
    }
  });

  test('a url that does not parse draws Invalid URL beside the https message (§15.6.5)', () => {
    for (const url of ['not a url', 'http://', 'https://', '//e.example/a.zip']) {
      expect(errorsOf({ source: 'archive', url })).toEqual([
        `${PATH}.url: Invalid URL`,
        `${PATH}.url: ${ARCHIVE_URL}`,
      ]);
    }
  });

  test('the scheme is read in any case, and a private address is allowed', () => {
    for (const url of ['HTTPS://e.example/a.zip', 'https://10.0.0.1/a.zip']) {
      expect(parse({ source: 'archive', url }).source).toEqual({ kind: 'archive', url });
    }
  });
});

describe('a command source names its command and timeout fields (oracle 2.1.325, J15.8)', () => {
  test('a timeout out of range is too small or too big', () => {
    expect(errorsOf({ source: 'command', command: 'x', timeout: 0 })).toEqual([
      `${PATH}.timeout: Too small: expected number to be >0`,
    ]);
    expect(errorsOf({ source: 'command', command: 'x', timeout: -1 })).toEqual([
      `${PATH}.timeout: Too small: expected number to be >0`,
    ]);
    expect(errorsOf({ source: 'command', command: 'x', timeout: 601 })).toEqual([
      `${PATH}.timeout: Too big: expected number to be <=600`,
    ]);
  });

  test('a timeout that is not a whole number, or a mode that is not copy or link, is Invalid input', () => {
    expect(errorsOf({ source: 'command', command: 'x', timeout: 1.5 })).toEqual([
      `${PATH}: Invalid input`,
    ]);
    expect(errorsOf({ source: 'command', command: 'x', mode: 'other' })).toEqual([
      `${PATH}: Invalid input`,
    ]);
  });

  test('non-ASCII text or a run of four spaces draws the one command message', () => {
    for (const command of ['my-tool é', 'a    b', 'run\tit']) {
      expect(errorsOf({ source: 'command', command })).toEqual([
        `${PATH}.command: ${COMMAND_TEXT}`,
      ]);
    }
  });

  test('over 500 characters draws the length message, before the text message when both break', () => {
    expect(errorsOf({ source: 'command', command: 'a'.repeat(501) })).toEqual([
      `${PATH}.command: ${COMMAND_LENGTH}`,
    ]);
    expect(errorsOf({ source: 'command', command: `${'a'.repeat(497)}    b` })).toEqual([
      `${PATH}.command: ${COMMAND_LENGTH}`,
      `${PATH}.command: ${COMMAND_TEXT}`,
    ]);
  });

  test('both fields are reported together', () => {
    expect(errorsOf({ source: 'command', command: 'aé', timeout: 0 })).toEqual([
      `${PATH}.command: ${COMMAND_TEXT}`,
      `${PATH}.timeout: Too small: expected number to be >0`,
    ]);
  });
});

describe('the bare Invalid input on a source (oracle 2.1.325, J15.8, §15.6.7)', () => {
  test('no source, an unknown type, a required field missing or mistyped, an npm package with ..', () => {
    for (const raw of [
      {},
      { source: 'bogus' },
      { source: 'github' },
      { source: 'github', repo: 1 },
      { source: 'url' },
      { source: 'git-subdir', url: 'o/r' },
      { source: 'npm' },
      { source: 'archive' },
      { source: 'command' },
      { source: 'npm', package: '../x' },
    ]) {
      expect([raw, errorsOf(raw)]).toEqual([raw, [`${PATH}: Invalid input`]]);
    }
  });
});
