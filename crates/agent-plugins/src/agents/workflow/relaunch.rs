//! Port of `src/agents/workflow/relaunch.ts`, switched.
//!
//! Resume after you leave the session (workflows.md, "Resume after you leave the session"): "the run
//! doesn't continue on its own. The run continues only when Claude relaunches it with a `Workflow` call,
//! which replays the run from its saved results." Pure decisions over facts the host holds: which way
//! the person left, whether the earlier run's saved results can be found.

use serde::{Deserialize, Serialize, Serializer};

use crate::js::JsValue;

/// The ways to leave the session.
pub const LEAVE_WAYS: [LeaveWay; 3] = [
    LeaveWay::Background,
    LeaveWay::MoveToBackgroundAndExit,
    LeaveWay::ExitAndStopTasks,
];

/// One of `LEAVE_WAYS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LeaveWay {
    /// The session goes to the background.
    Background,
    /// The session goes to the background and the person exits.
    MoveToBackgroundAndExit,
    /// The person exits and the tasks stop.
    ExitAndStopTasks,
}

/// Where the run can be picked up again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelaunchIn {
    /// Ask Claude in the background session.
    BackgroundSession,
    /// Resume the session first.
    ResumedSession,
}

/// What leaving in one way does to the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leaving {
    /// The run stopped with the session (`Exit and stop tasks`); in the other two it is only not running.
    pub stopped_with_session: bool,
    /// Where Claude relaunches it: ask Claude in the background session, or resume the session first.
    pub relaunch_in: RelaunchIn,
    /// The shell command that brings the session back, when it does not stay in the background.
    #[serde(serialize_with = "serialize_resume_command")]
    pub resume_command: Option<&'static str>,
}

/// The key is always there: `undefined` where the TypeScript table says `resumeCommand: undefined`.
fn serialize_resume_command<S: Serializer>(
    command: &Option<&'static str>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match command {
        Some(text) => serializer.serialize_str(text),
        None => JsValue::Undefined.serialize(serializer),
    }
}

const BACKGROUND: Leaving = Leaving {
    stopped_with_session: false,
    relaunch_in: RelaunchIn::BackgroundSession,
    resume_command: None,
};

/// How each way of leaving leaves the run, by way. In none of the three does the run continue on its
/// own: it continues only through a relaunch (`relaunch_after`), which is where the three differ.
pub const LEAVING: LeavingTable = LeavingTable {
    background: BACKGROUND,
    move_to_background_and_exit: BACKGROUND,
    exit_and_stop_tasks: Leaving {
        stopped_with_session: true,
        relaunch_in: RelaunchIn::ResumedSession,
        resume_command: Some("claude --resume"),
    },
};

/// `LEAVING`: one `Leaving` per way, keyed by the way's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct LeavingTable {
    /// The session goes to the background.
    pub background: Leaving,
    /// The session goes to the background and the person exits.
    pub move_to_background_and_exit: Leaving,
    /// The person exits and the tasks stop.
    pub exit_and_stop_tasks: Leaving,
}

/// What leaving in `way` does to the run.
#[must_use]
pub const fn leaving(way: LeaveWay) -> Leaving {
    match way {
        LeaveWay::Background => LEAVING.background,
        LeaveWay::MoveToBackgroundAndExit => LEAVING.move_to_background_and_exit,
        LeaveWay::ExitAndStopTasks => LEAVING.exit_and_stop_tasks,
    }
}

/// The error of a relaunch that finds no saved results at all: it never starts the run over on its own.
pub const NOTHING_TO_RESUME: &str = "nothing to resume";

/// What the host knows of the earlier run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's RelaunchFacts is these four facts of the docs' relaunch rules"
)]
pub struct RelaunchFacts {
    /// Claude has an earlier run of this workflow to relaunch (a fresh session has none).
    pub earlier_run: bool,
    /// The saved results of that run were found: under the session's directory in `~/.claude/projects/`, or restored with a cloud session's history.
    pub results_found: bool,
    /// The relaunch happens in a background session.
    pub in_background_session: bool,
    /// The run was started from the path of a script file outside `~/.claude/projects/`, such as one in the working directory.
    pub script_outside_projects: bool,
}

/// The facts of `RelaunchFacts` that do not depend on the way the person left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactsOfTheRun {
    /// Claude has an earlier run of this workflow to relaunch (a fresh session has none).
    pub earlier_run: bool,
    /// The saved results of that run were found.
    pub results_found: bool,
    /// The run was started from the path of a script file outside `~/.claude/projects/`.
    pub script_outside_projects: bool,
}

/// What a relaunch does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "relaunch", rename_all = "kebab-case")]
pub enum Relaunch {
    /// It replays from the saved results.
    Replay,
    /// It starts the workflow over.
    NewRun,
    /// It fails.
    Fail {
        /// `NOTHING_TO_RESUME`.
        error: &'static str,
    },
}

/// What a relaunch does. With no earlier run, Claude "starts the workflow over" as a new run. With one
/// whose saved results are gone, or a background session relaunching a script that lives outside
/// `~/.claude/projects/`, it fails with `nothing to resume`; otherwise it replays from the saved results.
#[must_use]
pub const fn relaunch(facts: RelaunchFacts) -> Relaunch {
    if !facts.earlier_run {
        return Relaunch::NewRun;
    }
    let cannot_find =
        !facts.results_found || (facts.in_background_session && facts.script_outside_projects);
    if cannot_find {
        Relaunch::Fail {
            error: NOTHING_TO_RESUME,
        }
    } else {
        Relaunch::Replay
    }
}

/// What Claude's relaunch does after the person left in `way`: in the background session it is a
/// relaunch there (so a script outside `~/.claude/projects/` cannot be found), after `Exit and stop
/// tasks` it happens in the resumed session.
#[must_use]
pub const fn relaunch_after(way: LeaveWay, facts: FactsOfTheRun) -> Relaunch {
    relaunch(RelaunchFacts {
        earlier_run: facts.earlier_run,
        results_found: facts.results_found,
        in_background_session: matches!(leaving(way).relaunch_in, RelaunchIn::BackgroundSession),
        script_outside_projects: facts.script_outside_projects,
    })
}

crate::registry::exports! {
    "agents/workflow/relaunch" {
        "LEAVE_WAYS" => ok || LEAVE_WAYS;
        "LEAVING" => ok || LEAVING;
        "NOTHING_TO_RESUME" => ok || NOTHING_TO_RESUME;
        "relaunch" => ok |facts: RelaunchFacts| relaunch(facts);
        "relaunchAfter" => ok |way: LeaveWay, facts: FactsOfTheRun| relaunch_after(way, facts);
    }
}
