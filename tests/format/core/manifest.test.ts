/**
 * The core manifest, root `plugin.json` (Agent Plugins 1.1.0, `spec/1.1.0.md`): core §5.2 (closed
 * object, unknown top-level field reported and ignored, every other violation fatal, `$schema`
 * selects the rules), §5.3 (required `$schema` and `name`), §5.4 (metadata typed by JSON type only;
 * `author` closed), §5.5 (name constraints), §5.6 and §8.1 (extensions). Claude Code has no core
 * packages, so these are judgment-style tests outside `spec(...)` (docs/SPEC.md, "The specification").
 */
import { describe, expect, test } from 'bun:test';
import { coreNameProblems, readCoreManifest } from '../../../src/entry/core.ts';

const SCHEMA = 'https://agent-plugins.org/schemas/1.1.0/plugin.schema.json';
const read = (extra: Record<string, unknown>, implemented?: readonly string[]) =>
  readCoreManifest({ $schema: SCHEMA, name: 'my-plugin', ...extra }, implemented);

describe('core §5.2: a closed manifest', () => {
  test('the minimal manifest loads with no issue and targets 1.1.0', () => {
    const result = read({});
    expect(result.manifest).toEqual({ $schema: SCHEMA, name: 'my-plugin' });
    expect(result.version).toBe('1.1.0');
    expect(result.issues).toEqual([]);
  });

  test('the full manifest of the example loads', () => {
    const full = {
      displayName: 'Plugin Name',
      version: '1.2.0',
      description: 'Brief plugin description',
      author: { name: 'Author Name', email: 'author@example.com', url: 'https://example.com' },
      homepage: 'https://docs.example.com/plugin',
      repository: 'https://github.com/example/plugin',
      license: 'MIT',
      keywords: ['keyword1', 'keyword2'],
    };
    expect(read(full).manifest).toEqual({ $schema: SCHEMA, name: 'my-plugin', ...full });
  });

  test('an unknown top-level field is reported, ignored, and the package still loads', () => {
    const result = read({ skills: './skills', hooks: {} });
    expect(result.manifest).toEqual({ $schema: SCHEMA, name: 'my-plugin' });
    expect(result.issues.map((one) => [one.severity, one.path])).toEqual([
      ['warning', 'skills'],
      ['warning', 'hooks'],
    ]);
  });

  test('any other violation is fatal: a field of the wrong type rejects the package', () => {
    const result = read({ description: 42 });
    expect(result.manifest).toBeUndefined();
    expect(result.issues).toEqual([
      {
        severity: 'error',
        path: 'description',
        message: 'Invalid "description": must be a string',
      },
    ]);
  });

  test('a manifest that is not an object is rejected', () => {
    expect(readCoreManifest([]).manifest).toBeUndefined();
  });
});

describe('core §5.2: $schema selects the rules', () => {
  test('a missing $schema rejects the package', () => {
    const result = readCoreManifest({ name: 'my-plugin' });
    expect(result.manifest).toBeUndefined();
    expect(result.issues[0]?.path).toBe('$schema');
  });

  test('a $schema that is no canonical identifier rejects the package', () => {
    const result = readCoreManifest({ $schema: 'https://example.com/plugin.json', name: 'x' });
    expect(result.manifest).toBeUndefined();
  });

  test('a canonical identifier of a version the kit does not implement is rejected and reported', () => {
    const result = readCoreManifest({
      $schema: 'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json',
      name: 'my-plugin',
    });
    expect(result.manifest).toBeUndefined();
    expect(result.version).toBeUndefined();
    expect(result.issues[0]?.message).toBe(
      'Unsupported Agent Plugins version 1.0.0: this client implements 1.1.0',
    );
  });

  test('the identifier of another document is not a manifest identifier', () => {
    const result = readCoreManifest({
      $schema: 'https://agent-plugins.org/schemas/1.1.0/mcp.schema.json',
      name: 'x',
    });
    expect(result.manifest).toBeUndefined();
  });
});

describe('core §5.3: required fields', () => {
  test('a missing name rejects the package and names the field', () => {
    const result = readCoreManifest({ $schema: SCHEMA });
    expect(result.manifest).toBeUndefined();
    expect(result.issues).toEqual([
      { severity: 'error', path: 'name', message: 'The core manifest has no "name"' },
    ]);
  });

  test('an empty or non-string name rejects the package', () => {
    expect(read({ name: '' }).manifest).toBeUndefined();
    expect(read({ name: 7 }).manifest).toBeUndefined();
  });
});

describe('core §5.4: metadata is typed by JSON type only', () => {
  test('a version that is not SemVer, a homepage that is no URL and an odd email load', () => {
    const result = read({
      version: 'latest',
      homepage: 'docs.example.com',
      repository: 'somewhere',
      license: 'not-spdx',
      author: { email: 'not an email' },
    });
    expect(result.issues).toEqual([]);
    expect(result.manifest?.author).toEqual({ email: 'not an email' });
  });

  test('author may hold only name, email and url, each a string', () => {
    expect(read({ author: { name: 'A', twitter: '@a' } }).manifest).toBeUndefined();
    expect(read({ author: { name: 1 } }).manifest).toBeUndefined();
    expect(read({ author: 'A' }).manifest).toBeUndefined();
  });

  test('keywords is an array of strings', () => {
    expect(read({ keywords: ['a', 1] }).manifest).toBeUndefined();
    expect(read({ keywords: ['a'] }).manifest?.keywords).toEqual(['a']);
  });
});

describe('core §5.5: plugin name constraints', () => {
  test('the valid names of the specification pass', () => {
    for (const name of ['my-plugin', 'acme.tools', 'lint3r', 'a', 'x'.repeat(64)]) {
      expect(coreNameProblems(name)).toEqual([]);
    }
  });

  test('the invalid names of the specification fail, each for its reason', () => {
    expect(coreNameProblems('My-Plugin')).toEqual([
      'may hold only lowercase letters, digits, "-" and "."',
    ]);
    expect(coreNameProblems('-start')).toEqual(['must start and end with a letter or a digit']);
    expect(coreNameProblems('has--double')).toEqual(['must not hold "--"']);
    expect(coreNameProblems('too.many..dots')).toEqual(['must not hold ".."']);
    expect(coreNameProblems('')).toEqual(['must be 1 to 64 characters long']);
    expect(coreNameProblems('x'.repeat(65))).toEqual(['must be 1 to 64 characters long']);
    expect(coreNameProblems('end.')).toEqual(['must start and end with a letter or a digit']);
  });

  test('a manifest with an invalid name is rejected', () => {
    expect(read({ name: 'My-Plugin' }).manifest).toBeUndefined();
  });
});

describe('core §5.6, §8.1: extensions', () => {
  test('extensions must be an object of objects, else the manifest is invalid', () => {
    expect(read({ extensions: [] }).manifest).toBeUndefined();
    expect(read({ extensions: { 'com.example.client': true } }).manifest).toBeUndefined();
  });

  test('namespaces the host does not implement are dropped without looking at their values', () => {
    const extensions = {
      'com.example.client': { setting: true },
      'org.other.client': { anything: [1, { deep: null }] },
    };
    expect(read({ extensions }).manifest?.extensions).toBeUndefined();
    expect(read({ extensions }, ['com.example.client']).manifest?.extensions).toEqual({
      'com.example.client': { setting: true },
    });
  });
});
