//! Port of `src/convert/namespaces.ts`, switched.
//!
//! Every client extension namespace a core package carries (core §8), so a conversion loads the
//! package with all of them and reports each one instead of the loader dropping the namespaces the
//! host does not implement unseen (Agent Plugins Extended §18.4.1 item 1): the `extensions` keys of
//! `plugin.json`, of each `mcp.json` server and of each `hooks.json` handler, and every top-level
//! directory named as a reverse-domain namespace. Judgment (spec/judgments.md): a directory counts
//! when its name is two or more dot-separated labels of letters, digits and hyphens, the form core §8
//! gives namespaces (`com.example.client`); `assets/` never does.

use crate::format::listing::list_sorted;
use crate::format::read_json::{JsonRead, read_json};
use crate::js::{JsValue, compare_code_units, promise_all, promise_all_pair};
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};
use crate::protocol::is_record::is_record;
use crate::registry::Port;

/// `^[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+$`.
fn is_namespace(name: &str) -> bool {
    let labels: Vec<&str> = name.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
}

fn keys_of(value: &JsValue) -> Vec<String> {
    value
        .get("extensions")
        .filter(|extensions| is_record(value) && is_record(extensions))
        .and_then(JsValue::as_object)
        .map(|extensions| extensions.keys().cloned().collect())
        .unwrap_or_default()
}

fn values_of(value: &JsValue) -> Vec<&JsValue> {
    value
        .as_object()
        .map(|members| members.values().collect())
        .unwrap_or_default()
}

fn document_of<'a>(
    files: &'a dyn PluginFiles,
    path: &'a str,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, Option<JsValue>> {
    Box::pin(async move {
        match read_json(files, path, signal).await {
            JsonRead::Value { value } => Some(value),
            JsonRead::Problem { .. } => None,
        }
    })
}

/// Every client extension namespace a core package carries, sorted.
pub fn extension_namespaces<'a>(
    files: &'a dyn PluginFiles,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, Vec<String>> {
    Box::pin(async move {
        let (documents, entries) = promise_all_pair(
            promise_all(vec![
                document_of(files, "plugin.json", signal),
                document_of(files, "mcp.json", signal),
                document_of(files, "hooks.json", signal),
            ]),
            list_sorted(files, ".", signal),
        )
        .await;
        let (manifest, mcp, hooks) = match documents.as_slice() {
            [manifest, mcp, hooks] => (manifest, mcp, hooks),
            _ => (&None, &None, &None),
        };
        let servers: Vec<&JsValue> = mcp
            .as_ref()
            .and_then(|mcp| mcp.get("mcpServers").filter(|_| is_record(mcp)))
            .map(values_of)
            .unwrap_or_default();
        let handlers = hooks
            .as_ref()
            .and_then(|hooks| hooks.get("hooks").filter(|_| is_record(hooks)))
            .map(values_of)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|list| list.as_array().into_iter().flatten());
        let directories = entries
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| entry.is_directory && is_namespace(&entry.name))
            .map(|entry| entry.name);
        let mut found: Vec<String> = Vec::new();
        let names = manifest
            .iter()
            .flat_map(keys_of)
            .chain(servers.into_iter().flat_map(keys_of))
            .chain(handlers.flat_map(keys_of))
            .chain(directories);
        for name in names {
            if !found.contains(&name) {
                found.push(name);
            }
        }
        found.sort_by(|first, second| compare_code_units(first, second));
        found
    })
}

/// Whether `hooks/hooks.json` declares `modules`: the plugin is a mod (plugins/mods/reference.md, "Files").
pub fn declares_modules<'a>(
    files: &'a dyn PluginFiles,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, bool> {
    Box::pin(async move {
        let value = document_of(files, "hooks/hooks.json", signal).await;
        value.is_some_and(|value| {
            is_record(&value)
                && value
                    .as_object()
                    .is_some_and(|members| members.contains_key("modules"))
        })
    })
}

crate::registry::exports! {
    "convert/namespaces" {
        "extensionNamespaces" => async ok |files: Port, signal: Option<Signal>|
            extension_namespaces(&files, signal.as_ref()).await;
        "declaresModules" => async ok |files: Port, signal: Option<Signal>|
            declares_modules(&files, signal.as_ref()).await;
    }
}
