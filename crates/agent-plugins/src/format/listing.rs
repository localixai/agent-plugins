//! Port of `src/format/listing.ts`, switched.
//! The one order of names the crate lists by: UTF-16 code units, so an order never depends on the
//! host's locale or its port; and a folder listed in that order.

use std::cmp::Ordering;

use crate::js::compare_code_units;
use crate::ports::files::{DirectoryEntry, PluginFiles};
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// Orders by UTF-16 code units, so an order never depends on the host's locale or its port.
#[must_use]
pub fn by_code_unit(first: &str, second: &str) -> Ordering {
    compare_code_units(first, second)
}

/// A folder's entries sorted by name (code units), or `None` when it does not exist. Every
/// listing the crate reads goes through here, so what a plugin loads, which duplicate wins and
/// what the skill listing keeps never depend on the order the host's `list` returns (judgment).
/// The sort is stable, as `toSorted` is.
pub fn list_sorted<'a>(
    files: &'a dyn PluginFiles,
    path: &'a str,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, Option<Vec<DirectoryEntry>>> {
    Box::pin(async move {
        let mut entries = files.list(path, signal).await?;
        entries.sort_by(|first, second| by_code_unit(&first.name, &second.name));
        Some(entries)
    })
}

crate::registry::exports! {
    "format/listing" {
        "byCodeUnit" => ok |first: String, second: String| crate::registry::sign(by_code_unit(&first, &second));
        "listSorted" => async ok |files: Port, path: String, signal: Option<Signal>|
            list_sorted(&files, &path, signal.as_ref()).await;
    }
}
