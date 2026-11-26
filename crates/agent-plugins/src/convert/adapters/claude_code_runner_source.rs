//! Port of `src/convert/adapters/claude-code-runner-source.ts`, switched.
//!
//! The body of `run-claude-code-handler.mjs` (Agent Plugins Extended §18.4.4): a Claude Code `command`
//! hook run under a core client. Its input is built from the core input by §18.2, leaving out what the
//! core does not supply; `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA` and `CLAUDE_PROJECT_DIR` (the core
//! input's `cwd`) are set; the hook runs in the core working directory, in exec form with the three
//! placeholders substituted per argument, or in shell form (`bash` through `$SHELL`, `pwsh` with the
//! placeholders as `$env:` references). Exit 2 becomes the block of its event, plain stdout context on
//! `SessionStart` and `UserPromptSubmit`; what the core cannot express is written to stderr, and the
//! adapter always exits 0 with one valid result or nothing (item 4). Judgment (spec/judgments.md): a
//! denial or a Stop block without a reason gets the reason `Blocked by a hook`, since the core's
//! closed shapes require one.

/// The text of the file the adapter writes.
pub const CLAUDE_CODE_RUNNER_SOURCE: &str = r#"const isObject = (value) => typeof value === 'object' && value !== null && !Array.isArray(value);
const record = (value) => (isObject(value) ? value : {});
const filled = (value) => typeof value === 'string' && value.trim() !== '';

// The adapter's own flags, before -- and the Claude Code handler's command and arguments.
function options(argv) {
  const end = argv.indexOf('--');
  const flags = end < 0 ? [] : argv.slice(0, end);
  const value = (name) => (flags.includes(name) ? flags[flags.indexOf(name) + 1] : undefined);
  const timeout = Number(value('--timeout'));
  return {
    event: value('--event'),
    profile: value('--profile'),
    timeout: Number.isFinite(timeout) && timeout > 0 ? timeout * 1000 : undefined,
    shell: value('--shell') ?? 'bash',
    exec: flags.includes('--exec'),
    command: end < 0 ? undefined : argv[end + 1],
    args: end < 0 ? [] : argv.slice(end + 2),
  };
}

const given = options(proc.argv.slice(2));
const { event } = given;

// The Claude Code input of a core occurrence: no field the core does not supply (\u00A718.4.4).
const tool = (core) => ({
  tool_name: core.tool_name,
  tool_input: core.tool_input,
  tool_use_id: core.tool_use_id,
});
const INPUTS = {
  SessionStart: () => ({}),
  UserPromptSubmit: (core) => ({ prompt: core.prompt }),
  PreToolUse: tool,
  PostToolUse: (core) => ({ ...tool(core), tool_response: core.tool_result }),
  Stop: (core) => ({ stop_hook_active: core.continuation_active === true }),
};

const DEFAULT_REASON = 'Blocked by a hook';
const deny = (reason) => ({ decision: 'deny', reason: filled(reason) ? reason : DEFAULT_REASON });
const proceed = (reason) => ({
  decision: 'continue',
  reason: filled(reason) ? reason : DEFAULT_REASON,
});
const context = (value) => (filled(value) ? { additional_context: value } : undefined);
const specific = (output) => record(output.hookSpecificOutput);

// Exit 2: the block of each event, stderr as its message (\u00A718.4.4 item 3).
const BLOCKS = {
  SessionStart: () => undefined,
  UserPromptSubmit: deny,
  PreToolUse: deny,
  PostToolUse: context,
  Stop: proceed,
};

// Exit 0 with a JSON object: each result the core can express, by \u00A718.2.
const RESULTS = {
  SessionStart: (output) => context(specific(output).additionalContext),
  UserPromptSubmit: (output) =>
    output.decision === 'block' ? deny(output.reason) : context(specific(output).additionalContext),
  PreToolUse: (output) => {
    const hook = specific(output);
    if (hook.permissionDecision === 'deny') return deny(hook.permissionDecisionReason);
    if (hook.permissionDecision === 'allow')
      return hook.updatedInput === undefined ? { decision: 'allow' } : undefined;
    if (hook.permissionDecision !== undefined) return undefined;
    if (output.decision === 'block') return deny(output.reason);
    return output.decision === 'approve' ? { decision: 'allow' } : undefined;
  },
  PostToolUse: (output) => context(specific(output).additionalContext),
  Stop: (output) => (output.decision === 'block' ? proceed(output.reason) : undefined),
};

// Plain stdout on exit 0 is context on these two events, as Claude Code reads it.
const PLAIN = { SessionStart: true, UserPromptSubmit: true };

const VARIABLES = /\$\{(CLAUDE_PLUGIN_ROOT|CLAUDE_PLUGIN_DATA|CLAUDE_PROJECT_DIR)\}/g;

function launch(environment) {
  const { command, args, exec, shell } = given;
  if (exec) {
    const substitute = (value) => value.replace(VARIABLES, (_match, name) => environment[name]);
    return [substitute(command), args.map(substitute)];
  }
  if (shell === 'powershell') {
    const script = command.replace(VARIABLES, (_match, name) => '$env:' + name);
    return ['pwsh', ['-NoProfile', '-Command', script]];
  }
  return [proc.env.SHELL || 'bash', ['-c', command]];
}

function note(message) {
  proc.stderr.write('Claude Code hook [' + given.command + '] ' + message + '\n');
}

function parsed(text) {
  try {
    return { value: JSON.parse(text) };
  } catch {
    return undefined;
  }
}

function resultOf(run) {
  if (run.error !== undefined) return note('did not start: ' + run.error.message);
  if (run.status === 2) return BLOCKS[event](run.stderr.trim());
  if (run.status !== 0) return note('failed with status ' + run.status + ': ' + run.stderr.trim());
  const stdout = run.stdout.trim();
  if (stdout === '') return undefined;
  const output = stdout.startsWith('{') ? parsed(stdout) : undefined;
  if (output !== undefined && isObject(output.value)) {
    const result = RESULTS[event](output.value);
    if (result === undefined && Object.keys(output.value).length > 0)
      note('gave a result the core cannot express');
    return result;
  }
  return PLAIN[event] === true ? context(stdout) : undefined;
}

function main() {
  if (!Object.hasOwn(INPUTS, event) || given.command === undefined) {
    return note('cannot run: usage is --event <event> [flags] -- <command> [args...]');
  }
  const core = record(parsed(readFileSync(0, 'utf8'))?.value);
  if (given.profile !== undefined && record(core.tool_profile).name !== given.profile)
    return undefined;
  const input = {
    hook_event_name: event,
    session_id: core.session_id,
    cwd: core.cwd,
    ...INPUTS[event](core),
  };
  const environment = {
    CLAUDE_PLUGIN_ROOT: proc.env.PLUGIN_ROOT ?? '',
    CLAUDE_PLUGIN_DATA: proc.env.PLUGIN_DATA ?? '',
    CLAUDE_PROJECT_DIR: typeof core.cwd === 'string' ? core.cwd : '',
  };
  const [program, programArgs] = launch(environment);
  const run = spawnSync(program, programArgs, {
    env: { ...proc.env, ...environment },
    input: JSON.stringify(input),
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    timeout: given.timeout,
    windowsHide: true,
  });
  const result = resultOf(run);
  if (result !== undefined) proc.stdout.write(JSON.stringify(result));
  return undefined;
}

main();
"#;

crate::registry::exports! {
    "convert/adapters/claude-code-runner-source" {
        "CLAUDE_CODE_RUNNER_SOURCE" => ok || CLAUDE_CODE_RUNNER_SOURCE;
    }
}
