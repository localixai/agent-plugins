//! Port of `src/convert/adapters/core-runner-source.ts`, switched.
//!
//! The body of `run-core-handler.mjs` (Agent Plugins Extended §18.4.4): which Claude Code occurrences
//! are core occurrences (§18.1.2: `SessionStart` on `startup`, `clear` and `fork` outside a subagent,
//! `UserPromptSubmit` on every turn but a loop or schedule wakeup (§18.4.4 item 5, J18.23: `system`
//! also carries peer messages and task notifications, which a hook cannot tell apart), `PostToolUse` also on a failure that is no interrupt), the
//! core input of each, the run of the core handler as core §7.3.3 starts one (the plugin root as its
//! working directory, `PLUGIN_ROOT` and `PLUGIN_DATA` set, the data directory created), and its result
//! read by the closed shapes and written as Claude Code's (§18.2). A failure is a non-blocking error:
//! exit 1, never 2 (§18.4.4 item 3).

/// The text of the file the adapter writes.
pub const CORE_RUNNER_SOURCE: &str = r#"const NEW_SESSION = ['startup', 'clear', 'fork'];
const WAKEUPS = ['loop_wakeup', 'schedule_wakeup'];

const common = (claude) => ({
  hook_event_name: event,
  session_id: claude.session_id,
  cwd: claude.cwd,
});

function toolInput(claude, after) {
  const base = {
    ...common(claude),
    tool_use_id: claude.tool_use_id,
    tool_name: claude.tool_name,
    tool_input: claude.tool_input ?? {},
  };
  const name = text(claude.tool_name) ?? '';
  if (after === undefined) {
    const profile = pendingProfile(name, claude.tool_input, claude.cwd);
    return profile === undefined ? base : { ...base, tool_profile: profile };
  }
  const profile = completedProfile(name, claude.tool_input, after, claude.cwd);
  const result = after.ok ? after.response : after.error;
  const input = { ...base, tool_result: result };
  return profile === undefined ? input : { ...input, tool_profile: profile };
}

// The core input of this occurrence, or undefined when the core event does not fire on it.
const INPUTS = {
  SessionStart: (claude) =>
    claude.hook_event_name === 'SessionStart' &&
    NEW_SESSION.includes(claude.source) &&
    claude.agent_id === undefined
      ? common(claude)
      : undefined,
  UserPromptSubmit: (claude) =>
    claude.hook_event_name === 'UserPromptSubmit' && !WAKEUPS.includes(claude.source)
      ? { ...common(claude), prompt: text(claude.prompt) ?? '' }
      : undefined,
  PreToolUse: (claude) => (claude.hook_event_name === 'PreToolUse' ? toolInput(claude) : undefined),
  PostToolUse: (claude) => {
    if (claude.hook_event_name === 'PostToolUse')
      return toolInput(claude, { ok: true, response: claude.tool_response });
    if (claude.hook_event_name !== 'PostToolUseFailure' || claude.is_interrupt === true)
      return undefined;
    return toolInput(claude, { ok: false, error: text(claude.error) ?? '' });
  },
  Stop: (claude) =>
    claude.hook_event_name === 'Stop'
      ? { ...common(claude), continuation_active: claude.stop_hook_active === true }
      : undefined,
};

// The closed output shapes of the core (schemas/1.1.0/hooks.schema.json) as Claude Code results.
const filled = (value) => typeof value === 'string' && value !== '';
const only = (output, keys) =>
  Object.keys(output).length === keys.length && keys.every((key) => Object.hasOwn(output, key));
const contextOutput = (claude, output) =>
  only(output, ['additional_context']) && filled(output.additional_context)
    ? {
        hookSpecificOutput: {
          hookEventName: claude.hook_event_name,
          additionalContext: output.additional_context,
        },
      }
    : undefined;
const decision = (output, value, reason) =>
  reason
    ? only(output, ['decision', 'reason']) && output.decision === value && filled(output.reason)
    : only(output, ['decision']) && output.decision === value;

const RESULTS = {
  SessionStart: contextOutput,
  UserPromptSubmit: (claude, output) =>
    decision(output, 'deny', true)
      ? { decision: 'block', reason: output.reason }
      : contextOutput(claude, output),
  PreToolUse: (_claude, output) => {
    if (decision(output, 'allow', false)) {
      return { hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'allow' } };
    }
    if (!decision(output, 'deny', true)) return undefined;
    return {
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        permissionDecision: 'deny',
        permissionDecisionReason: output.reason,
      },
    };
  },
  PostToolUse: contextOutput,
  Stop: (_claude, output) =>
    decision(output, 'continue', true) ? { decision: 'block', reason: output.reason } : undefined,
};

const label = [command, ...args].join(' ');

// A failure of the core handler: a Claude Code non-blocking error (exit 1, never 2).
function fail(message, detail) {
  const more = typeof detail === 'string' && detail.trim() !== '' ? ': ' + detail.trim() : '';
  proc.stderr.write('core hook handler [' + label + '] ' + message + more + '\n');
  proc.exitCode = 1;
}

function parsed(stdout) {
  try {
    return { value: JSON.parse(stdout) };
  } catch {
    return undefined;
  }
}

function main() {
  if (!Object.hasOwn(INPUTS, event) || separator !== '--' || command === undefined) {
    return fail('cannot run: usage is <core event> -- <command> [args...]');
  }
  const claude = record(parsed(readFileSync(0, 'utf8'))?.value);
  const input = INPUTS[event](claude);
  if (input === undefined) return undefined;
  const root = proc.env.CLAUDE_PLUGIN_ROOT;
  const data = proc.env.CLAUDE_PLUGIN_DATA;
  if (!filled(root) || !filled(data))
    return fail('cannot run: CLAUDE_PLUGIN_ROOT or CLAUDE_PLUGIN_DATA is not set');
  mkdirSync(data, { recursive: true });
  const program = command.startsWith('./') ? resolve(root, command) : command;
  const run = spawnSync(program, args, {
    cwd: root,
    env: { ...proc.env, PLUGIN_ROOT: root, PLUGIN_DATA: data },
    input: JSON.stringify(input),
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    windowsHide: true,
  });
  if (run.error !== undefined) return fail('did not start: ' + run.error.message);
  if (run.status !== 0)
    return fail(
      run.status === null ? 'was killed' : 'exited with status ' + run.status,
      run.stderr,
    );
  if (/^[ \t\n\r]*$/.test(run.stdout)) return undefined;
  const output = parsed(run.stdout);
  if (output === undefined || !isObject(output.value))
    return fail('wrote output that is not one JSON object');
  const result = RESULTS[event](claude, output.value);
  if (result === undefined) return fail('wrote an object that is not a ' + event + ' result');
  proc.stdout.write(JSON.stringify(result));
  return undefined;
}

main();
"#;

crate::registry::exports! {
    "convert/adapters/core-runner-source" {
        "CORE_RUNNER_SOURCE" => ok || CORE_RUNNER_SOURCE;
    }
}
