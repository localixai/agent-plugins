//! Port of `src/format/mcp-add.ts`, switched.
//!
//! `claude mcp add [options] <name> -- <command> [args...]` and `claude mcp add --transport <type>
//! <name> <url>` (mcp.md, "Option 1" to "Option 3"): the arguments become the entry `add-json` would
//! have received. Everything after `--` is the server's own command line, "passed to the server
//! untouched". The CLI itself, its help and its exit codes are the host's.

use indexmap::IndexMap;

use super::issues::{PluginIssue, error, warning};
use super::mcp_add_json::{
    McpAddOptions, McpAddRead, McpAddScope, McpAddTarget, ScopeRead, complete_mcp_add,
    read_add_scope,
};
use super::mcp_env::EnvLookup;
use crate::js::{JsObject, JsValue, in_property_order, trim};
use crate::registry::{Callback, Set};

const TRANSPORTS: [&str; 3] = ["stdio", "sse", "http"];

/// Flag spellings of the options that take one value, as `(long, short)`.
const VALUE_FLAGS: [(&str, Option<&str>); 4] = [
    ("--transport", Some("-t")),
    ("--scope", Some("-s")),
    ("--client-id", None),
    ("--callback-port", None),
];
const HEADER_FLAGS: [&str; 2] = ["--header", "-H"];
const ENV_FLAGS: [&str; 2] = ["--env", "-e"];
const SWITCHES: [&str; 1] = ["--client-secret"];
const WHERE: &str = "claude mcp add";

/// The argument list as read.
#[derive(Default)]
struct Parsed {
    values: IndexMap<&'static str, String>,
    headers: Vec<String>,
    env: JsObject,
    flags: Vec<String>,
    positionals: Vec<String>,
    after: Option<Vec<String>>,
    problems: Vec<PluginIssue>,
}

fn long_name(token: &str) -> Option<&'static str> {
    VALUE_FLAGS
        .iter()
        .find(|(long, short)| token == *long || Some(token) == *short)
        .map(|(long, _)| *long)
}

fn problem(message: &str) -> PluginIssue {
    error(WHERE, message)
}

/// Takes a flag that has a value; returns the index of the last token it used.
fn take_value(argv: &[String], at: usize, out: &mut Parsed) -> usize {
    let token = argv.get(at).map_or("", String::as_str);
    match (argv.get(at + 1), long_name(token)) {
        (None, _) => out
            .problems
            .push(problem(&format!("{token} needs a value"))),
        (Some(next), None) => out.headers.push(next.clone()),
        (Some(next), Some(long)) => {
            out.values.insert(long, next.clone());
        }
    }
    at + 1
}

/// `KEY=value`: the first `=` splits, so a value keeps its own.
fn pair(text: &str) -> Option<(&str, &str)> {
    text.split_once('=').filter(|(key, _)| !key.is_empty())
}

fn add_pair(out: &mut Parsed, text: &str) {
    if let Some((key, value)) = pair(text) {
        out.env.insert(key.to_owned(), value.into());
        return;
    }
    let hint = format!(
        "if \"{text}\" is the server name, put another option such as --transport stdio between --env and the name"
    );
    let message = format!("--env takes KEY=value pairs and \"{text}\" is not one: {hint}");
    out.problems.push(problem(&message));
}

/// Takes `--env` and the `KEY=value` pairs that follow it.
fn take_env(argv: &[String], at: usize, out: &mut Parsed) -> usize {
    let mut last = at;
    while let Some(next) = argv.get(last + 1).filter(|next| !next.starts_with('-')) {
        add_pair(out, next);
        last += 1;
    }
    last
}

/// Takes a switch, an unknown option or a positional.
fn take_other(argv: &[String], at: usize, out: &mut Parsed) -> usize {
    let token = argv.get(at).map_or("", String::as_str);
    if SWITCHES.contains(&token) {
        if !out.flags.iter().any(|flag| flag == token) {
            out.flags.push(token.to_owned());
        }
    } else if token.starts_with('-') {
        let message = format!("Unknown option {token}: put the server's own flags after --");
        out.problems.push(problem(&message));
    } else {
        out.positionals.push(token.to_owned());
    }
    at
}

