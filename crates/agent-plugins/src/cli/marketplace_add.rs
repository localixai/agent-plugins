//! Port of `src/cli/marketplace-add.ts`, switched.
//!
//! `claude plugin marketplace add <source>` as two decisions around the host's fetch
//! (plugins/cli-reference.md, "plugin marketplace add", "JSON result for marketplace commands";
//! plugins/marketplace-reference.md, "Reserved names"): `startMarketplaceAdd` reads the arguments and
//! says what to fetch, or what the command ends with; `concludeMarketplaceAdd` takes what the fetch
//! found and says what to declare, what to install and what to print. The source forms, the
//! unrecognized-source and bare-host messages and `--sparse` are `parseMarketplaceInput`'s. What the
//! docs leave open is in spec/judgments.md.

use serde::{Deserialize, Serialize};

use super::lfs_pointer_line::lfs_pointer_line;
use super::result::{
    CommandOutput, MarketplaceCommand, marketplace_failure, marketplace_success, no_result_line,
    usage_error,
};
use super::scope::{ChosenOrError, ScopeChoice, ScopedCommand, choose_scope};
use crate::format::issues::Severity;
use crate::format::marketplace::marketplace_names::{
    NameOrigin, RegisteredMarketplace, download_folder_issues, load_name_issues,
    validation_name_issues,
};
use crate::format::marketplace::types::MarketplaceSource;
use crate::format::path_text::PathPlatform;
use crate::js::{self, value::to_js_value};
use crate::marketplace::add_failures::id_part_refusal;
use crate::marketplace::declared_source::{DeclaredSourceFacts, declared_source_refusal};
use crate::marketplace::dependency_graph::GraphFacts;
use crate::marketplace::dependency_install_plan::{InstallPlan, InstallRequest, install_closure};
use crate::marketplace::install_scopes::InstallScope;
use crate::marketplace::marketplace_input::{
    MarketplaceInputOptions, Parsed, parse_marketplace_input,
};

const ADD: MarketplaceCommand = MarketplaceCommand::MarketplaceAdd;

/// The command line.
#[derive(Clone, Debug, Deserialize)]
pub struct AddArguments {
    /// The source.
    pub source: String,
    /// The raw `--scope` value.
    #[serde(default)]
    pub scope: Option<String>,
    /// `--sparse`.
    #[serde(default)]
    pub sparse: Option<Vec<String>>,
    /// `--claudeai`: `source` is the name of a marketplace hosted on claude.ai.
    #[serde(default)]
    pub claudeai: Option<bool>,
    /// `--json`.
    #[serde(default)]
    pub json: Option<bool>,
    /// Whose path rules apply.
    #[serde(default)]
    pub platform: Option<PathPlatform>,
}

/// What to fetch, or what the command ends with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AddStart {
    /// Fetch this source.
    Fetch {
        /// The source.
        source: MarketplaceSource,
        /// The scope declared in.
        scope: InstallScope,
        /// The host prints the JSON line (`--json`).
        json: bool,
    },
    /// "`--json` ... has no effect with `--claudeai`": the human line is printed.
    Claudeai {
        /// The claude.ai marketplace.
        name: String,
        /// Always `false`.
        json: bool,
    },
    /// The command ends.
    Done {
        /// What it prints.
        output: CommandOutput,
    },
}

/// "`plugin marketplace add` and `plugin marketplace remove` can print no result line" for this name.
pub const RESERVED_QUIET_NAME: &str = "anthropic-plugin-directory";

/// The argument is the reserved name once surrounding whitespace is dropped (spec/judgments.md).
#[must_use]
pub fn is_reserved_name(argument: &str) -> bool {
    js::trim(argument) == RESERVED_QUIET_NAME
}

const fn done(output: CommandOutput) -> AddStart {
    AddStart::Done { output }
}

fn invalid_source(message: &str) -> AddStart {
    done(marketplace_failure(ADD, None, message, "invalid_source"))
}

fn claude_ai_start(args: &AddArguments) -> AddStart {
    if args.scope.is_some() {
        return done(usage_error("--claudeai cannot be combined with --scope"));
    }
    if args.sparse.is_some() {
        return done(usage_error("--claudeai cannot be combined with --sparse"));
    }
    AddStart::Claudeai {
        name: js::trim(&args.source).to_owned(),
        json: false,
    }
}

