//! Port of `src/format/credential-env.ts`, switched.
//!
//! The credential variables of mcp.md ("Credential variables that read as empty"): in a remote MCP
//! server's `url` and `headers` they read as empty, set or not, and a `:-default` on them is ignored,
//! so a project's `.mcp.json` or a plugin cannot send them to the server it names. The single home of
//! the list: plugin servers (`loading/resolve/surfaces`) and servers of the settings scopes
//! (`mcp-env`) read it here.
//!
//! The page lists names only "such as"; remote-mcp-credentials owns the oracle-proven full set.
//! The five names below are the documented examples, not the complete classifier.

use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::helper_credentials::removed_from_headers_helper;
use super::remote_mcp_credentials::reads_as_empty_in_remote_mcp;
use crate::js::{Pattern, SPACE_CLASS, key_order, regex_test};

/// The credential variables of mcp.md, as documented.
pub const CREDENTIAL_ENV_NAMES: [&str; 5] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "AWS_BEARER_TOKEN_BEDROCK",
    "HTTPS_PROXY",
    "NPM_TOKEN",
];

/// True for a name that reads as empty in the `url` and `headers` of a remote server.
#[must_use]
pub fn reads_as_empty_toward_remote(name: &str) -> bool {
    reads_as_empty_in_remote_mcp(name)
}

/// The phrase mcp.md tells the reader to search the debug file for.
pub const NOT_EXPANDED_TOWARD_REMOTE: &str = "never expanded toward a remote server";

/// The debug-log line for a covered variable the user has set and a remote server's `url` or
/// `headers` references. The docs give the phrase, not the whole line; the rest is a judgment. It
/// names the variable and never its value.
#[must_use]
pub fn credential_debug_line(name: &str) -> String {
    format!("Environment variable {name} is a credential and is {NOT_EXPANDED_TOWARD_REMOTE}")
}

/// The credential rule of every process the kit starts for a command it did not write (mcp.md,
/// "Which variables a helper can read"; plugins/host-marketplace.md, "Variables Claude Code removes"):
/// "Apart from Git's `GIT_CONFIG_KEY_<n>` variables, Claude Code removes every variable from your
/// environment whose name looks like a credential, such as a name with `TOKEN`, `SECRET`, `PASSWORD`,
/// `KEY`, or `AUTH` in it in either letter case ... Claude Code also removes a fixed list of
/// credential variables whose names don't follow that pattern, such as `ANTHROPIC_CUSTOM_HEADERS`."
/// The pattern is the docs' own. The fixed list is only partly named: `ANTHROPIC_CUSTOM_HEADERS` is
/// the docs', the OpenTelemetry exporter headers (which carry `Authorization` values) are the kit's
/// judgment (spec/judgments.md). JavaScript's `i` without `u` folds ASCII letters only (RS6).
static CREDENTIAL_NAME_PATTERN: Pattern =
    LazyLock::new(|| Regex::new(r"(?i-u:TOKEN|SECRET|PASSWORD|KEY|AUTH)").ok());
static GIT_CONFIG_KEY: Pattern = LazyLock::new(|| Regex::new(r"^GIT_CONFIG_KEY_[0-9]+$").ok());

/// The credential variables whose names do not follow the pattern.
pub const FIXED_CREDENTIAL_NAMES: [&str; 5] = [
    "ANTHROPIC_CUSTOM_HEADERS",
    "OTEL_EXPORTER_OTLP_HEADERS",
    "OTEL_EXPORTER_OTLP_TRACES_HEADERS",
    "OTEL_EXPORTER_OTLP_METRICS_HEADERS",
    "OTEL_EXPORTER_OTLP_LOGS_HEADERS",
];

/// True for a variable name that looks like a credential, by the pattern or the fixed list.
#[must_use]
pub fn is_credential_variable(name: &str) -> bool {
    if FIXED_CREDENTIAL_NAMES.contains(&name) {
        return true;
    }
    regex_test(&CREDENTIAL_NAME_PATTERN, name) && !regex_test(&GIT_CONFIG_KEY, name)
}

/// `environment` without its credential variables.
#[must_use]
pub fn without_credentials(environment: &IndexMap<String, String>) -> IndexMap<String, String> {
    key_order(environment.keys())
        .into_iter()
        .filter(|name| !removed_from_headers_helper(name))
        .filter_map(|name| {
            environment
                .get(name)
                .map(|value| (name.clone(), value.clone()))
        })
        .collect()
}

