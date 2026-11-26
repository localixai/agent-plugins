//! Port of `src/format/mcp-prompts.ts`, switched.
//!
//! MCP prompts as commands (mcp.md, "Use MCP prompts as commands", "Execute MCP prompts"). A connected
//! server's prompts are listed as `/servername:promptname (MCP)` and run by typing
//! `/mcp__servername__promptname` followed by space-separated arguments. Pure over the prompts the
//! host listed with `prompts/list`; `prompts/get` and injecting the result into the conversation are the
//! host's.
//!
//! An `McpPromptDefinition` is `Pick<Prompt, 'name' | 'description'>` with a `server` and its
//! `arguments`; the host's prompt objects may carry more, and `listablePrompts` answers the same
//! objects, so a definition is a `JsValue` here.

use serde::Serialize;

use super::mcp_tool::normalize_mcp_server_name;
use crate::js::{JsValue, in_key_order, is_whitespace, trim};
use indexmap::IndexMap;

/// A prompt as the host listed it, with the `server` it comes from (`McpPromptDefinition`).
pub type McpPromptDefinition = JsValue;

/// The parameters of `prompts/get`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct McpPromptInvocation {
    /// The server.
    pub server: String,
    /// The prompt.
    pub prompt: String,
    /// Its arguments by name.
    pub arguments: IndexMap<String, String>,
}

/// What reading a typed line gave.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum McpPromptRead {
    /// The line is no prompt command.
    NotAPrompt,
    /// The prompt to run.
    Invoke {
        /// Its parameters.
        invocation: McpPromptInvocation,
    },
    /// The line names a prompt that cannot run as typed.
    Refused {
        /// Why.
        message: String,
    },
}

/// "Prompts from a server named `anthropic-skills` don't appear": the name is reserved for synced skills.
const RESERVED_SERVER: &str = "anthropic-skills";

fn server_of(prompt: &McpPromptDefinition) -> Option<&str> {
    prompt.get("server").and_then(JsValue::as_str)
}

/// The prompts a host lists.
#[must_use]
pub fn listable_prompts(prompts: &[McpPromptDefinition]) -> Vec<McpPromptDefinition> {
    prompts
        .iter()
        .filter(|prompt| server_of(prompt) != Some(RESERVED_SERVER))
        .cloned()
        .collect()
}

/// "Claude Code lists each MCP prompt as `/servername:promptname (MCP)`". Judgment (spec/judgments.md):
/// both names as configured and declared, since only the typed form is said to be normalised.
#[must_use]
pub fn mcp_prompt_listing(server: &str, prompt: &str) -> String {
    format!("/{server}:{prompt} (MCP)")
}

const COMMAND_PREFIX: &str = "mcp__";

/// "In the `/mcp__servername__promptname` form, Claude Code replaces any character in the server name
/// outside `A-Z`, `a-z`, `0-9`, `_`, and `-` with `_`, and uses the prompt name as the server declares it."
#[must_use]
pub fn mcp_prompt_command(server: &str, prompt: &str) -> String {
    format!(
        "{COMMAND_PREFIX}{}__{prompt}",
        normalize_mcp_server_name(server)
    )
}

fn refused(message: String) -> McpPromptRead {
    McpPromptRead::Refused { message }
}

/// "Pass them space-separated after the command. Claude Code splits the arguments on whitespace, so
/// each argument is a single token": no quoting. They go to the prompt's defined parameters in order.
/// Judgments (spec/judgments.md): a prompt takes no more arguments than it declares and needs its
/// required ones; two prompts with one command are answered by the first listed; the wording of a refusal.
#[must_use]
pub fn read_mcp_prompt_invocation(input: &str, prompts: &[McpPromptDefinition]) -> McpPromptRead {
    // `input.trim().split(/\s+/)`: a run of blanks separates; nothing is left empty but a blank input.
    let mut parts = trim(input)
        .split(is_whitespace)
        .filter(|part| !part.is_empty());
    let word = parts.next().unwrap_or("");
    let tokens: Vec<&str> = parts.collect();
    let Some(command) = word.strip_prefix(&format!("/{COMMAND_PREFIX}")) else {
        return McpPromptRead::NotAPrompt;
    };
    let command = format!("{COMMAND_PREFIX}{command}");
    let listable = listable_prompts(prompts);
    let found = listable.iter().find(|each| {
        mcp_prompt_command(
            server_of(each).unwrap_or_default(),
            each.get("name")
                .and_then(JsValue::as_str)
                .unwrap_or_default(),
        ) == command
    });
    let Some(prompt) = found else {
        return refused(format!("No MCP prompt matches {word}"));
    };
    let declared: &[JsValue] = prompt
        .get("arguments")
        .and_then(JsValue::as_array)
        .map_or(&[], Vec::as_slice);
    if tokens.len() > declared.len() {
        return refused(format!(
            "{word} takes {} argument(s) and was given {}",
            declared.len(),
            tokens.len()
        ));
    }
    let missing: Vec<&str> = declared
        .iter()
        .skip(tokens.len())
        .filter(|each| each.get("required").and_then(JsValue::as_bool) == Some(true))
        .map(|each| {
            each.get("name")
                .and_then(JsValue::as_str)
                .unwrap_or_default()
        })
        .collect();
    if !missing.is_empty() {
        return refused(format!("{word} needs {}", missing.join(", ")));
    }
    let given: IndexMap<String, String> = tokens
        .iter()
        .enumerate()
        .map(|(at, token)| {
            let name = declared
                .get(at)
                .and_then(|each| each.get("name"))
                .and_then(JsValue::as_str)
                .unwrap_or_default();
            (name.to_owned(), (*token).to_owned())
        })
        .collect();
    McpPromptRead::Invoke {
        invocation: McpPromptInvocation {
            server: server_of(prompt).unwrap_or_default().to_owned(),
            prompt: prompt
                .get("name")
                .and_then(JsValue::as_str)
                .unwrap_or_default()
                .to_owned(),
            arguments: in_key_order(given),
        },
    }
}

crate::registry::exports! {
    "format/mcp-prompts" {
        "listablePrompts" => ok |prompts: Vec<McpPromptDefinition>| listable_prompts(&prompts);
        "mcpPromptListing" => ok |server: String, prompt: String| mcp_prompt_listing(&server, &prompt);
        "mcpPromptCommand" => ok |server: String, prompt: String| mcp_prompt_command(&server, &prompt);
        "readMcpPromptInvocation" => ok |input: String, prompts: Vec<McpPromptDefinition>| read_mcp_prompt_invocation(&input, &prompts);
    }
}
