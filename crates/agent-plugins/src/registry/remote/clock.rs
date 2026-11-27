//! [`Port`] as the host's clock (`src/ports/clock.ts`). A timer's `fire` is a function of the
//! core's that the host calls when the timer is due: it crosses as a [`Function`], and the host's
//! answer, the function that cancels the timer, is a [`Callback`].

use std::sync::{Mutex, PoisonError};

use super::{Callback, Port, arg};
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::ports::clock::{CancelTimer, Clock};
use crate::ports::{BoxFuture, Signal};
use crate::registry::Function;

/// A timer's `fire`, run at most once, when the host calls it.
struct Fire(Mutex<Option<Box<dyn FnOnce() + Send>>>);

impl Fire {
    fn fire(&self) {
        let fire = self.0.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(fire) = fire {
            fire();
        }
    }
}

crate::registry::methods! {
    Fire {
        "call" => ok |this| this.fire();
    }
}

impl Clock for Port {
    fn now(&self) -> BoxFuture<'_, f64> {
        self.ask("now", vec![])
    }

    fn set_timer(&self, fire: Box<dyn FnOnce() + Send>, ms: f64) -> BoxFuture<'_, CancelTimer> {
        let fire = to_js_value(&Function::new(Fire(Mutex::new(Some(fire))))).unwrap_or_default();
        let cancel = self.ask::<Callback>("setTimer", vec![fire, JsValue::from(ms)]);
        Box::pin(async move {
            let cancel = cancel.await;
            let cancel: CancelTimer = Box::new(move || {
                let called = cancel.call::<JsValue>(vec![]);
                Box::pin(async move {
                    called.await;
                })
            });
            cancel
        })
    }

    /// The host's sleep resolves (`Ok`) or rejects; a rejection, the abort included, is the
    /// host's throw, which ends the task as the TypeScript's `await` threw it.
    fn sleep<'a>(
        &'a self,
        ms: f64,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Result<(), String>> {
        let slept = self.ask::<JsValue>("sleep", vec![JsValue::from(ms), arg(signal)]);
        Box::pin(async move {
            slept.await;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::js::JsValue;
    use crate::ports::clock::Clock as _;
    use crate::registry::task::{self, Reply, Step};
    use crate::registry::{Answer, answer};

    #[test]
    fn the_time_and_a_sleep_are_asked() {
        let (asked, read) = exchange(async { port().now().await }, 5.0.into());
        assert_eq!((asked, read.as_str()), (need("now", vec![]), "5.0"));
        let (asked, read) = exchange(async { port().sleep(10.0, None).await }, JsValue::Undefined);
        assert_eq!(asked, need("sleep", vec![10.0.into(), JsValue::Undefined]));
        assert_eq!(read, "Ok(())");
    }

    #[test]
    fn a_timer_crosses_its_fire_and_cancels_through_the_hosts_answer() {
        let fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen = std::sync::Arc::clone(&fired);
        let Step::Needs { task, needs } = task::start(Box::pin(async move {
            let fire = Box::new(move || seen.store(true, std::sync::atomic::Ordering::SeqCst));
            let cancel = port().set_timer(fire, 20.0).await;
            cancel().await;
            Answer::Result(JsValue::Undefined)
        })) else {
            panic!("the timer asks nothing")
        };
        let set = needs.first().unwrap();
        assert_eq!(
            (set.method.as_str(), set.args.get(1)),
            ("setTimer", Some(&20.0.into()))
        );
        let Some(JsValue::Object(fire)) = set.args.first() else {
            panic!("no fire")
        };
        let Some(JsValue::Number(handle)) = fire.get("$call") else {
            panic!("the fire is no function: {fire:?}")
        };
        let handle = handle.to_string();
        assert_eq!(
            answer(&format!("{handle}#call"), &[]),
            Answer::Result(JsValue::Undefined)
        );
        assert!(fired.load(std::sync::atomic::Ordering::SeqCst));
        let step = task::resume(task, vec![(set.id, Reply::Result(json(r#"{"$fn": 3}"#)))]);
        let Step::Needs { task, needs } = step else {
            panic!("the cancel is not asked: {step:?}")
        };
        let cancel = needs.first().unwrap();
        assert_eq!((cancel.port, cancel.method.as_str()), (3, "call"));
        let done = task::resume(task, vec![(cancel.id, Reply::Result(JsValue::Undefined))]);
        assert_eq!(done, Step::Done(Answer::Result(JsValue::Undefined)));
        assert!(crate::registry::release(handle.parse().unwrap()));
    }
}
