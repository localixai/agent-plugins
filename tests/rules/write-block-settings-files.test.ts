/**
 * settings-reference.md § `permissions.blockWritesOutsideWorkingDirectories` and § What the write
 * block refuses and what stays writable; errors.md § Setting blocks writes outside the working
 * directories and § Writes to Claude Code settings files are blocked (the refusals verbatim, and the
 * files beyond "your settings files": "A file you passed to `--agents` or `--mcp-config`", "The
 * stored copy of server-managed settings and the files kept with it", "Any `.json` file, while
 * Claude Code can't work out where the `managed-settings.d/` folder leads"); permission-modes.md §
 * Protected paths (`remote-settings.json` "and the signature, sync, and consent files Claude Code
 * keeps with it, in `~/.claude` or the directory `CLAUDE_CONFIG_DIR` points to"). Which kept files
 * the configuration directory holds, and how `<dirs>` is joined, are judgments (spec/judgments.md).
 */
import { describe, expect, test } from 'bun:test';
import { writeBlockRefusal, writeBlockShortRefusal } from '../../src/rules/index.ts';
import { deny, onPath } from '../support/decide.ts';
import { spec } from '../support/spec.ts';

const WRITES = { blockWritesOutsideWorkingDirectories: true } as const;
const CONFIG = '/srv/claude-config';
const launched = {
  protectedPaths: {
    files: ['/work/app/agents.json', '/work/app/mcp.json'],
    configDirectory: CONFIG,
    remoteSettingsFiles: ['/var/cache/claude/remote.sig'],
  },
};

const write = (path: string, extra: Record<string, unknown> = {}) =>
  onPath('acceptEdits', 'Write', path, { ...WRITES, ...launched, ...extra });

spec('permissions.reference.What the write block refuses and what stays writable', () => {
  describe('the files beyond the named settings files are settings files too', () => {
    test('a file passed to --agents or --mcp-config, inside the working directories', () => {
      expect(write('/work/app/agents.json')).toEqual(deny('settings-file'));
      expect(write('/work/app/mcp.json')).toEqual(deny('settings-file'));
    });

    test('the stored server-managed settings and the files kept with it', () => {
      expect(write(`${CONFIG}/remote-settings.json`)).toEqual(deny('settings-file'));
      expect(write('/var/cache/claude/remote.sig')).toEqual(deny('settings-file'));
    });

    test('any .json file while the managed-settings.d folder cannot be resolved', () => {
      const unresolved = { managedSettingsFolderUnresolved: true };
      expect(write('/work/app/src/data.json', unresolved)).toEqual(deny('settings-file'));
      expect(write('/work/app/src/data.json')).not.toEqual(deny('settings-file'));
      expect(write('/work/app/src/data.ts', unresolved)).not.toEqual(deny('settings-file'));
    });
  });

  test('the settings-file refusal, verbatim', () => {
    expect(
      writeBlockRefusal('settings-file', {
        path: '/work/app/.claude/settings.json',
        workingDirectories: ['/work/app'],
      }),
    ).toBe(
      '/work/app/.claude/settings.json: Writes to Claude Code settings files are blocked while permissions.blockWritesOutsideWorkingDirectories is on, because a settings file can lift that block. While a managed settings folder cannot be resolved, any .json file counts as one.',
    );
  });
});

spec(
  'permissions.behavior.Block file-tool reads and writes outside the working directories',
  () => {
    test('the refusal of a write outside the working directories, and its other wording', () => {
      expect(
        writeBlockRefusal('outside-working-directory', {
          path: '/home/alice/notes.txt',
          workingDirectories: ['/work/app'],
        }),
      ).toBe(
        '/home/alice/notes.txt is outside /work/app; the permissions.blockWritesOutsideWorkingDirectories setting blocks writes outside the working directories. Ask the user to add the directory with /add-dir, or to remove that setting.',
      );
      expect(writeBlockShortRefusal('/home/alice/notes.txt')).toBe(
        '/home/alice/notes.txt: Writes outside the working directories are blocked (permissions.blockWritesOutsideWorkingDirectories). Add the directory with /add-dir, or remove that setting.',
      );
    });
  },
);

spec('permissions.modes.Protected paths.remote-settings.json', () => {
  const protectedPaths = {
    configDirectory: CONFIG,
    remoteSettingsFiles: ['/var/cache/claude/remote.sig'],
  };
  const edit = (path: string) => onPath('acceptEdits', 'Write', path, { protectedPaths });

  test('remote-settings.json and the files the host names as kept with it are protected', () => {
    expect(edit(`${CONFIG}/remote-settings.json`).reason).toBe('protected-path');
    expect(edit('/var/cache/claude/remote.sig').reason).toBe('protected-path');
  });

  test('a file of another name there is not one of them', () => {
    expect(edit(`${CONFIG}/notes.txt`).reason).not.toBe('protected-path');
  });
});

describe('judgment: the files kept with remote-settings.json (Agent Plugins Extended J13.35)', () => {
  const protectedPaths = { configDirectory: CONFIG };
  test('are the ones the host names: no name is guessed, a remote-settings prefix included', () => {
    const edit = onPath('acceptEdits', 'Write', `${CONFIG}/remote-settings.sig`, {
      protectedPaths,
    });
    expect(edit.reason).not.toBe('protected-path');
    expect(write(`${CONFIG}/remote-settings-consent.json`)).not.toEqual(deny('settings-file'));
  });
});

describe('judgment: several working directories in the refusal', () => {
  test('are joined with a comma and a space', () => {
    expect(
      writeBlockRefusal('outside-working-directory', {
        path: '/tmp/x',
        workingDirectories: ['/work/app', '/work/lib'],
      }),
    ).toContain('/tmp/x is outside /work/app, /work/lib;');
  });
});
