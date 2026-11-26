//! The host's loop over the task table: tasks interleaved, needs asked together, a host's throw,
//! a task dropped, a signal aborted, a handle's methods and its release.

use serde::Deserialize as _;

use super::{resume, resume_json, start, start_json};
use crate::format::read_json::BOM_PARSE_ERROR;
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::ports::Signal;
use crate::ports::files::TextFileReader as _;
use crate::registry::probe::Counter;
use crate::registry::task::{self, Need, Reply, Step};
use crate::registry::{Answer, Handle, Port, call, handle};

fn port(number: f64) -> JsValue {
    JsValue::from([("$port", JsValue::from(number))])
}

fn read_text(id: u32, path: &str) -> Need {
    Need {
        id,
        port: 0,
        method: "readText".to_owned(),
        args: vec![JsValue::from(path), JsValue::Undefined],
    }
}

/// The task number and needs of a step that waits.
fn waits(step: Step) -> (u32, Vec<Need>) {
    match step {
        Step::Needs { task, needs } => (task, needs),
        other @ (Step::Done(_) | Step::Rethrow { .. }) => {
            panic!("the task does not wait: {other:?}")
        }
    }
}

fn result(text: &str) -> JsValue {
    JsValue::from([("value", crate::js::json_parse::json_parse(text).unwrap())])
}

#[test]
fn two_tasks_wait_and_end_in_any_order() {
    let (first, asked) = waits(start(
        "format/read-json#readJson",
        &[port(0.0), "a.json".into()],
    ));
    assert_eq!(asked, vec![read_text(0, "a.json")]);
    let (second, asked) = waits(start(
        "format/read-json#readJson",
        &[port(0.0), "b.json".into()],
    ));
    assert_eq!(asked, vec![read_text(0, "b.json")]);
    assert_ne!(first, second);

    let done = resume(second, vec![(0, Reply::Result("[2]".into()))], &[]);
    assert_eq!(done, Step::Done(Answer::Result(result("[2]"))));
    let done = resume(first, vec![(0, Reply::Result(JsValue::Undefined))], &[]);
    let missing = JsValue::from([
        ("problem", JsValue::from("the file could not be read")),
        ("missing", JsValue::from(true)),
    ]);
    assert_eq!(done, Step::Done(Answer::Result(missing)));
}

