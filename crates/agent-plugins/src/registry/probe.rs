//! The boundary's own exports, `registry/probe`: no module of the package, only what a host's
//! loop is tested with (the npm package's `tests/wasm/`), so each part of the protocol has an
//! export that uses it while no ported module does yet: needs asked together, and a handle with
//! a sync and an async method.

use std::sync::Mutex;

use super::{Callback, Handle, Port};
use crate::ports::files::TextFileReader as _;

/// A running total, the probe's object with methods.
#[derive(Debug)]
pub(crate) struct Counter {
    total: Mutex<f64>,
}

impl Counter {
    /// A counter from `start`.
    pub(crate) const fn new(start: f64) -> Self {
        Self {
            total: Mutex::new(start),
        }
    }

    /// Adds `by`; the new total.
    fn add(&self, by: f64) -> f64 {
        let mut total = self
            .total
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *total += by;
        *total
    }
}

crate::registry::methods! {
    Counter {
        "add" => ok |this, by: f64| this.add(by);
        "addLater" => async ok |this, by: Callback| this.add(by.call::<f64>(vec![]).await);
    }
}

/// The texts of two files, both asked before either is awaited.
async fn both(files: Port, first: String, second: String) -> Vec<Option<String>> {
    let (one, two) = (
        files.read_text(&first, None),
        files.read_text(&second, None),
    );
    vec![one.await, two.await]
}

crate::registry::exports! {
    "registry/probe" {
        "counter" => ok |start: f64| Handle::new(Counter::new(start));
        "both" => async ok |files: Port, first: String, second: String| both(files, first, second).await;
    }
}