/// The source the text stands for, or what ends the command: `--sparse` on a source that takes none is a flag problem.
fn source_of(args: &AddArguments) -> Result<MarketplaceSource, Box<AddStart>> {
    let options = MarketplaceInputOptions {
        platform: args.platform,
        sparse: None,
    };
    let with_sparse = MarketplaceInputOptions {
        sparse: args.sparse.clone(),
        ..options
    };
    let issue = match parse_marketplace_input(&args.source, &with_sparse) {
        Parsed::Source(source) => return Ok(source),
        Parsed::Issue(issue) => issue,
    };
    Err(Box::new(
        match parse_marketplace_input(&args.source, &options) {
            Parsed::Source(_) => done(usage_error(&issue.message)),
            Parsed::Issue(_) => invalid_source(&issue.message),
        },
    ))
}

/// What to fetch, or what the command ends with before any fetch.
#[must_use]
pub fn start_marketplace_add(args: &AddArguments) -> AddStart {
    if is_reserved_name(&args.source) {
        let reason = format!("\"{RESERVED_QUIET_NAME}\" is a reserved marketplace name");
        return done(no_result_line(&reason));
    }
    if args.claudeai == Some(true) {
        return claude_ai_start(args);
    }
    let scope = match choose_scope(ScopedCommand::MarketplaceAdd, args.scope.as_deref()) {
        ChosenOrError::Error(output) => return done(output),
        ChosenOrError::Choice(ScopeChoice::Scope { scope }) => scope,
        // `marketplace-add` writes to one scope, `user` when `--scope` is omitted (`DefaultedCommand`).
        ChosenOrError::Choice(ScopeChoice::AutoDetect { .. } | ScopeChoice::AllScopes { .. }) => {
            InstallScope::User
        }
    };
    match source_of(args) {
        Err(end) => *end,
        Ok(source) => AddStart::Fetch {
            source,
            scope,
            json: args.json == Some(true),
        },
    }
}

/// What the host's fetch found.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddFetched {
    /// The `name` of the marketplace's own manifest.
    pub name: String,
    /// A copy of it is already on disk.
    #[serde(default)]
    pub already_on_disk: bool,
    /// The marketplaces already registered, for the download-folder rule.
    #[serde(default)]
    pub registered: Vec<RegisteredMarketplace>,
    /// The plugin graph with this marketplace's catalog read; absent when the host has no installed plugins to check.
    #[serde(default)]
    pub graph: Option<GraphFacts>,
    /// The source a user or managed `extraKnownMarketplaces` entry lists for this name; absent when none does.
    #[serde(default)]
    pub declared: Option<MarketplaceSource>,
    /// How many LFS-tracked files the clone left as pointer files (`github` and `git` sources).
    #[serde(default)]
    pub lfs_pointers: Option<f64>,
}

/// The `extraKnownMarketplaces` entry to write.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Declare {
    /// The scope.
    pub scope: InstallScope,
    /// The marketplace.
    pub name: String,
    /// Its source.
    pub source: MarketplaceSource,
}

/// What the add ends with.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AddConclusion {
    /// What it prints.
    pub output: CommandOutput,
    /// The `extraKnownMarketplaces` entry to write; absent when nothing is declared (a failure, a claude.ai marketplace).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declare: Option<Declare>,
    /// The dependency installs of installed plugins the new catalog can now satisfy, one plan per plugin.
    pub dependencies: Vec<InstallPlan>,
}

/// "After you add it, Claude Code installs any dependencies that your installed plugins were
/// missing": the install plan of every installed plugin the catalogs list that declares dependencies,
/// keeping the plans that install something, are refused or still `needs` a plugin fetched (the host
/// fetches it and plans that plugin again, `installClosure`). Judgment: a refused plan is reported, not
/// a failure of the add.
fn missing_dependencies(graph: Option<&GraphFacts>) -> Result<Vec<InstallPlan>, String> {
    let Some(graph) = graph else {
        return Ok(Vec::new());
    };
    let mut plans = Vec::new();
    for record in &graph.installed {
        if record.dependencies.is_empty() || !graph.catalog.contains_key(&record.id) {
            continue;
        }
        let request = InstallRequest {
            root: record.id.clone(),
            scope: record.scope,
            operation: "marketplace-add".to_owned(),
            host_version: None,
        };
        let plan = install_closure(&request, graph)?;
        if !plan.steps.is_empty() || !plan.ok {
            plans.push(plan);
        }
    }
    Ok(plans)
}

