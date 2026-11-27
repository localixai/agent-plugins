//! Port of `src/format/mod-files.ts`, switched.
//!
//! The files of a mod (plugins/mods/reference.md, "Files"): `hooks/hooks.json` names one hooks
//! module in `modules`, the module has one of eight extensions, `*.test.ts` and `*.test.tsx` are
//! tests, and `types` in the manifest may name the declaration file. Pure readers: a problem is a
//! `PluginIssue`, never a throw.

use serde::{Deserialize, Serialize};

use super::issues::{PluginIssue, error};
use super::path_text::{
    PathPlatform, escapes_as_written, is_absolute_path, resolve_segments, root_relative,
};
use crate::js::JsValue;

/// The extensions a hooks module may have ("Named `.js`, `.mjs`, `.cjs`, `.jsx`, `.ts`, `.mts`, `.cts`, or `.tsx`").
const HOOKS_MODULE_EXTENSIONS: [&str; 8] =
    [".js", ".mjs", ".cjs", ".jsx", ".ts", ".mts", ".cts", ".tsx"];

/// Whether `path` names a hooks module by its extension.
#[must_use]
pub fn is_hooks_module_name(path: &str) -> bool {
    HOOKS_MODULE_EXTENSIONS
        .iter()
        .any(|extension| path.ends_with(extension))
}

/// Tests `claude plugin test` runs: "files whose names end in `.test.ts` or `.test.tsx`".
#[must_use]
pub fn is_mod_test_file(path: &str) -> bool {
    path.ends_with(".test.ts") || path.ends_with(".test.tsx")
}

/// Where the declarations are when the manifest says nothing ("`types/index.d.ts`, unless `types` in the manifest names another path").
const DEFAULT_TYPES_PATH: &str = "types/index.d.ts";

/// What `resolveTypesPath` reads of a manifest: `{ types?: string }`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TypesManifest {
    /// The path of the declaration file, as written.
    #[serde(default)]
    pub types: Option<String>,
}

/// The declaration file of a mod: the manifest's `types` path, else `types/index.d.ts`; relative to
/// the plugin folder, `/`-separated. `None` when the path leaves the plugin as written (`..`,
/// absolute, a backslash): there is no declaration file to read then (judgment, spec/judgments.md).
#[must_use]
pub fn resolve_types_path(manifest: &TypesManifest) -> Option<String> {
    let declared = manifest.types.as_deref();
    let Some(declared) = declared.filter(|path| !path.is_empty()) else {
        return Some(DEFAULT_TYPES_PATH.to_owned());
    };
    if escapes_as_written(declared, PathPlatform::Posix) {
        None
    } else {
        Some(root_relative(declared, PathPlatform::Posix))
    }
}

/// What `hooks/hooks.json` names (`HooksModules`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HooksModules {
    /// The one module path, relative to the plugin folder (`hooks/` joined to the entry), or `None` when the file names none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// The path as written in `hooks.json`, relative to that file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<String>,
    /// What is wrong with it.
    pub issues: Vec<PluginIssue>,
}

const PATH: &str = "hooks/hooks.json";

/// `./register.js` read from `hooks/` as a path from the plugin folder, or `None` when it
/// leaves the folder: climbs above it, is absolute, or holds a backslash (judgment: the module path
/// follows the plugin path rules, plugins/loading.md "Paths that escape the plugin directory").
fn from_plugin_root(written: &str) -> Option<String> {
    if is_absolute_path(written) || written.contains('\\') {
        return None;
    }
    resolve_segments(&format!("hooks/{written}"), PathPlatform::Posix)
        .map(|segments| segments.join("/"))
}

fn module_issues(value: &JsValue) -> Vec<PluginIssue> {
    let Some(written) = value.as_str().filter(|text| !text.is_empty()) else {
        return vec![error(
            &format!("{PATH}:modules[0]"),
            "A hooks module path must be a non-empty string",
        )];
    };
    if from_plugin_root(written).is_none() {
        return vec![error(
            &format!("{PATH}:modules[0]"),
            &format!("\"{written}\" leaves the plugin folder"),
        )];
    }
    if !is_hooks_module_name(written) {
        return vec![error(
            &format!("{PATH}:modules[0]"),
            &format!(
                "\"{written}\" is not a hooks module: name it {}",
                HOOKS_MODULE_EXTENSIONS.join(", ")
            ),
        )];
    }
    Vec::new()
}

const SECOND_ENTRY: &str =
    "hooks.json `modules` names one hooks module per plugin; a second entry is refused";

fn only_issues(issues: Vec<PluginIssue>) -> HooksModules {
    HooksModules {
        entry: None,
        written: None,
        issues,
    }
}

/// Reads `modules` of `hooks/hooks.json`: "an array with one path, relative to this file" (`"modules":
/// ["./register.js"]`). A file with no `modules` key holds no mod (a mod-less plugin is valid), and
/// the entry is `None`; one with the wrong number of paths, or a path of the wrong kind, is an
/// error. Settings hooks under `hooks` are read by `readHooksFile`.
#[must_use]
pub fn read_hooks_json_modules(raw: &JsValue) -> HooksModules {
    let JsValue::Object(members) = raw else {
        return only_issues(vec![error(PATH, "The hooks file must be a JSON object")]);
    };
    let Some(modules) = members.get("modules") else {
        return only_issues(Vec::new());
    };
    let JsValue::Array(modules) = modules else {
        return only_issues(vec![error(
            &format!("{PATH}:modules"),
            "\"modules\" must be an array with one path",
        )]);
    };
    if modules.is_empty() {
        return only_issues(vec![error(
            &format!("{PATH}:modules"),
            "\"modules\" must list exactly one hooks module, found 0",
        )]);
    }
    // Claude Code 2.1.325's wording (Agent Plugins Extended §10.3.1 item 2, Judgment J10.2).
    if modules.len() > 1 {
        return only_issues(vec![error(&format!("{PATH}:modules"), SECOND_ENTRY)]);
    }
    let written = modules.first().unwrap_or(&JsValue::Undefined);
    let issues = module_issues(written);
    let written = written.as_str();
    let entry = written.and_then(from_plugin_root);
    match (entry, written) {
        (Some(entry), Some(written)) if issues.is_empty() => HooksModules {
            entry: Some(entry),
            written: Some(written.to_owned()),
            issues,
        },
        _ => only_issues(issues),
    }
}

crate::registry::exports! {
    "format/mod-files" {
        "isHooksModuleName" => ok |path: String| is_hooks_module_name(&path);
        "isModTestFile" => ok |path: String| is_mod_test_file(&path);
        "resolveTypesPath" => ok |manifest: TypesManifest| resolve_types_path(&manifest);
        "readHooksJsonModules" => ok |raw: JsValue| read_hooks_json_modules(&raw);
    }
}
