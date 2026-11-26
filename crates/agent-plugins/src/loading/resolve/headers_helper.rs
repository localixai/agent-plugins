//! Port of `src/loading/resolve/headers-helper.ts`, switched.
//!
//! What to do with a server's `headersHelper` each time the server connects (mcp.md, "Use dynamic
//! headers for custom authentication").

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::headers_helper_origins::{
    DirectoryKind, HeadersHelperOrigin, HelperContext, directory_of, origin_rule,
};
use super::headers_helper_trust::{GateResult, HelperNotice, HelperTrust, pass_gate};
use crate::format::credential_env::is_credential_variable;
use crate::js::{code_unit_length, encode_uri_component, in_key_order, key_order};
use crate::registry::Port;

/// The server a helper belongs to: the name Claude Code shows, its URL as expanded, and the command.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperServer {
    /// The name Claude Code shows.
    pub name: String,
    /// The URL as expanded.
    pub url: String,
    /// The environment variables the expansion of `url` substituted (the `environment` entries of
    /// `Substituted.referenced`). Only their live values are redacted from the URL the helper
    /// receives; a value that merely occurs in the URL is not.
    pub expanded_variables: Vec<String>,
    /// The command.
    #[serde(default)]
    pub headers_helper: Option<String>,
    /// The path variables `headersHelper` references, for the helper's environment: with the default
    /// `SubstitutionScope.shellPaths` the command holds `${CLAUDE_PLUGIN_DATA}` and the like rather than
    /// the path, so these give the references a value (judgment, spec/judgments.md). Absent or empty
    /// for a pasted command and for a server no plugin supplies.
    #[serde(default)]
    pub paths: Option<IndexMap<String, String>>,
}

/// mcp.md: "gives up on it after 10 seconds".
pub const HEADERS_HELPER_TIMEOUT_MS: u32 = 10_000;

/// The three variables of "Claude Code sets these environment variables when executing the helper".
fn helper_variables(
    name: &str,
    url: String,
    plugin_root: Option<&String>,
) -> IndexMap<String, String> {
    let mut variables = IndexMap::new();
    variables.insert("CLAUDE_CODE_MCP_SERVER_NAME".to_owned(), name.to_owned());
    variables.insert("CLAUDE_CODE_MCP_SERVER_URL".to_owned(), url);
    if let Some(root) = plugin_root {
        variables.insert("CLAUDE_PLUGIN_ROOT".to_owned(), root.clone());
    }
    variables
}

const OWN_VARIABLES: [&str; 3] = [
    "CLAUDE_CODE_MCP_SERVER_NAME",
    "CLAUDE_CODE_MCP_SERVER_URL",
    "CLAUDE_PLUGIN_ROOT",
];

const REDACTED: &str = "REDACTED";

/// "The `CLAUDE_CODE_MCP_SERVER_URL` value the helper receives has that part replaced with `REDACTED`":
/// the live value of each removed variable the URL expanded, longest first, in the URL as written and
/// URL-encoded (judgment: an embedded value with a space or a slash is written encoded). An empty
/// value matches nothing, and a removed variable the URL did not expand is never looked for.
fn redacted(url: &str, removed: &[(&String, &String)], expanded: &[String]) -> String {
    let mut values: Vec<String> = removed
        .iter()
        .filter(|(name, value)| !value.is_empty() && expanded.contains(name))
        .flat_map(|(_, value)| [(*value).clone(), encode_uri_component(value)])
        .collect();
    values.sort_by_key(|value| std::cmp::Reverse(code_unit_length(value)));
    values.iter().fold(url.to_owned(), |text, value| {
        text.replace(value.as_str(), REDACTED)
    })
}

/// What the helper starts with: the inherited environment, less credentials where they are removed, plus the three.
fn helper_environment(
    server: &HelperServer,
    origin: HeadersHelperOrigin,
    context: &HelperContext<Port>,
) -> IndexMap<String, String> {
    let rule = origin_rule(origin);
    let inherited: Vec<(&String, &String)> = in_key_order_entries(&context.environment)
        .into_iter()
        .filter(|(name, _)| !OWN_VARIABLES.contains(&name.as_str()))
        .collect();
    let removed: Vec<(&String, &String)> = if rule.removes_credentials {
        inherited
            .iter()
            .copied()
            .filter(|(name, _)| is_credential_variable(name))
            .collect()
    } else {
        Vec::new()
    };
    let mut environment: IndexMap<String, String> = inherited
        .iter()
        .filter(|(name, _)| !rule.removes_credentials || !is_credential_variable(name))
        .map(|(name, value)| ((*name).clone(), (*value).clone()))
        .collect();
    if let Some(paths) = &server.paths {
        environment.extend(paths.clone());
    }
    let plugin_root = if rule.provides_plugin_root {
        context.plugin_root.as_ref()
    } else {
        None
    };
    let url = redacted(&server.url, &removed, &server.expanded_variables);
    environment.extend(helper_variables(&server.name, url, plugin_root));
    in_key_order(environment)
}

