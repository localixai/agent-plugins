//! Port of `src/agents/builtin-model.ts`, switched.
//!
//! The model a built-in subagent runs on when the model order of "Choose a model" ends at the main
//! conversation's model (sub-agents.md, "Built-in subagents"). Only Explore has a rule: "When the main
//! conversation runs Fable, Explore's model depends on how you connect".

use serde::{Deserialize, Serialize};

/// How the session reaches Claude, as far as the Explore rule tells the ways apart.
pub const CONNECTIONS: [Connection; 8] = [
    Connection::Subscription,
    Connection::Console,
    Connection::LlmGateway,
    Connection::Bedrock,
    Connection::Vertex,
    Connection::Foundry,
    Connection::ClaudePlatformOnAws,
    Connection::ClaudeAppsGateway,
];

/// One of `CONNECTIONS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Connection {
    /// A Claude subscription.
    Subscription,
    /// An Anthropic Console account.
    Console,
    /// An LLM gateway reached through `ANTHROPIC_BASE_URL`.
    LlmGateway,
    /// Amazon Bedrock.
    Bedrock,
    /// Google Cloud's Agent Platform.
    Vertex,
    /// Microsoft Foundry.
    Foundry,
    /// Claude Platform on AWS.
    ClaudePlatformOnAws,
    /// A Claude apps gateway.
    ClaudeAppsGateway,
}

/// A built-in, and the connection it is run over.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinModelInput {
    /// The built-in's name.
    pub name: String,
    /// How the session reaches Claude.
    pub connection: Connection,
}

/// Whether Fable on the main conversation moves Explore to the `opus` alias: "With a Claude subscription,
/// an Anthropic Console account, or an LLM gateway reached through `ANTHROPIC_BASE_URL`"; "On Amazon
/// Bedrock, Google Cloud's Agent Platform, Microsoft Foundry, Claude Platform on AWS, or a Claude apps
/// gateway, Explore stays on the main conversation's model".
const fn fable_moves_to_opus(connection: Connection) -> bool {
    match connection {
        Connection::Subscription | Connection::Console | Connection::LlmGateway => true,
        Connection::Bedrock
        | Connection::Vertex
        | Connection::Foundry
        | Connection::ClaudePlatformOnAws
        | Connection::ClaudeAppsGateway => false,
    }
}

/// The alias a rule picks; the host resolves it to a version.
const OPUS: &str = "opus";

type Rule = fn(&BuiltinModelInput, Option<&str>) -> Option<&'static str>;

fn explore_rule(input: &BuiltinModelInput, main_family: Option<&str>) -> Option<&'static str> {
    (main_family == Some("fable") && fable_moves_to_opus(input.connection)).then_some(OPUS)
}

/// The built-ins that have a model rule of their own; every other follows the main model.
fn rule_of(name: &str) -> Option<Rule> {
    match name {
        "Explore" => Some(explore_rule),
        _ => None,
    }
}

/// The alias a built-in's rule picks, or `None` when it follows the main conversation's model.
#[must_use]
pub fn builtin_model_alias(
    input: Option<&BuiltinModelInput>,
    main_family: Option<&str>,
) -> Option<&'static str> {
    let input = input?;
    rule_of(&input.name).and_then(|rule| rule(input, main_family))
}

crate::registry::exports! {
    "agents/builtin-model" {
        "CONNECTIONS" => ok || CONNECTIONS;
    }
}