fn settings_label(scope: InstallScope) -> String {
    format!("{} settings", scope.as_str())
}

/// The name and the registered marketplaces (`Pick<AddFetched, 'name' | 'registered'>`).
#[derive(Clone, Debug, Deserialize)]
pub struct NameFacts {
    /// The marketplace's name.
    pub name: String,
    /// The marketplaces already registered.
    #[serde(default)]
    pub registered: Vec<RegisteredMarketplace>,
}

/// The first reason the name is refused, or `None`; `plugin install --marketplace` runs the same check.
#[must_use]
pub fn name_refusal(
    name: &str,
    registered: &[RegisteredMarketplace],
    source: Option<&MarketplaceSource>,
) -> Option<String> {
    if let Some(grammar) = id_part_refusal(name) {
        return Some(grammar);
    }
    let origin = match source {
        None => NameOrigin {
            origin: js::JsValue::Undefined,
            hosted_on_claude_ai: Some(true),
        },
        Some(source) => NameOrigin {
            origin: to_js_value(source).unwrap_or_default(),
            hosted_on_claude_ai: None,
        },
    };
    [
        validation_name_issues(name),
        load_name_issues(name, &origin),
        download_folder_issues(name, registered),
    ]
    .concat()
    .into_iter()
    .find(|issue| issue.severity == Severity::Error)
    .map(|issue| issue.message)
}

fn refused(found: &AddFetched, refusal: &str) -> AddConclusion {
    AddConclusion {
        output: marketplace_failure(ADD, Some(&found.name), refusal, "invalid_name"),
        declare: None,
        dependencies: Vec::new(),
    }
}

fn conclude_fetch(
    source: &MarketplaceSource,
    scope: InstallScope,
    found: &AddFetched,
) -> Result<AddConclusion, String> {
    let refusal = name_refusal(&found.name, &found.registered, Some(source)).or_else(|| {
        declared_source_refusal(&DeclaredSourceFacts {
            name: found.name.clone(),
            given: source.clone(),
            declared: found.declared.clone(),
        })
    });
    if let Some(refusal) = refusal {
        return Ok(refused(found, &refusal));
    }
    let place = settings_label(scope);
    let head = if found.already_on_disk {
        format!(
            "Marketplace '{}' already on disk — declared in {place}",
            found.name
        )
    } else {
        format!(
            "Successfully added marketplace: {} (declared in {place})",
            found.name
        )
    };
    let message = match lfs_pointer_line(found.lfs_pointers.unwrap_or(0.0)) {
        None => head,
        Some(lfs) => format!("{head}\n{lfs}"),
    };
    Ok(AddConclusion {
        output: marketplace_success(ADD, Some(&found.name), &message),
        declare: Some(Declare {
            scope,
            name: found.name.clone(),
            source: source.clone(),
        }),
        dependencies: missing_dependencies(found.graph.as_ref())?,
    })
}

/// A marketplace "hosted for your account, not declared in a settings file" (judgment: the line).
fn conclude_claude_ai(found: &AddFetched) -> Result<AddConclusion, String> {
    if let Some(refusal) = name_refusal(&found.name, &found.registered, None) {
        return Ok(refused(found, &refusal));
    }
    let message = format!("Successfully added marketplace: {}", found.name);
    Ok(AddConclusion {
        output: marketplace_success(ADD, Some(&found.name), &message),
        declare: None,
        dependencies: missing_dependencies(found.graph.as_ref())?,
    })
}

/// What the add ends with once the marketplace is fetched. A `done` start waits for no fetch: the
/// TypeScript's table has no concluder for it, and it is refused here.
pub fn conclude_marketplace_add(
    start: &AddStart,
    found: &AddFetched,
) -> Result<AddConclusion, String> {
    match start {
        AddStart::Fetch { source, scope, .. } => conclude_fetch(source, *scope, found),
        AddStart::Claudeai { .. } => conclude_claude_ai(found),
        AddStart::Done { .. } => Err("a done start waits for no fetch".to_owned()),
    }
}

crate::registry::exports! {
    "cli/marketplace-add" {
        "startMarketplaceAdd" => ok |args: AddArguments| start_marketplace_add(&args);
        "nameRefusal" => ok |found: NameFacts, source: Option<MarketplaceSource>|
            name_refusal(&found.name, &found.registered, source.as_ref());
        "concludeMarketplaceAdd" => fallible |start: AddStart, found: AddFetched| conclude_marketplace_add(&start, &found);
    }
}
