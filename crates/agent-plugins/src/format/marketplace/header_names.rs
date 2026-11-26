//! Port of `src/format/marketplace/header-names.ts`, switched.
//!
//! The request-routing and client-identity header names a catalog entry may not set
//! (plugins/host-marketplace.md, "When Claude Code skips a headersHelper command or drops its
//! output": "Claude Code drops request-routing and client-identity names such as `Host`, `Cookie`,
//! and `X-Forwarded-*` ... and keeps authentication names such as `Authorization`").
//!
//! The docs say "such as": the set is the one `claude plugin validate` 2.1.325 warns for, the spec's
//! Judgment J15.12 (spec/judgments.md). The one copy: the catalog validator, the archive download
//! and the settings-file filter all read it.

const EXACT: [&str; 21] = [
    "host",
    "cookie",
    "forwarded",
    "x-real-ip",
    "x-client-ip",
    "true-client-ip",
    "client-ip",
    "cf-connecting-ip",
    "fastly-client-ip",
    "x-originating-ip",
    "x-remote-ip",
    "x-remote-addr",
    "x-cluster-client-ip",
    "connection",
    "upgrade",
    "transfer-encoding",
    "content-length",
    "te",
    "trailer",
    "expect",
    "via",
];
const PREFIXES: [&str; 3] = ["x-forwarded-", "x-original-", "proxy-"];

/// True for a routing or identity name, lowercased with `_` read as `-` (J15.12).
#[must_use]
pub fn is_routing_header(name: &str) -> bool {
    let folded = name.to_lowercase().replace('_', "-");
    EXACT.contains(&folded.as_str()) || PREFIXES.iter().any(|prefix| folded.starts_with(prefix))
}

crate::registry::exports! {
    "format/marketplace/header-names" {
        "isRoutingHeader" => ok |name: String| is_routing_header(&name);
    }
}
