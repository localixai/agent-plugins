//! Port of `src/marketplace/installed-plugins-store.ts`, switched.
//!
//! Reading `installed_plugins.json` through `PluginStateStore` and doing what the troubleshooting
//! page says Claude Code does with a file it cannot use (plugins/troubleshooting.md): keep the old
//! text of an unreadable file in a dated `.kept` copy and start the file over; copy records under
//! names no version can use into a dated `.set-aside` file, then drop them; and never rewrite the
//! file while a record, or the format, is one this version cannot read. A copy is written before
//! the file changes, so a failed copy loses nothing.
//!
//! Judgments (spec/judgments.md): `<date>` is the clock's UTC day as `YYYY-MM-DD`; `<hash>` is the
//! first eight hex digits of the SHA-256 of the text the copy keeps; a `.set-aside` copy is the
//! dropped keys with their raw values, as JSON.

use serde::{Deserialize, Serialize};

use super::command_source::WithSignal;
use super::installed_plugins::{
    InstalledPluginsRead, parse_installed_plugins, records_value, serialize_installed_plugins,
};
use super::store_layout::{STORE_NAMES, StoreLayout, create_store_layout};
use crate::js::slice_code_units;
use crate::js::{JsObject, JsValue, json_stringify_indented, number_to_string, pad_start};
use crate::ports::Signal;
use crate::ports::clock::Clock;
use crate::ports::failure::PortFailure;
use crate::ports::marketplace::{PluginStateStore, Sha256};
use crate::registry::Port;

/// The host's ports a load uses.
#[derive(Clone, Copy)]
pub struct InstalledPluginsPorts<'a> {
    /// The install records.
    pub store: &'a dyn PluginStateStore,
    /// The digest of a copy's text.
    pub sha256: &'a dyn Sha256,
    /// The day a copy is dated.
    pub clock: &'a dyn Clock,
}

/// The records and what became of the file.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LoadedInstalledPlugins {
    /// What the file said.
    pub read: InstalledPluginsRead,
    /// A copy or the rewritten file could not be written: the file is as it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<PortFailure>,
}

fn pad(value: f64, width: usize) -> String {
    pad_start(&number_to_string(value), width, "0")
}

/// Days since 1970-01-01 to `YYYY-MM-DD` (the civil-from-days algorithm; the kit uses no `Date`).
fn iso_day(ms: f64) -> String {
    let days = (ms / 86_400_000.0).floor() + 719_468.0;
    let era = (days / 146_097.0).floor();
    let day_of_era = days - era * 146_097.0;
    let year_of_era = ((day_of_era - (day_of_era / 1460.0).floor()
        + (day_of_era / 36_524.0).floor()
        - (day_of_era / 146_096.0).floor())
        / 365.0)
        .floor();
    let day_of_year = day_of_era
        - (365.0 * year_of_era + (year_of_era / 4.0).floor() - (year_of_era / 100.0).floor());
    let shifted = ((5.0 * day_of_year + 2.0) / 153.0).floor();
    let day = day_of_year - ((153.0 * shifted + 2.0) / 5.0).floor() + 1.0;
    let month = if shifted < 10.0 {
        shifted + 3.0
    } else {
        shifted - 9.0
    };
    let year = year_of_era + era * 400.0 + if month <= 2.0 { 1.0 } else { 0.0 };
    format!("{}-{}-{}", pad(year, 4), pad(month, 2), pad(day, 2))
}

/// The name of a copy: `name(<date>, <hash>)` for the clock's day and the text's digest.
enum CopyKind {
    /// `layout.unreadableCopy`.
    Unreadable,
    /// `layout.setAsideCopy`.
    SetAside,
}

type NameOnly = fn(&str, &str) -> String;

async fn copy_name(
    ports: InstalledPluginsPorts<'_>,
    layout: &StoreLayout<NameOnly>,
    text: &str,
    kind: CopyKind,
) -> String {
    let digest = ports.sha256.hex(text.as_bytes()).await;
    let date = iso_day(ports.clock.now().await);
    let hash = slice_code_units(&digest, 0, Some(8));
    match kind {
        CopyKind::Unreadable => layout.unreadable_copy(&date, &hash).await,
        CopyKind::SetAside => layout.set_aside_copy(&date, &hash).await,
    }
}

fn relative_only(_base: &str, relative: &str) -> String {
    relative.to_owned()
}

async fn write_both(
    store: &dyn PluginStateStore,
    copy: (&str, &str),
    records: &JsObject,
    signal: &Signal,
) -> Option<PortFailure> {
    if let Err(failure) = store.write(copy.0, copy.1, signal).await {
        return Some(failure);
    }
    let text = serialize_installed_plugins(records);
    store
        .write(STORE_NAMES.installed_plugins, &text, signal)
        .await
        .err()
}

/// Reads the records and applies the recovery the read calls for. A name here is relative to the
/// root.
pub async fn load_installed_plugins(
    ports: InstalledPluginsPorts<'_>,
    signal: &Signal,
) -> LoadedInstalledPlugins {
    let store = ports.store;
    let text = store.read(STORE_NAMES.installed_plugins, signal).await;
    let read = parse_installed_plugins(text.as_deref());
    let layout = create_store_layout(String::new(), relative_only as NameOnly);
    if let Some(rebuilt) = &read.rebuilt {
        let kept = copy_name(ports, &layout, &rebuilt.text, CopyKind::Unreadable).await;
        let failure = write_both(store, (&kept, &rebuilt.text), &JsObject::new(), signal).await;
        return LoadedInstalledPlugins { read, failure };
    }
    if !read.writable || read.set_aside.is_empty() {
        return LoadedInstalledPlugins {
            read,
            failure: None,
        };
    }
    let copy =
        json_stringify_indented(&JsValue::Object(read.set_aside.clone()), 2).unwrap_or_default();
    let name = copy_name(ports, &layout, &copy, CopyKind::SetAside).await;
    let records = records_value(&read.records);
    let failure = write_both(store, (&name, &copy), &records, signal).await;
    LoadedInstalledPlugins { read, failure }
}

/// The ports as they cross: a record of the host's objects.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct InstalledPluginsPortsGiven {
    /// The install records.
    pub store: Port,
    /// The digest.
    pub sha256: Port,
    /// The clock.
    pub clock: Port,
}

impl InstalledPluginsPortsGiven {
    /// The ports as `load_installed_plugins` takes them.
    #[must_use]
    pub fn ports(&self) -> InstalledPluginsPorts<'_> {
        InstalledPluginsPorts {
            store: &self.store,
            sha256: &self.sha256,
            clock: &self.clock,
        }
    }
}

crate::registry::exports! {
    "marketplace/installed-plugins-store" {
        "loadInstalledPlugins" => async ok |ports: InstalledPluginsPortsGiven, with: WithSignal|
            load_installed_plugins(ports.ports(), &with.signal).await;
    }
}
