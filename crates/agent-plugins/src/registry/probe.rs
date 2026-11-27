//! The boundary's own exports, `registry/probe`: no module of the package, only what a host's
//! loop is tested with (the npm package's `tests/wasm/`), so each part of the protocol has an
//! export that uses it while no ported module does yet: needs asked together, a port with a
//! method the host answers at once and one it answers with a promise, a host's throw caught, and
//! a handle with a sync and an async method, promises of the host's awaited and caught, and an
//! answer that holds a promise of the core's, and a handle given back (`Own`).

use std::sync::Mutex;

use serde::Serialize;

use super::{Callback, CorePromise, Handle, HostPromise, Own, Port, catch_host};
use crate::js::JsValue;
use crate::js::promise_all;
use crate::ports::files::TextFileReader as _;
use crate::ports::folder_scan::FolderScan as _;
use crate::ports::locations::Locations as _;

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

/// Where a plugin keeps its data, under the home folder: `homeDir` answers at once on the
/// host, `pluginData` with a promise.
async fn located(locations: Port, plugin: String) -> Vec<String> {
    let home = locations.home_dir().await;
    let data = locations.plugin_data(&plugin, None).await;
    vec![home, data]
}

/// The `dir` property of the folder the host opens, a port its answer hands over.
async fn opened(scan: Port, dir: String) -> Option<String> {
    let folder = scan.open(&dir, None).await?;
    Some(folder.dir().await)
}

/// What the host's callback answered, or the message and name of what it threw, caught.
async fn caught(callback: Callback) -> Vec<Option<String>> {
    match catch_host(callback.call::<String>(vec![])).await {
        Ok(text) => vec![Some(text)],
        Err(cause) => vec![Some(cause.message), cause.name],
    }
}

/// What each of the host's promises settled to, in their order: its value, or the message of
/// its rejection, caught.
async fn settled(promises: Vec<HostPromise<JsValue>>) -> Vec<JsValue> {
    let each = promises.into_iter().map(|promise| {
        let settling: crate::ports::BoxFuture<'static, JsValue> = Box::pin(async move {
            catch_host(promise)
                .await
                .unwrap_or_else(|cause| format!("rejected: {}", cause.message).into())
        });
        settling
    });
    promise_all(each.collect()).await
}

/// An answer that holds a promise of the core's: what the host's callback answers, called when
/// the host reads the answer.
#[derive(Debug, Serialize)]
struct Later {
    done: CorePromise<JsValue>,
}

crate::registry::exports! {
    "registry/probe" {
        "counter" => ok |start: f64| Handle::own(Counter::new(start));
        "addTo" => ok |counter: Own<Counter>, by: f64| counter.add(by);
        "both" => async ok |files: Port, first: String, second: String| both(files, first, second).await;
        "located" => async ok |locations: Port, plugin: String| located(locations, plugin).await;
        "opened" => async ok |scan: Port, dir: String| opened(scan, dir).await;
        "caught" => async ok |callback: Callback| caught(callback).await;
        "settled" => async ok |promises: Vec<HostPromise<JsValue>>| settled(promises).await;
        "later" => ok |callback: Callback| Later {
            done: CorePromise::new(async move { callback.call::<JsValue>(vec![]).await }),
        };
    }
}