/// `scheme://` and the userinfo up to its `@`, for each URL in a value.
static USERINFO: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"((?i-u:[a-z][a-z0-9+.-]*)://)[^/?#@{SPACE_CLASS}]*@"
    ))
    .ok()
});
static PASSWORD_IN_USERINFO: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i-u:[a-z][a-z0-9+.-]*)://[^/?#@:{SPACE_CLASS}]*:[^/?#@{SPACE_CLASS}]+@"
    ))
    .ok()
});

/// Whether `value` holds a URL whose userinfo carries a password: "a value that looks like a credential".
#[must_use]
pub fn has_url_password(value: &str) -> bool {
    regex_test(&PASSWORD_IN_USERINFO, value)
}

/// `value` with the username and password cut from every URL in it.
#[must_use]
pub fn without_userinfo(value: &str) -> String {
    USERINFO.as_ref().map_or_else(
        || value.to_owned(),
        |pattern| pattern.replace_all(value, "${1}").into_owned(),
    )
}

/// What the scrub does to one variable (env-vars.md, "What the subprocess environment scrub removes").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrubAction {
    /// The variable goes.
    #[serde(rename = "remove")]
    Remove,
    /// The variable stays.
    #[serde(rename = "keep")]
    Keep,
    /// The variable stays, its URL without the username and password.
    #[serde(rename = "cut-userinfo")]
    CutUserinfo,
}

/// The scrub's rules (env-vars.md, "What the subprocess environment scrub removes"): the table is the
/// docs', the rest is oracle 2.1.321, a hook printing its environment under the scrub
/// (spec/judgments.md). Every name compares without regard to case.
/// - A name is a credential when a `_`-separated word of it is one of `CREDENTIAL_WORDS`: `MY_AUTH`,
///   `FOO_KEY`, `MY_PASS`, `SSH_AUTH_SOCK` go, `MONKEY`, `TOKENIZER`, `TOKENS`, `OAUTH`, `KEYCHAIN` stay.
/// - A value is a credential when it holds a URL with a password, a PEM private key or a JWT; a token
///   by its prefix alone (`ghp_`, `sk-ant-`, `AKIA`) and a public key stay.
/// - The registry URLs keep the URL without its userinfo, a bare token as the username included.
const CREDENTIAL_WORDS: [&str; 13] = [
    "TOKEN",
    "PASS",
    "PASSWD",
    "PASSWORD",
    "SECRET",
    "SECRETS",
    "CREDENTIAL",
    "CREDENTIALS",
    "KEY",
    "APIKEY",
    "PRIVATEKEY",
    "ACCESSKEY",
    "AUTH",
];

/// "Leaves it in place": the GitHub tokens, so `gh` keeps working, and the proxies.
const KEPT_NAMES: [&str; 6] = [
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
    "HTTP_PROXY",
    "HTTPS_PROXY",
];

/// "Keeps the URL and cuts the username and password from it".
const REGISTRY_URL_NAMES: [&str; 6] = [
    "PIP_INDEX_URL",
    "PIP_EXTRA_INDEX_URL",
    "NPM_CONFIG_REGISTRY",
    "UV_INDEX_URL",
    "UV_EXTRA_INDEX_URL",
    "YARN_REGISTRY",
];

/// `CLAUDE_CONFIG_DIR` and `FIXED_CREDENTIAL_NAMES`.
fn is_removed_name(key: &str) -> bool {
    key == "CLAUDE_CONFIG_DIR" || FIXED_CREDENTIAL_NAMES.contains(&key)
}

static GIT_CONFIG_ENTRY: Pattern =
    LazyLock::new(|| Regex::new(r"^GIT_CONFIG_(?:COUNT|KEY_[0-9]+|VALUE_[0-9]+)$").ok());
