/**
 * Runs a converted package's hook adapter as a client would (Agent Plugins Extended §18.4.4): the
 * package written to a temporary folder this helper makes, the adapter started with `node` and the
 * input on stdin. Tests that use it skip where `node` is not on `PATH`.
 */
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

export const NODE = Bun.which('node');

/** Writes the tree into a new temporary folder and returns its path. */
export function writeTree(tree: Readonly<Record<string, string>>): string {
  const root = mkdtempSync(join(tmpdir(), 'agent-plugins-convert-'));
  for (const [path, text] of Object.entries(tree)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
}

export interface AdapterRun {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
}

/** Runs `node <args>` in `cwd` with `input` as JSON on stdin and `env` over a minimal environment. */
export function runNode(
  args: readonly string[],
  input: unknown,
  options: { readonly cwd: string; readonly env: Readonly<Record<string, string>> },
): AdapterRun {
  const run = Bun.spawnSync([NODE ?? 'node', ...args], {
    cwd: options.cwd,
    env: { PATH: process.env.PATH ?? '', ...options.env },
    stdin: new TextEncoder().encode(typeof input === 'string' ? input : JSON.stringify(input)),
  });
  return {
    status: run.exitCode,
    stdout: run.stdout.toString(),
    stderr: run.stderr.toString(),
  };
}

/** The `args` of the one hook a hooks file of a conversion registers under `event`. */
export function hookArgs(hooks: unknown, event: string): readonly string[] {
  const text = JSON.stringify(hooks);
  const parsed: { hooks: Record<string, { hooks?: { args?: string[] }[]; args?: string[] }[]> } =
    JSON.parse(text);
  const first = parsed.hooks[event]?.[0];
  const args = first?.hooks?.[0]?.args ?? first?.args;
  if (args === undefined) throw new Error(`no hook with args under ${event}`);
  return args;
}

/** Echoes what it received as context: its input, working directory and the core variables. */
export const ECHO = [
  "let text = ''",
  "process.stdin.on('data', (chunk) => (text += chunk))",
  "process.stdin.on('end', () => {",
  '  const seen = { input: JSON.parse(text), cwd: process.cwd(), root: process.env.PLUGIN_ROOT, data: process.env.PLUGIN_DATA }',
  '  process.stdout.write(JSON.stringify({ additional_context: JSON.stringify(seen) }))',
  '})',
  '',
].join('\n');
/** Denies with its tool profile as the reason, so a PreToolUse test sees the profile. */
export const PROFILE = [
  "let text = ''",
  "process.stdin.on('data', (chunk) => (text += chunk))",
  "process.stdin.on('end', () => {",
  '  const reason = JSON.stringify(JSON.parse(text).tool_profile ?? null)',
  "  process.stdout.write(JSON.stringify({ decision: 'deny', reason }))",
  '})',
  '',
].join('\n');
/** Writes its first argument and exits with its second. */
export const ANSWER = [
  "process.stdin.on('data', () => {})",
  "process.stdin.on('end', () => {",
  '  process.stdout.write(process.argv[2] ?? "")',
  "  process.stderr.write(process.argv[4] ?? '')",
  '  process.exitCode = Number(process.argv[3] ?? 0)',
  '})',
  '',
].join('\n');
