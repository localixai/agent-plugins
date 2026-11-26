//! Port of `src/format/core/placeholders.ts`, switched.
//!
//! The two plugin variables of the core (core §9): `${PLUGIN_ROOT}`, the filesystem-resolved plugin
//! root, and `${PLUGIN_DATA}`, the client-managed data directory of the installed instance. Expansion
//! is one non-recursive textual pass over every exact occurrence of either placeholder: text a
//! replacement introduces is never scanned again, and any other `${...}` stays literal (core §9.2).

use serde::{Deserialize, Serialize};

use crate::format::path_text::{PathPlatform, resolve_segments};

/// The two directories, absolute and `/`-spelled, as the host resolved them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorePaths {
    /// The plugin root.
    pub root: String,
    /// The data directory.
    pub data: String,
}

/// The two variables, in the package's order.
pub const CORE_VARIABLES: [CoreVariable; 2] = [CoreVariable::PluginRoot, CoreVariable::PluginData];

/// A plugin variable of the core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoreVariable {
    /// `${PLUGIN_ROOT}`.
    #[serde(rename = "PLUGIN_ROOT")]
    PluginRoot,
    /// `${PLUGIN_DATA}`.
    #[serde(rename = "PLUGIN_DATA")]
    PluginData,
}

impl CoreVariable {
    /// The variable's name, as written between `${` and `}`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PluginRoot => "PLUGIN_ROOT",
            Self::PluginData => "PLUGIN_DATA",
        }
    }
}

/// The directory each variable stands for.
fn value_of(paths: &CorePaths, variable: CoreVariable) -> &str {
    match variable {
        CoreVariable::PluginRoot => &paths.root,
        CoreVariable::PluginData => &paths.data,
    }
}

/// The variable whose placeholder `text` begins with, and the placeholder's length.
fn placeholder_at(text: &str) -> Option<(CoreVariable, usize)> {
    CORE_VARIABLES.iter().find_map(|variable| {
        let placeholder = format!("${{{}}}", variable.name());
        text.starts_with(&placeholder)
            .then_some((*variable, placeholder.len()))
    })
}

/// `text` with both placeholders replaced in one pass (core §9.2).
#[must_use]
pub fn expand_placeholders(text: &str, paths: &CorePaths) -> String {
    let mut expanded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        let (before, from_start) = rest.split_at(start);
        expanded.push_str(before);
        match placeholder_at(from_start) {
            Some((variable, length)) => {
                expanded.push_str(value_of(paths, variable));
                rest = from_start.get(length..).unwrap_or_default();
            }
            None => {
                expanded.push_str("${");
                rest = from_start.get(2..).unwrap_or_default();
            }
        }
    }
    expanded.push_str(rest);
    expanded
}

/// The three forms a stdio server's `cwd` may take (core §7.2.1): a `./` path under the plugin root,
/// `${PLUGIN_ROOT}` or a path below it, `${PLUGIN_DATA}` or a path below it. `rest` is what follows
/// the base, before expansion.
struct CwdForm<'a> {
    base: CoreVariable,
    rest: &'a str,
}

fn cwd_form(cwd: &str) -> Option<CwdForm<'_>> {
    if let Some(rest) = cwd.strip_prefix("./") {
        return Some(CwdForm {
            base: CoreVariable::PluginRoot,
            rest,
        });
    }
    CORE_VARIABLES.iter().find_map(|name| {
        let rest = cwd.strip_prefix(&format!("${{{}}}", name.name()))?;
        (rest.is_empty() || rest.starts_with('/')).then_some(CwdForm { base: *name, rest })
    })
}

/// A resolved `cwd`: under which directory, the path below it, and the absolute path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCwd {
    /// The directory it is under.
    pub base: CoreVariable,
    /// `/`-separated, no `.` or `..`; `.` for the base itself.
    pub below: String,
    /// The absolute path.
    pub path: String,
}

/// What `resolve_cwd` answers: the resolved `cwd`, or why the server entry is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CwdResolution {
    /// The `cwd` resolved.
    Cwd(ResolvedCwd),
    /// Why it did not.
    Problem(String),
}

/// Expands, then resolves a `cwd` (core §7.2.1: "Clients MUST expand placeholders before resolving
/// `cwd`"), or says why the server entry is invalid: another form, or a path that climbs out of its
/// base. The containment checked here is textual; whether a link below the base leads out is the
/// loader's question for the plugin root (`loadCorePackage`) and the host's for the data directory.
#[must_use]
pub fn resolve_cwd(cwd: &str, paths: &CorePaths) -> CwdResolution {
    let Some(form) = cwd_form(cwd) else {
        return CwdResolution::Problem(
            "must begin with \"./\", \"${PLUGIN_ROOT}\" or \"${PLUGIN_DATA}\"".to_owned(),
        );
    };
    let Some(segments) =
        resolve_segments(&expand_placeholders(form.rest, paths), PathPlatform::Posix)
    else {
        return CwdResolution::Problem(format!("leaves {} after it is resolved", form.base.name()));
    };
    let base = value_of(paths, form.base);
    let below = if segments.is_empty() {
        ".".to_owned()
    } else {
        segments.join("/")
    };
    let path = if below == "." {
        base.to_owned()
    } else {
        format!("{base}/{below}")
    };
    CwdResolution::Cwd(ResolvedCwd {
        base: form.base,
        below,
        path,
    })
}

/// Paths to judge a configuration by before the host knows the real ones: absolute paths that hold
/// no `..`, as any real resolved directory does, so containment comes out the same.
#[must_use]
pub fn symbolic_paths() -> CorePaths {
    CorePaths {
        root: "/plugin-root".to_owned(),
        data: "/plugin-data".to_owned(),
    }
}

crate::registry::exports! {
    "format/core/placeholders" {
        "CORE_VARIABLES" => ok || CORE_VARIABLES;
        "expandPlaceholders" => ok |text: String, paths: CorePaths| expand_placeholders(&text, &paths);
        "resolveCwd" => ok |cwd: String, paths: CorePaths| resolve_cwd(&cwd, &paths);
    }
}