#[test]
fn needs_asked_in_one_poll_are_given_together() {
    let files = Port::deserialize(port(3.0)).unwrap();
    let (number, needs) = waits(task::start(Box::pin(async move {
        let first = files.read_text("a", None);
        let second = files.read_text("b", None);
        let text = format!("{:?} {:?}", first.await, second.await);
        Answer::Result(text.into())
    })));
    let ids: Vec<(u32, u32)> = needs.iter().map(|need| (need.id, need.port)).collect();
    assert_eq!(ids, vec![(0, 3), (1, 3)]);
    // The second answer alone: the task still waits on the first, and asks nothing new.
    let (_, none) = waits(resume(number, vec![(1, Reply::Result("B".into()))], &[]));
    assert!(none.is_empty());
    let done = resume(number, vec![(0, Reply::Result("A".into()))], &[]);
    assert_eq!(
        done,
        Step::Done(Answer::Result(r#"Some("A") Some("B")"#.into()))
    );
}

#[test]
fn a_host_throw_ends_the_task_as_a_rethrow() {
    let before = task::waiting();
    let (number, _) = waits(start("format/listing#listSorted", &[port(0.0), ".".into()]));
    assert_eq!(task::waiting(), before + 1);
    let step = resume(
        number,
        vec![(
            0,
            Reply::Throws {
                message: "EACCES".into(),
                name: None,
            },
        )],
        &[],
    );
    assert_eq!(step, Step::Rethrow { need: 0 });
    assert_eq!(task::waiting(), before);
    let again = resume(number, vec![], &[]);
    assert!(matches!(again, Step::Done(Answer::Fault(why)) if why.contains("ended")));
}

#[test]
fn a_dropped_task_answers_nothing_more() {
    let (number, _) = waits(start("format/read-json#readJson", &[port(0.0), "a".into()]));
    assert!(task::cancel(number));
    assert!(!task::cancel(number));
    let late = resume(number, vec![(0, Reply::Result("{}".into()))], &[]);
    assert!(matches!(late, Step::Done(Answer::Fault(_))));
}

#[test]
fn an_answer_to_no_need_and_a_wrong_shape_are_faults() {
    let (number, _) = waits(start("format/read-json#readJson", &[port(0.0), "a".into()]));
    let wrong = resume(number, vec![(9, Reply::Result("{}".into()))], &[]);
    assert!(matches!(wrong, Step::Done(Answer::Fault(why)) if why.contains("need 9")));
    let (number, _) = waits(start("format/read-json#readJson", &[port(0.0), "a".into()]));
    let shape = resume(number, vec![(0, Reply::Result(JsValue::from(1.0)))], &[]);
    assert!(matches!(shape, Step::Done(Answer::Fault(why)) if why.contains("readText")));
}

#[test]
fn an_export_that_waits_is_no_sync_call() {
    let text = call("format/read-json#readJson", r#"[{"$port":0},"a"]"#);
    assert!(text.starts_with(r#"{"fault":"#), "{text}");
    let sync = start_json("format/read-json#invalidJsonSyntax", r#"["x"]"#);
    assert_eq!(sync, r#"{"done":{"result":"Invalid JSON syntax: x"}}"#);
}

#[test]
fn an_async_export_that_asks_nothing_answers_a_sync_call() {
    let ledger =
        r#"{"loaded":[{"command":"a","content":"a body","order":1}],"grants":[],"disallowed":[]}"#;
    let text = call(
        "skills/lifecycle#reattachAfterCompaction",
        &format!("[{ledger}]"),
    );
    assert_eq!(
        text,
        r#"{"result":[{"command":"a","content":"a body","order":1}]}"#
    );
    let waits = call(
        "skills/lifecycle#reattachAfterCompaction",
        &format!(r#"[{ledger},{{"tokens":{{"$port":0}}}}]"#),
    );
    assert!(waits.contains("it waits on the host"), "{waits}");
}

#[test]
fn one_round_as_json_text() {
    let signal = r#"{"$signal":4,"aborted":false}"#;
    let step = start_json(
        "format/read-json#readJsonWithoutBom",
        &format!(r#"[{{"$port":1}},"hooks/hooks.json",{signal}]"#),
    );
    let number = step
        .strip_prefix(r#"{"task":"#)
        .and_then(|rest| rest.split_once(','))
        .map(|(number, _)| number.parse::<u32>().unwrap())
        .unwrap();
    assert_eq!(
        step,
        format!(
            r#"{{"task":{number},"needs":[{{"id":0,"port":1,"method":"readBytes","args":["hooks/hooks.json",{{"$signal":4,"aborted":false}}]}}]}}"#
        )
    );
    let done = resume_json(
        number,
        r#"{"answers":[{"id":0,"result":{"$bytes":"77u/e30="}}]}"#,
    );
    let bom = JsValue::from([("problem", BOM_PARSE_ERROR.into())]);
    assert_eq!(done, Step::Done(Answer::Result(bom)).to_json_text());
    let (number, _) = waits(start("format/read-json#readJson", &[port(0.0), "a".into()]));
    let thrown = resume_json(number, r#"{"answers":[{"id":0,"throws":"EIO"}]}"#);
    assert_eq!(thrown, r#"{"rethrow":0}"#);
}

#[test]
fn a_callback_is_a_need_named_call() {
    let callback = JsValue::from([("$fn", JsValue::from(2.0))]);
    let step = start(
        "format/config-directory#configDirectoryOf",
        &["/home".into(), JsValue::Undefined, callback],
    );
    let (number, needs) = waits(step);
    let call = Need {
        id: 0,
        port: 2,
        method: "call".to_owned(),
        args: vec!["/home".into(), ".claude".into()],
    };
    assert_eq!(needs, vec![call]);
    let done = resume(
        number,
        vec![(0, Reply::Result("/home/.claude".into()))],
        &[],
    );
    assert_eq!(done, Step::Done(Answer::Result("/home/.claude".into())));
}

#[test]
fn a_signal_aborts_when_the_host_says_so() {
    let tag = JsValue::from([("$signal", JsValue::from(8.0)), ("aborted", false.into())]);
    let signal = Signal::deserialize(tag).unwrap();
    let files = Port::deserialize(port(0.0)).unwrap();
    let (number, _) = waits(task::start(Box::pin(async move {
        files.read_text("a", Some(&signal)).await;
        Answer::Result(signal.is_aborted().into())
    })));
    let done = resume(number, vec![(0, Reply::Result("x".into()))], &[8]);
    assert_eq!(done, Step::Done(Answer::Result(true.into())));
}

#[test]
fn a_handle_is_called_by_its_number_and_released() {
    let before = handle::held();
    let counter = Handle::new(Counter::new(0.0));
    let JsValue::Object(tag) = to_js_value(&counter).unwrap() else {
        panic!("a handle is an object");
    };
    let number = u32::deserialize(tag.get("$handle").cloned().unwrap()).unwrap();
    assert_eq!(tag.get("sync"), Some(&JsValue::from(vec!["add".into()])));
    assert_eq!(
        tag.get("async"),
        Some(&JsValue::from(vec!["addLater".into()]))
    );
    assert_eq!(handle::held(), before + 1);
    let name = |method: &str| format!("{number}#{method}");

    let added = start(&name("add"), &[2.0.into()]);
    assert_eq!(added, Step::Done(Answer::Result(2.0.into())));
    let callback = JsValue::from([("$fn", JsValue::from(0.0))]);
    let (task, needs) = waits(start(&name("addLater"), &[callback]));
    assert_eq!(needs.first().map(|need| need.method.as_str()), Some("call"));
    let done = resume(task, vec![(0, Reply::Result(3.0.into()))], &[]);
    assert_eq!(done, Step::Done(Answer::Result(5.0.into())));

    assert!(crate::registry::release(number));
    assert_eq!(handle::held(), before);
    assert!(matches!(
        start(&name("add"), &[1.0.into()]),
        Step::Done(Answer::Fault(_))
    ));
}
