//! [`Port`] as the host's command runner (`src/ports/command-runner.ts`).

use super::{Port, arg};
use crate::ports::BoxFuture;
use crate::ports::command_runner::{CommandResult, CommandRun, CommandRunner};

impl CommandRunner for Port {
    fn run(&self, run: CommandRun) -> BoxFuture<'_, CommandResult> {
        self.ask("run", vec![arg(Some(&run))])
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::super::testing::{exchange, json, need, port};
    use crate::ports::Signal;
    use crate::ports::command_runner::{CommandRun, CommandRunner as _, CommandSpec};

    #[test]
    fn a_run_crosses_as_the_typescript_names_it() {
        let run = CommandRun {
            spec: CommandSpec::Exec {
                command: "git".to_owned(),
                args: vec!["status".to_owned()],
            },
            cwd: "/p".to_owned(),
            env: IndexMap::from([("A".to_owned(), "1".to_owned())]),
            stdin: None,
            timeout_ms: Some(5.0),
            merge_stderr: Some(false),
            signal: Signal::new(),
        };
        let (asked, read) = exchange(
            async move { port().run(run).await },
            json(r#"{"exitCode": 0, "stdout": "ok", "stderr": "", "timedOut": false}"#),
        );
        let sent = json(
            r#"{"spec": {"form": "exec", "command": "git", "args": ["status"]}, "cwd": "/p",
            "env": {"A": "1"}, "timeoutMs": 5, "mergeStderr": false,
            "signal": {"$signal": null, "aborted": false}}"#,
        );
        assert_eq!(asked, need("run", vec![sent]));
        assert_eq!(
            read,
            r#"CommandResult { exit_code: Some(0), stdout: "ok", stderr: "", timed_out: false, start_error: None, output_held_open: None }"#
        );
    }
}