fn take(argv: &[String], at: usize, out: &mut Parsed) -> usize {
    let token = argv.get(at).map_or("", String::as_str);
    if long_name(token).is_some() || HEADER_FLAGS.contains(&token) {
        return take_value(argv, at, out);
    }
    if ENV_FLAGS.contains(&token) {
        take_env(argv, at, out)
    } else {
        take_other(argv, at, out)
    }
}

/// Reads the argument list: flags anywhere before `--`, positionals in order, the rest after `--`.
fn parse(argv: &[String]) -> Parsed {
    let mut out = Parsed::default();
    let mut at = 0;
    while let Some(token) = argv.get(at) {
        if token == "--" {
            out.after = Some(argv.get(at + 1..).unwrap_or_default().to_vec());
            break;
        }
        at = take(argv, at, &mut out) + 1;
    }
    out.env = in_property_order(std::mem::take(&mut out.env));
    out
}

fn headers(texts: &[String], problems: &mut Vec<PluginIssue>) -> JsObject {
    let mut out = JsObject::new();
    for text in texts {
        match text.split_once(':').filter(|(name, _)| !name.is_empty()) {
            Some((name, value)) => {
                out.insert(trim(name).to_owned(), trim(value).into());
            }
            None => {
                let message = format!("--header takes \"Name: value\" and \"{text}\" is not one");
                problems.push(problem(&message));
            }
        }
    }
    in_property_order(out)
}

fn strings(items: &[String]) -> JsValue {
    JsValue::Array(items.iter().map(|item| item.as_str().into()).collect())
}

/// The stdio entry the arguments describe, or why they describe none.
fn stdio(parsed: &Parsed, rest: &[String]) -> Result<JsObject, String> {
    let line = parsed.after.as_deref().unwrap_or(rest);
    let Some((command, args)) = line.split_first() else {
        return Err(
            "A stdio server needs a command: claude mcp add <name> -- <command> [args...]"
                .to_owned(),
        );
    };
    let mut entry = JsObject::from([
        ("type".to_owned(), "stdio".into()),
        ("command".to_owned(), command.as_str().into()),
    ]);
    if !args.is_empty() {
        entry.insert("args".to_owned(), strings(args));
    }
    if !parsed.env.is_empty() {
        entry.insert("env".to_owned(), JsValue::Object(parsed.env.clone()));
    }
    Ok(entry)
}

/// An HTTP or SSE entry the arguments describe, or why they describe none.
fn remote(kind: &str, parsed: &mut Parsed, rest: &[String]) -> Result<JsObject, String> {
    if parsed.after.is_some() {
        return Err(format!(
            "Arguments after -- are a stdio server's command line, not a {kind} server's"
        ));
    }
    let Some((url, extra)) = rest.split_first() else {
        return Err(format!(
            "A {kind} server needs a URL: claude mcp add --transport {kind} <name> <url>"
        ));
    };
    if !extra.is_empty() {
        return Err(format!(
            "Unexpected arguments after the URL: {}",
            extra.join(" ")
        ));
    }
    let sent = headers(&parsed.headers, &mut parsed.problems);
    let mut entry = JsObject::from([
        ("type".to_owned(), kind.into()),
        ("url".to_owned(), url.as_str().into()),
    ]);
    if !sent.is_empty() {
        entry.insert("headers".to_owned(), JsValue::Object(sent));
    }
    Ok(entry)
}

/// `--client-id` and `--callback-port` as the `oauth` object of the entry.
fn oauth_flags(parsed: &Parsed) -> Option<JsObject> {
    let client_id = parsed.values.get("--client-id");
    let port = parsed.values.get("--callback-port");
    if client_id.is_none() && port.is_none() {
        return None;
    }
    let mut oauth = JsObject::new();
    if let Some(client_id) = client_id {
        oauth.insert("clientId".to_owned(), client_id.as_str().into());
    }
    if let Some(port) = port {
        let digits = !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit());
        let value = match port.parse::<f64>() {
            Ok(number) if digits => JsValue::Number(number),
            Ok(_) | Err(_) => port.as_str().into(),
        };
        oauth.insert("callbackPort".to_owned(), value);
    }
    Some(oauth)
}

