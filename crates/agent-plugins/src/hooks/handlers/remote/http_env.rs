//! Ported from `src/hooks/handlers/remote/http-env.ts`; only the core uses it now, so it has no wrapper.
//!
//! Environment variables in the headers of an `http` hook (hooks.md, "HTTP hook fields"):
//! `headers` values "support environment variable interpolation using `$VAR_NAME` or `${VAR_NAME}`
//! syntax. Only variables listed in `allowedEnvVars` are resolved", "References to unlisted
//! variables are replaced with empty strings. Required for any env var interpolation to work."

use indexmap::IndexMap;

use crate::js::in_key_order;
use crate::ports::environment::Environment;

/// The name a reference at the start of `text` holds, and the length of the reference: `${NAME}`
/// or `$NAME`; a `$` that starts neither is plain text
/// (`/\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)/`).
fn reference(text: &str) -> Option<(&str, usize)> {
    let rest = text.strip_prefix('$')?;
    let name_length = |text: &str| {
        let mut chars = text.bytes();
        match chars.next() {
            Some(first) if first.is_ascii_alphabetic() || first == b'_' => {
                1 + chars
                    .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                    .count()
            }
            _ => 0,
        }
    };
    if let Some(braced) = rest.strip_prefix('{') {
        let length = name_length(braced);
        if length > 0 && braced.get(length..)?.starts_with('}') {
            return Some((braced.get(..length)?, length + 3));
        }
    }
    let length = name_length(rest);
    (length > 0).then(|| (rest.get(..length).unwrap_or_default(), length + 1))
}

/// One header value with its references resolved. Judgment (spec/judgments.md): the text of a
/// variable is not read again (one pass), and its line breaks are dropped.
async fn interpolate_header(
    value: &str,
    allowed: &[String],
    environment: &dyn Environment,
) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(character) = rest.chars().next() {
        if let Some((name, length)) = reference(rest) {
            if allowed.iter().any(|each| each == name) {
                let held = environment.get(name).await.unwrap_or_default();
                out.extend(held.chars().filter(|each| !matches!(each, '\r' | '\n')));
            }
            rest = rest.get(length..).unwrap_or_default();
        } else {
            out.push(character);
            rest = rest.get(character.len_utf8()..).unwrap_or_default();
        }
    }
    out
}

/// Every header of the hook with its value interpolated; names are lower case (the port's rule).
pub async fn interpolate_headers(
    headers: Option<&IndexMap<String, String>>,
    allowed_env_vars: Option<&[String]>,
    environment: &dyn Environment,
) -> IndexMap<String, String> {
    let allowed = allowed_env_vars.unwrap_or_default();
    let mut interpolated = IndexMap::new();
    for (name, value) in headers.into_iter().flatten() {
        interpolated.insert(
            name.to_lowercase(),
            interpolate_header(value, allowed, environment).await,
        );
    }
    in_key_order(interpolated)
}
