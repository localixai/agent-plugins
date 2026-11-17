/**
 * permission-modes.md § Skip all checks with bypassPermissions mode ("Reading another organization's
 * public artifact needs your approval, and this mode doesn't ask for it, so Claude can't read one.
 * The Remove-Item in PowerShell denies also apply in this mode"), § Remove-Item in PowerShell (a
 * system path or a wildcard is denied "in every mode, without asking you", "before the classifier
 * sees it"; the `cmd` built-ins `rd`, `rmdir`, `del` and `erase` on a system path; a variable after
 * literal text counts as empty; a trailing wildcard counts as the folder it empties;
 * `CLAUDE_CODE_DISABLE_POWERSHELL_CMD_RM_DENY=1` turns the `cmd` check off) and artifacts.md § Read
 * an artifact shared with you ("Claude Code asks you first, even in auto mode").
 */
import { describe, expect, test } from 'bun:test';
import type { PermissionMode } from '../../src/protocol/official.ts';
import type { DecideContext } from '../../src/rules/index.ts';
import { allow, ask, decision, deny, pathsWith } from '../support/decide.ts';
import { spec } from '../support/spec.ts';

const match = pathsWith({
  platform: 'windows',
  cwd: 'C:\\work\\app',
  primaryWorkingDirectory: 'C:\\work\\app',
  home: 'C:\\Users\\alice',
});

const powershell = (
  mode: PermissionMode,
  command: string,
  overrides: Partial<DecideContext> = {},
) => decision(mode, 'PowerShell', { command }, { match, autoAvailable: true, ...overrides });

const DENIED = deny('powershell-removal');
const ALL_MODES: readonly PermissionMode[] = [
  'default',
  'acceptEdits',
  'auto',
  'dontAsk',
  'bypassPermissions',
];

const SHARED = 'https://claude.ai/artifact/0123';
const artifactRead = (mode: PermissionMode, foreign: boolean) =>
  decision(
    mode,
    'Artifact',
    { action: 'read', url: SHARED },
    { autoAvailable: true, foreignArtifact: (url) => foreign && url === SHARED },
  );

spec('permissions.modes.Skip all checks with bypassPermissions mode', () => {
  describe('the Remove-Item denies apply in bypassPermissions', () => {
    test('Remove-Item on a system path or the home directory is denied', () => {
      for (const command of [
        'Remove-Item C:\\',
        'Remove-Item C:\\Windows -Recurse',
        'ri C:\\Users\\alice',
      ]) {
        expect(powershell('bypassPermissions', command)).toEqual(DENIED);
      }
    });

    test('Remove-Item on a wildcard is denied: a bare *, a target ending in /* or \\*, a glob under a variable', () => {
      for (const command of [
        'Remove-Item *',
        'Remove-Item .\\build\\*',
        'Remove-Item -Path ./dist/* -Force',
        'Remove-Item $dir/*',
        'Remove-Item "$env:TEMP\\*" -Recurse',
      ]) {
        expect(powershell('bypassPermissions', command)).toEqual(DENIED);
      }
    });

    test('cmd rd, rmdir, del and erase on a system path are denied', () => {
      for (const command of [
        'cmd /c rd /s /q C:\\Users',
        'cmd.exe /C rmdir /s C:\\Windows',
        'cmd /c del /q C:\\*',
        'cmd /c "erase /q C:\\Windows"',
        'cmd /c rd /s /q "C:\\$name"',
      ]) {
        expect(powershell('bypassPermissions', command)).toEqual(DENIED);
      }
    });

    test('an ordinary removal still runs, and the working directory with -Recurse is not checked', () => {
      expect(powershell('bypassPermissions', 'Remove-Item .\\build.log')).toEqual(
        allow('mode-bypass'),
      );
      expect(powershell('bypassPermissions', 'cmd /c del /q dist\\*')).toEqual(
        allow('mode-bypass'),
      );
      expect(powershell('bypassPermissions', 'Remove-Item . -Recurse')).toEqual(
        allow('mode-bypass'),
      );
    });

    test('CLAUDE_CODE_DISABLE_POWERSHELL_CMD_RM_DENY turns off the cmd case only', () => {
      const off = { powershellCmdRemovalDenyOff: true };
      expect(powershell('bypassPermissions', 'cmd /c rd /s /q C:\\Users', off)).toEqual(
        allow('mode-bypass'),
      );
      expect(powershell('bypassPermissions', 'Remove-Item C:\\Users', off)).toEqual(DENIED);
    });
  });

  describe("another organization's public artifact", () => {
    test('bypassPermissions never asks, so the read does not happen', () => {
      expect(artifactRead('bypassPermissions', true)).toEqual(deny('foreign-artifact'));
    });

    test('the same read of an artifact of the own organization runs', () => {
      expect(artifactRead('bypassPermissions', false)).toEqual(allow('mode-bypass'));
    });
  });
});

spec('permissions.mode.auto', () => {
  test('the Remove-Item denies hold in every mode, before any classifier', () => {
    for (const mode of ALL_MODES) {
      expect(powershell(mode, 'Remove-Item C:\\Windows')).toEqual(DENIED);
      expect(powershell(mode, 'Remove-Item *')).toEqual(DENIED);
    }
  });

  test("a read of another organization's public artifact asks even in auto mode", () => {
    expect(artifactRead('auto', true)).toEqual(ask('foreign-artifact'));
    expect(artifactRead('default', true)).toEqual(ask('foreign-artifact'));
    expect(artifactRead('dontAsk', true)).toEqual(deny('foreign-artifact'));
  });
});
