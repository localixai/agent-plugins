//! Port of `src/format/remote-mcp-credentials.ts`, switched.
//!
//! The `${VAR}` names a remote MCP server's `url` and `headers` read as empty, whether or not the
//! variable is set and whatever `:-default` it carries (mcp.md, "Credential variables that read as
//! empty"). The page lists them only "such as"; the set here is Claude Code's, observed with
//! the spec's remote-mcp-env oracle (spec/judgments.md, "Remote MCP credential names"), and
//! tests/loading/remote-mcp-credentials.test.ts checks it against the recorded result.
//!
//! Names compare without regard to case (`npm_token`, `Http_Proxy` are emptied too).

use std::sync::LazyLock;

use regex::Regex;

use crate::js::{Pattern, regex_test};

/// Exact names, upper-cased.
const NAMES: [&str; 61] = [
    // Proxies (NO_PROXY is not covered)
    "ALL_PROXY",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    // Claude Code's own credentials and session
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_AWS_API_KEY",
    "ANTHROPIC_CUSTOM_HEADERS",
    "ANTHROPIC_FOUNDRY_API_KEY",
    "ANTHROPIC_FOUNDRY_AUTH_TOKEN",
    "ANTHROPIC_IDENTITY_TOKEN",
    "ANTHROPIC_IDENTITY_TOKEN_FILE",
    "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
    "CLAUDE_CODE_CLIENT_CERT",
    "CLAUDE_CODE_CLIENT_KEY",
    "CLAUDE_CODE_CLIENT_KEY_PASSPHRASE",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
    "CLAUDE_CODE_OTEL_DIAG_STDERR",
    "CLAUDE_CODE_RESUME_INTERRUPTED_TURN",
    "CLAUDE_CODE_RESUME_INTERRUPTED_TURN_MAX_AGE_MS",
    "CLAUDE_CODE_RESUME_PROMPT",
    "CLAUDE_CODE_RESUME_SHOWS_CUT_OFF_CALLS",
    "CLAUDE_CODE_WEBSOCKET_AUTH_FILE_DESCRIPTOR",
    "CLAUDE_SESSION_INGRESS_TOKEN_FILE",
    "MCP_CLIENT_SECRET",
    "SELF_HOSTED_RUNNER_ENVIRONMENT_SECRET",
    "SELF_HOSTED_RUNNER_POOL_SECRET",
    // Cloud providers (AWS_ACCESS_KEY_ID, AWS_PROFILE, AZURE_CLIENT_ID are not covered)
    "AWS_BEARER_TOKEN_BEDROCK",
    "AWS_CONTAINER_AUTHORIZATION_TOKEN",
    "AWS_CONTAINER_CREDENTIALS_FULL_URI",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AWS_WEB_IDENTITY_TOKEN_FILE",
    "AZURE_CLIENT_CERTIFICATE_PASSWORD",
    "AZURE_CLIENT_CERTIFICATE_PATH",
    "AZURE_CLIENT_SECRET",
    "AZURE_FEDERATED_TOKEN_FILE",
    "CLOUDSDK_AUTH_ACCESS_TOKEN_FILE",
    "GOOGLE_APPLICATION_CREDENTIALS",
    "GOOGLE_OAUTH_ACCESS_TOKEN",
    "VAULT_TOKEN",
    // Package registries (GITHUB_TOKEN, GH_TOKEN, OPENAI_API_KEY, YARN_NPM_AUTH_TOKEN expand)
    "CARGO_REGISTRY_TOKEN",
    "CODEARTIFACT_AUTH_TOKEN",
    "GEM_HOST_API_KEY",
    "GOPROXY",
    "NODE_AUTH_TOKEN",
    "NPM_TOKEN",
    "PIP_EXTRA_INDEX_URL",
    "PIP_INDEX_URL",
    "PYPI_TOKEN",
    "TWINE_PASSWORD",
    "TWINE_USERNAME",
    "UV_DEFAULT_INDEX",
    "UV_EXTRA_INDEX_URL",
    "UV_INDEX",
    "UV_INDEX_URL",
    "UV_PUBLISH_PASSWORD",
    "UV_PUBLISH_TOKEN",
    "UV_PUBLISH_USERNAME",
];

/// Name families, upper-cased: OpenTelemetry settings and per-registry credentials.
static PATTERNS: [Pattern; 5] = [
    LazyLock::new(|| Regex::new(r"^OTEL_").ok()),
    LazyLock::new(|| Regex::new(r"^POETRY_PYPI_TOKEN_").ok()),
    LazyLock::new(|| Regex::new(r"^POETRY_HTTP_BASIC_").ok()),
    LazyLock::new(|| Regex::new(r"^CARGO_REGISTRIES_[^\n\r\u{2028}\u{2029}]+_TOKEN$").ok()),
    // Bundler's per-host credential, the host's dots written `__` (`BUNDLE_GEMS__EXAMPLE__COM`).
    LazyLock::new(|| Regex::new(r"^BUNDLE_[^\n\r\u{2028}\u{2029}]+__").ok()),
];

/// Whether a remote MCP server's `url` or `headers` reads `${name}` as empty.
#[must_use]
pub fn reads_as_empty_in_remote_mcp(name: &str) -> bool {
    let upper = name.to_uppercase();
    NAMES.contains(&upper.as_str()) || PATTERNS.iter().any(|pattern| regex_test(pattern, &upper))
}

crate::registry::exports! {
    "format/remote-mcp-credentials" {
        "readsAsEmptyInRemoteMcp" => ok |name: String| reads_as_empty_in_remote_mcp(&name);
    }
}