/// `Object.entries` of an environment: integer keys first, as a JavaScript object orders them.
fn in_key_order_entries(environment: &IndexMap<String, String>) -> Vec<(&String, &String)> {
    key_order(environment.keys())
        .into_iter()
        .filter_map(|name| environment.get_key_value(name))
        .collect()
}

/// What to do with a server's `headersHelper`.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum HeadersHelperPlan {
    /// Run it.
    Run(HelperRun),
    /// The server connects with its static `headers` alone.
    Skipped(HelperSkipped),
    /// An agent file's server in a folder that is not trusted: not loaded, so nothing connects.
    NotLoaded {
        /// Always false.
        run: bool,
        /// Always false.
        loaded: bool,
    },
}

/// The helper runs.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's JSON shape: one flag per fact"
)]
pub struct HelperRun {
    /// Always true.
    pub run: bool,
    /// Always true.
    pub loaded: bool,
    /// Always false.
    pub headers_only: bool,
    /// The command.
    pub command: String,
    /// Which directory it runs in.
    pub working_directory: DirectoryKind,
    /// That directory.
    pub cwd: String,
    /// Its environment.
    pub environment: IndexMap<String, String>,
    /// Credential variables were taken out of `environment`.
    pub removes_credentials: bool,
    /// How long it may take.
    pub timeout_ms: u32,
}

/// The server connects with its static `headers` alone; `notice` is the refusal a trust gate prints.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperSkipped {
    /// Always false.
    pub run: bool,
    /// Always true.
    pub loaded: bool,
    /// Always true.
    pub headers_only: bool,
    /// The refusal a trust gate prints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<HelperNotice>,
}

const NO_HELPER: HelperSkipped = HelperSkipped {
    run: false,
    loaded: true,
    headers_only: true,
    notice: None,
};

/// A closed gate: an unloaded server stays unloaded whatever it holds; a loaded one connects with its
/// static headers, and the notice is printed only for a server whose helper was skipped (errors.md,
/// "headersHelper not run"), so a server with no helper gets none.
fn refused(gate: GateResult, server: &HelperServer) -> HeadersHelperPlan {
    let GateResult::Refused { notice, .. } = gate else {
        return HeadersHelperPlan::NotLoaded {
            run: false,
            loaded: false,
        };
    };
    if server.headers_helper.is_none() {
        HeadersHelperPlan::Skipped(NO_HELPER)
    } else {
        HeadersHelperPlan::Skipped(HelperSkipped {
            notice: Some(notice),
            ..NO_HELPER
        })
    }
}

/// What to do with a server's `headersHelper` each time the server connects (mcp.md, "Use dynamic
/// headers for custom authentication"): where to run it, with what environment, or that it does not
/// run. Pure; the host runs it through `runHeadersHelper` and the plan is made again on every
/// connection ("the helper fresh on each connection ... doesn't cache the result").
pub async fn plan_headers_helper(
    server: &HelperServer,
    origin: HeadersHelperOrigin,
    trust: &HelperTrust,
    context: &HelperContext<Port>,
) -> HeadersHelperPlan {
    let rule = origin_rule(origin);
    let gate = pass_gate(rule.gate, &server.name, trust, context).await;
    if !matches!(gate, GateResult::Open { .. }) {
        return refused(gate, server);
    }
    let cwd = directory_of(rule.directory, context).await;
    let (Some(command), Some(cwd)) = (&server.headers_helper, cwd) else {
        return HeadersHelperPlan::Skipped(NO_HELPER);
    };
    HeadersHelperPlan::Run(HelperRun {
        run: true,
        loaded: true,
        headers_only: false,
        command: command.clone(),
        working_directory: rule.directory,
        cwd,
        environment: helper_environment(server, origin, context),
        removes_credentials: rule.removes_credentials,
        timeout_ms: HEADERS_HELPER_TIMEOUT_MS,
    })
}

crate::registry::exports! {
    "loading/resolve/headers-helper" {
        "HEADERS_HELPER_TIMEOUT_MS" => ok || HEADERS_HELPER_TIMEOUT_MS;
        "planHeadersHelper" => async ok |server: HelperServer, origin: HeadersHelperOrigin, trust: HelperTrust, context: HelperContext<Port>|
            plan_headers_helper(&server, origin, &trust, &context).await;
    }
}
