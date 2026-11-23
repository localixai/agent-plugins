import { textPatternOf } from '../bash/text-pattern.ts';
import { isWrapperName } from '../bash/wrappers.ts';
import type { PermissionRule } from '../types.ts';
import { isAllowRule } from './behavior.ts';

/**
 * permission-modes.md, "How the classifier evaluates actions": "On entering auto mode, broad allow
 * rules that grant arbitrary code execution are dropped": a blanket `Bash(*)` or `PowerShell(*)`,
 * wildcarded interpreters like `Bash(python*)`, package-manager run commands, `Agent` allow rules
 * and `Monitor` allow rules; from 2.1.330 also a wrapper followed only by a wildcard, a shell rule
 * that starts with a wildcard and some wildcarded paths (Agent Plugins Extended §13.9.4 item 3). "Narrow rules like `Bash(npm test)` stay in effect", and the dropped
 * rules come back when auto mode is left (they are only left out of this view of the rules).
 */

/** A bare rule (`Bash(*)` reads as bare; `PowerShell(*)` is the pattern `*`, handled below) on one of the tools whose every use runs code. */
const BARE_TOOLS: ReadonlySet<string> = new Set(['Bash', 'PowerShell', 'Agent', 'Monitor']);

/**
 * The docs name `python`; the others are the same kind of interpreter, and the shell wrappers
 * `bash`, `sh` and `zsh` (with their `-c` forms) are the "shell-wrapper prefixes" of
 * settings-reference.md, `autoMode.classifyAllShell` (spec/judgments.md).
 */
const INTERPRETERS: ReadonlySet<string> = new Set([
  'python',
  'python3',
  'node',
  'ruby',
  'perl',
  'bash',
  'sh',
  'zsh',
]);

/** The docs say "package-manager run commands" without naming the managers (spec/judgments.md). */
const PACKAGE_RUN = /^(?:npm|yarn|pnpm|bun) run\b/;

const SHELL_TOOLS: ReadonlySet<string> = new Set(['Bash', 'PowerShell']);

/** 2.1.330: "a wrapper followed only by a wildcard (`Bash(timeout *)`)", the legacy `:*` included. */
function bareWrapper(pattern: string): boolean {
  const parts = /^(\S+?)(?::\*| \*)$/.exec(pattern);
  return parts !== null && isWrapperName(parts[1] ?? '');
}

/**
 * 2.1.330: "some wildcarded paths (`Bash($HOME/bin/*)`, `PowerShell(C:\tools\x.exe *)`)". Judgment
 * (spec/judgments.md): a rule whose program is an absolute or home-anchored path (`/`, `~`, `$HOME`,
 * a drive letter) and that holds a wildcard; a relative path stays.
 */
const ANCHORED_PATH = /^(?:\/|~|\$HOME\b|\$\{HOME\}|[A-Za-z]:[\\/])/;

const wildcardedPath = (pattern: string): boolean =>
  pattern.includes('*') && ANCHORED_PATH.test(pattern);

/** Whether a Bash or PowerShell allow pattern is one auto mode drops (J13.24, §13.9.4 item 3). */
const broadShellPattern = (pattern: string): boolean =>
  pattern === '*' ||
  pattern.startsWith('*') ||
  wildcardedInterpreter(pattern) ||
  PACKAGE_RUN.test(pattern) ||
  bareWrapper(pattern) ||
  wildcardedPath(pattern);

function wildcardedInterpreter(pattern: string): boolean {
  const program = pattern.split(' ')[0]?.replace(/:?\*+$/, '') ?? '';
  return pattern.includes('*') && INTERPRETERS.has(program);
}

/**
 * Whether `rule` is an allow rule auto mode drops. With `autoMode.classifyAllShell` "auto mode suspends
 * every shell allow rule" as well (settings-reference.md), `Bash(npm test)` included.
 */
export function isDroppedInAuto(rule: PermissionRule, classifyAllShell = false): boolean {
  if (!isAllowRule(rule)) return false;
  const tool = rule.value.toolName;
  if (classifyAllShell && SHELL_TOOLS.has(tool)) return true;
  if (rule.specifier === undefined) return BARE_TOOLS.has(tool);
  const pattern = textPatternOf(rule);
  return SHELL_TOOLS.has(tool) && pattern !== undefined && broadShellPattern(pattern);
}
