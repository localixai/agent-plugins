//! Port of `src/convert/adapters/sources.ts`, switched.
//!
//! The two hook adapters a conversion adds (Agent Plugins Extended §18.4.4), as the text of Node.js
//! ES modules: self-contained, since they run where the kit is not installed, and the same bytes for
//! every conversion (§18.4.1 item 5). The runtime they need is `node` on `PATH`.

use std::sync::LazyLock;

use super::claude_code_runner_source::CLAUDE_CODE_RUNNER_SOURCE;
use super::core_profiles_source::CORE_PROFILES_SOURCE;
use super::core_runner_source::CORE_RUNNER_SOURCE;

/// The interpreter the adapters need on `PATH`.
pub const ADAPTER_RUNTIME: &str = "node";

fn header(purpose: &str) -> String {
    [
        "#!/usr/bin/env node",
        &format!("// {purpose}"),
        "// Written by @localix-ai/agent-plugins (Agent Plugins Extended §18.4.4): convert the package",
        "// again rather than editing it.",
        "",
    ]
    .join("\n")
}

fn imports(lines: &[&str]) -> String {
    format!("{}\n\n", lines.join("\n"))
}

/// Runs a core hook handler under Claude Code: `run-core-handler.mjs <event> -- <command> [args...]`.
pub static RUN_CORE_HANDLER_SOURCE: LazyLock<String> = LazyLock::new(|| {
    [
        header("Runs a core Agent Plugins hook handler under Claude Code."),
        imports(&[
            "const { spawnSync } = await import('node:child_process');",
            "const { mkdirSync, readFileSync } = await import('node:fs');",
            "const { isAbsolute, resolve } = await import('node:path');",
            "const { default: proc } = await import('node:process');",
            "",
            "const [event, separator, command, ...args] = proc.argv.slice(2);",
        ]),
        CORE_PROFILES_SOURCE.to_owned(),
        "\n".to_owned(),
        CORE_RUNNER_SOURCE.to_owned(),
    ]
    .concat()
});

/// Runs a Claude Code `command` hook under a core client: `run-claude-code-handler.mjs --event <event>
/// [--profile <name>] [--timeout <seconds>] [--shell bash|powershell] [--exec] -- <command> [args...]`.
pub static RUN_CLAUDE_CODE_HANDLER_SOURCE: LazyLock<String> = LazyLock::new(|| {
    [
        header("Runs a Claude Code command hook under a core Agent Plugins client."),
        imports(&[
            "const { spawnSync } = await import('node:child_process');",
            "const { readFileSync } = await import('node:fs');",
            "const { default: proc } = await import('node:process');",
        ]),
        CLAUDE_CODE_RUNNER_SOURCE.to_owned(),
    ]
    .concat()
});

crate::registry::exports! {
    "convert/adapters/sources" {
        "ADAPTER_RUNTIME" => ok || ADAPTER_RUNTIME;
        "RUN_CLAUDE_CODE_HANDLER_SOURCE" => ok || RUN_CLAUDE_CODE_HANDLER_SOURCE.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_adapters_start_with_their_header_and_end_with_their_body() {
        let core = RUN_CORE_HANDLER_SOURCE.as_str();
        assert!(core.starts_with("#!/usr/bin/env node\n// Runs a core Agent Plugins hook handler"));
        assert!(core.ends_with(CORE_RUNNER_SOURCE));
        let claude = RUN_CLAUDE_CODE_HANDLER_SOURCE.as_str();
        assert!(claude.ends_with(CLAUDE_CODE_RUNNER_SOURCE));
    }
}