static PRIVATE_KEY_BLOCK: Pattern =
    LazyLock::new(|| Regex::new(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----").ok());
static JWT: Pattern = LazyLock::new(|| {
    Regex::new(r"(?-u:\b)eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]*").ok()
});

fn name_is_credential(key: &str) -> bool {
    key.split('_').any(|word| CREDENTIAL_WORDS.contains(&word))
}

fn value_is_credential(value: &str) -> bool {
    has_url_password(value) || regex_test(&PRIVATE_KEY_BLOCK, value) || regex_test(&JWT, value)
}

type ScrubRule = fn(&str, &str) -> Option<ScrubAction>;

/// First rule that decides wins: what the docs leave in place whatever it holds comes first.
const SCRUB_RULES: [ScrubRule; 4] = [
    |key, _| {
        (regex_test(&GIT_CONFIG_ENTRY, key) || KEPT_NAMES.contains(&key))
            .then_some(ScrubAction::Keep)
    },
    |key, _| (is_removed_name(key) || name_is_credential(key)).then_some(ScrubAction::Remove),
    |key, value| {
        (REGISTRY_URL_NAMES.contains(&key) && without_userinfo(value) != value)
            .then_some(ScrubAction::CutUserinfo)
    },
    |_key, value| value_is_credential(value).then_some(ScrubAction::Remove),
];

/// What the scrub does to the variable `name` holding `value`; a variable no rule names is kept.
#[must_use]
pub fn scrub_action(name: &str, value: &str) -> ScrubAction {
    let key = name.to_uppercase();
    SCRUB_RULES
        .iter()
        .find_map(|rule| rule(&key, value))
        .unwrap_or(ScrubAction::Keep)
}

crate::registry::exports! {
    "format/credential-env" {
        "CREDENTIAL_ENV_NAMES" => ok || CREDENTIAL_ENV_NAMES;
        "readsAsEmptyTowardRemote" => ok |name: String| reads_as_empty_toward_remote(&name);
        "NOT_EXPANDED_TOWARD_REMOTE" => ok || NOT_EXPANDED_TOWARD_REMOTE;
        "credentialDebugLine" => ok |name: String| credential_debug_line(&name);
        "FIXED_CREDENTIAL_NAMES" => ok || FIXED_CREDENTIAL_NAMES;
        "isCredentialVariable" => ok |name: String| is_credential_variable(&name);
        "withoutCredentials" => ok |environment: IndexMap<String, String>| without_credentials(&environment);
        "hasUrlPassword" => ok |value: String| has_url_password(&value);
        "withoutUserinfo" => ok |value: String| without_userinfo(&value);
        "scrubAction" => ok |name: String, value: String| scrub_action(&name, &value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'import { withoutUserinfo, hasUrlPassword, scrubAction } from "./src/format/credential-env.ts";
    //   console.log(JSON.stringify([withoutUserinfo("a https://u:p@host/x and ftp://tok@h and HTTPS://x@y"),
    //   hasUrlPassword("https://u:p@h"), hasUrlPassword("https://u@h"), scrubAction("x", "eyJabc.eyJdef.ghi"),
    //   scrubAction("x", "xeyJabc.eyJdef.ghi"), scrubAction("x", "-----BEGIN RSA PRIVATE KEY-----"),
    //   scrubAction("pip_index_url", "https://tok@pypi/simple"), scrubAction("GIT_CONFIG_KEY_0", "x"),
    //   scrubAction("ssh_auth_sock", "x")]))'
    // prints: ["a https://host/x and ftp://h and HTTPS://y",true,false,"remove","keep","remove",
    //   "cut-userinfo","keep","remove"]
    #[test]
    fn the_patterns_compile_and_answer_as_javascript_does() {
        assert!(USERINFO.is_some() && PASSWORD_IN_USERINFO.is_some() && JWT.is_some());
        assert_eq!(
            without_userinfo("a https://u:p@host/x and ftp://tok@h and HTTPS://x@y"),
            "a https://host/x and ftp://h and HTTPS://y"
        );
        assert!(has_url_password("https://u:p@h"));
        assert!(!has_url_password("https://u@h"));
        assert_eq!(scrub_action("x", "eyJabc.eyJdef.ghi"), ScrubAction::Remove);
        assert_eq!(scrub_action("x", "xeyJabc.eyJdef.ghi"), ScrubAction::Keep);
        assert_eq!(
            scrub_action("x", "-----BEGIN RSA PRIVATE KEY-----"),
            ScrubAction::Remove
        );
        assert_eq!(
            scrub_action("pip_index_url", "https://tok@pypi/simple"),
            ScrubAction::CutUserinfo
        );
        assert_eq!(scrub_action("GIT_CONFIG_KEY_0", "x"), ScrubAction::Keep);
        assert_eq!(scrub_action("ssh_auth_sock", "x"), ScrubAction::Remove);
    }
}
