//! Port of `src/format/helper-credentials.ts`, switched.
//!
//! The variables Claude Code removes from the environment of a `headersHelper` that a project
//! `.mcp.json` or a plugin supplies (mcp.md, "Which variables a helper can read": "every variable
//! ... whose name looks like a credential, such as a name with `TOKEN`, `SECRET`, `PASSWORD`, `KEY`,
//! or `AUTH` in it in either letter case", Git's `GIT_CONFIG_KEY_<n>` apart, and "a fixed list of
//! credential variables whose names don't follow that pattern"). The page gives the rule only by
//! example; this is Claude Code's, observed with the spec's headers-helper-env oracle
//! (spec/judgments.md, "headersHelper credential names") and checked against the recorded result by
//! tests/loading/helper-credentials.test.ts.

use std::sync::LazyLock;

use regex::Regex;

use super::remote_mcp_credentials::reads_as_empty_in_remote_mcp;
use crate::js::{Pattern, regex_test};

/// A name looks like a credential, without regard to case, when a part between underscores is one
/// of the words, when a part ends in `TOKEN`, `SECRET`, `PASSWORD` or `PASSWD` (`FOOSECRET`, not
/// `TOKENIZER`), or when `PASS` or `JWT` is a part after the first (`SERVICE_JWT`, not `JWT`).
/// JavaScript's `i` without `u` folds ASCII letters only: `(?i-u:...)`.
static CREDENTIAL_LIKE: Pattern = LazyLock::new(|| {
    Regex::new(
        r"(?i-u:(?:^|_)(?:KEYS?|APIKEY|AUTH|CREDENTIALS?|CREDS|COOKIE|PAT|SECRETS|PASSWORDS)(?:_|$)|(?:TOKEN|SECRET|PASSWORD|PASSWD)(?:_|$)|_(?:PASS|JWT)(?:_|$))",
    )
    .ok()
});

/// Git's configuration by environment, `GIT_CONFIG_KEY_<n>` in capitals as Git reads it, stays.
static GIT_CONFIG_KEY: Pattern = LazyLock::new(|| Regex::new(r"^GIT_CONFIG_KEY_[0-9]+$").ok());

/// Of the names a remote server's `url` and `headers` read as empty, the ones a helper still
/// receives: the proxies and the package index addresses.
const HELPER_KEEPS: [&str; 11] = [
    "ALL_PROXY",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "CLAUDE_CODE_CLIENT_CERT",
    "GOPROXY",
    "PIP_EXTRA_INDEX_URL",
    "PIP_INDEX_URL",
    "UV_DEFAULT_INDEX",
    "UV_EXTRA_INDEX_URL",
    "UV_INDEX",
    "UV_INDEX_URL",
];

/// Whether a supplied `headersHelper` runs without `name` in its environment.
#[must_use]
pub fn removed_from_headers_helper(name: &str) -> bool {
    if regex_test(&GIT_CONFIG_KEY, name) {
        return false;
    }
    if regex_test(&CREDENTIAL_LIKE, name) {
        return true;
    }
    reads_as_empty_in_remote_mcp(name) && !HELPER_KEEPS.contains(&name.to_uppercase().as_str())
}

crate::registry::exports! {
    "format/helper-credentials" {
        "removedFromHeadersHelper" => ok |name: String| removed_from_headers_helper(&name);
    }
}