/// The transport the flags name, or the issue that refuses it.
fn transport_of(parsed: &Parsed) -> Result<&'static str, PluginIssue> {
    let named = parsed
        .values
        .get("--transport")
        .map_or("stdio", String::as_str);
    if named == "ws" {
        return Err(problem(
            "--transport does not accept \"ws\": add a WebSocket server with claude mcp add-json",
        ));
    }
    TRANSPORTS
        .iter()
        .find(|transport| **transport == named)
        .copied()
        .ok_or_else(|| {
            let message = format!(
                "--transport must be one of {}, not \"{named}\"",
                TRANSPORTS.join(", ")
            );
            problem(&message)
        })
}

/// The entry the arguments describe and its transport, or the refusal's issues.
fn entry_of(
    parsed: &mut Parsed,
    name: &str,
    rest: &[String],
) -> Result<(JsObject, &'static str), Vec<PluginIssue>> {
    if name.is_empty() {
        return Err(vec![problem(
            "A server name is needed: claude mcp add [options] <name> ...",
        )]);
    }
    let transport = transport_of(parsed).map_err(|issue| vec![issue])?;
    let built = if parsed.after.is_some() && !rest.is_empty() {
        Err(format!(
            "Unexpected arguments before --: {}",
            rest.join(" ")
        ))
    } else if transport == "stdio" {
        stdio(parsed, rest)
    } else {
        remote(transport, parsed, rest)
    };
    let mut entry = match built {
        Err(why) => {
            let mut issues = vec![problem(&why)];
            issues.append(&mut parsed.problems);
            return Err(issues);
        }
        Ok(entry) => entry,
    };
    if !parsed.problems.is_empty() {
        return Err(std::mem::take(&mut parsed.problems));
    }
    if let Some(oauth) = oauth_flags(parsed) {
        entry.insert("oauth".to_owned(), JsValue::Object(oauth));
    }
    Ok((entry, transport))
}

/// An add that is refused: the issues say why.
fn refusal(name: &str, scope: Option<McpAddScope>, issues: Vec<PluginIssue>) -> McpAddRead {
    McpAddRead {
        name: name.to_owned(),
        scope,
        server: None,
        oauth: None,
        secret: None,
        issues,
    }
}

/// Reads the arguments after `claude mcp add`. Judgments (spec/judgments.md): no `--transport` means
/// stdio; without `--` the command is the positional after the name; `--header` on a stdio server and
/// the OAuth flags on a stdio one have no effect and say so; `ws` is refused with a pointer to
/// `add-json`, since "the `claude mcp add --transport` flag doesn't accept `ws`"; the wording of every
/// refusal is the kit's.
pub async fn read_mcp_add(
    argv: &[String],
    lookup: &dyn EnvLookup,
    existing: Set<String>,
) -> McpAddRead {
    let mut parsed = parse(argv);
    let positionals = std::mem::take(&mut parsed.positionals);
    let (name, rest) = positionals
        .split_first()
        .map_or(("", &[][..]), |(name, rest)| (name.as_str(), rest));
    let scope = read_add_scope(name, parsed.values.get("--scope").map(String::as_str));
    let known = match &scope {
        ScopeRead::Scope(scope) => Some(*scope),
        ScopeRead::Issue(_) => None,
    };
    if !parsed.problems.is_empty() {
        return refusal(name, known, std::mem::take(&mut parsed.problems));
    }
    let scope = match scope {
        ScopeRead::Scope(scope) => scope,
        ScopeRead::Issue(issue) => return refusal(name, known, vec![issue]),
    };
    let (entry, transport) = match entry_of(&mut parsed, name, rest) {
        Ok(made) => made,
        Err(issues) => return refusal(name, known, issues),
    };
    let options = McpAddOptions {
        scope: None,
        client_secret: Some(parsed.flags.iter().any(|flag| flag == "--client-secret")),
        existing: Some(existing),
    };
    let target = McpAddTarget {
        name: name.to_owned(),
        scope,
    };
    let mut done = complete_mcp_add(&target, &entry, &options, lookup).await;
    if transport == "stdio" && !parsed.headers.is_empty() {
        let useless = warning(WHERE, "--header has no effect on a stdio server");
        done.issues.push(useless);
    }
    done
}

crate::registry::exports! {
    "format/mcp-add" {
        "readMcpAdd" => async ok |argv: Vec<String>, lookup: Callback, existing: Option<Set<String>>|
            read_mcp_add(&argv, &lookup, existing.unwrap_or(Set { members: Vec::new() })).await;
    }
}
