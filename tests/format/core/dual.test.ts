/**
 * The manifests of a dual package (Agent Plugins Extended §18.3.1): the same values in the fields
 * both forms define, chosen to load in both; J18.19 for the name, `author.name` and `homepage`. A
 * judgment-style test outside `spec(...)`: the chapter carries no requirement ids.
 */
import { describe, expect, test } from 'bun:test';
import { checkDualManifests } from '../../../src/entry/core.ts';

const SCHEMA = 'https://agent-plugins.org/schemas/1.1.0/plugin.schema.json';
const shared = {
  name: 'release-guard',
  version: '1.0.0',
  description: 'Drafts release notes',
  author: { name: 'Acme' },
  homepage: 'https://example.com/release-guard',
};

describe('§18.3.1: a dual package manifest', () => {
  test('equivalent halves give no finding', () => {
    expect(checkDualManifests({ $schema: SCHEMA, ...shared }, shared)).toEqual([]);
  });

  test('a value that differs between the halves is an error on the field', () => {
    const findings = checkDualManifests(
      { $schema: SCHEMA, ...shared },
      { ...shared, version: '1.0.1' },
    );
    expect(findings).toEqual([
      {
        severity: 'error',
        file: 'plugin.json',
        path: 'version',
        message:
          '"version" differs from .claude-plugin/plugin.json: a dual package gives both the same value',
      },
    ]);
  });

  test('J18.19: a name with a dot loads in both but is not kebab-case', () => {
    const findings = checkDualManifests(
      { $schema: SCHEMA, ...shared, name: 'acme.tools' },
      { ...shared, name: 'acme.tools' },
    );
    expect(findings.map((one) => [one.severity, one.path])).toEqual([['warning', 'name']]);
  });

  test('values only the core accepts: author without name, a homepage that is no URL', () => {
    const core = {
      $schema: SCHEMA,
      ...shared,
      author: { email: 'a@b.c' },
      homepage: 'docs.example.com',
    };
    const findings = checkDualManifests(core, {
      ...shared,
      author: core.author,
      homepage: core.homepage,
    });
    expect(findings.map((one) => [one.severity, one.path])).toEqual([
      ['error', 'author'],
      ['error', 'homepage'],
    ]);
  });

  test('a missing description and a non-SemVer version are warned', () => {
    const { description: _omit, ...rest } = shared;
    const findings = checkDualManifests(
      { $schema: SCHEMA, ...rest, version: 'v1' },
      { ...rest, version: 'v1' },
    );
    expect(findings.map((one) => [one.severity, one.path])).toEqual([
      ['warning', 'description'],
      ['warning', 'version'],
    ]);
  });

  test('$schema and extensions belong in the core manifest only', () => {
    const findings = checkDualManifests(
      { $schema: SCHEMA, ...shared },
      { ...shared, $schema: SCHEMA, extensions: {} },
    );
    expect(findings.map((one) => [one.file, one.path])).toEqual([
      ['.claude-plugin/plugin.json', '$schema'],
      ['.claude-plugin/plugin.json', 'extensions'],
    ]);
  });

  test('Claude Code fields in the core manifest are reported by core §5.2 (unknown, ignored)', () => {
    const findings = checkDualManifests({ $schema: SCHEMA, ...shared, skills: './x' }, shared);
    expect(findings).toEqual([
      {
        severity: 'warning',
        file: 'plugin.json',
        path: 'skills',
        message: 'Unknown field "skills": ignored (core §5.2)',
      },
    ]);
  });
});
