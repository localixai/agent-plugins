//! Port of `src/agents/memory.ts`, switched.
//!
//! The persistent memory of a subagent (sub-agents.md, "Enable persistent memory"). The kit plans
//! the directory, the part of `MEMORY.md` to include and the tools to add; reading and writing the
//! files in the system prompt are the host's.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::config_directory::{JoinPath, config_directory_of};
use crate::format::utf8::prefix_within_bytes;
use crate::js::Pattern;
use crate::ports::Signal;
use crate::ports::files::TextFileReader;
use crate::ports::locations::Locations;
use crate::registry::Port;

/// memory.md, "Storage location" of MEMORY.md: "The first 200 lines ... or the first 25KB".
const MEMORY_MAX_LINES: usize = 200;
/// "25KB" read as 25 * 1024 bytes of UTF-8 (judgment, spec/judgments.md).
const MEMORY_MAX_BYTES: usize = 25 * 1024;

/// `/^---[ \t]*\r?\n[\s\S]*?\r?\n---[ \t]*(?:\r?\n|$)/`.
static FRONTMATTER: Pattern =
    LazyLock::new(|| Regex::new(r"^---[ \t]*\r?\n(?s:.)*?\r?\n---[ \t]*(?:\r?\n|$)").ok());
/// `/^ {0,3}<!--/`.
static COMMENT_OPEN: Pattern = LazyLock::new(|| Regex::new(r"^ {0,3}<!--").ok());
/// `` /^ {0,3}(`{3,}|~{3,})/ ``.
static FENCE: Pattern = LazyLock::new(|| Regex::new(r"^ {0,3}(`{3,}|~{3,})").ok());
/// `/^.*?<!--/`: `.` is no line terminator.
static BEFORE_COMMENT: Pattern =
    LazyLock::new(|| Regex::new(r"^[^\n\r\u{2028}\u{2029}]*?<!--").ok());

/// `text.replace(pattern, '')`: the first match removed.
fn without_first(pattern: &Pattern, text: &str) -> String {
    match pattern.as_ref().and_then(|regex| regex.find(text)) {
        Some(found) => {
            let before = text.get(..found.start()).unwrap_or_default();
            format!("{before}{}", text.get(found.end()..).unwrap_or_default())
        }
        None => text.to_owned(),
    }
}

/// The line after the first `-->` of the comment opened on `start`, or `None`.
fn comment_end(lines: &[&str], start: usize) -> Option<usize> {
    let first = without_first(
        &BEFORE_COMMENT,
        lines.get(start).copied().unwrap_or_default(),
    );
    let rest = std::iter::once(first.as_str()).chain(lines.iter().skip(start + 1).copied());
    let found = rest.enumerate().find(|(_, line)| line.contains("-->"))?.0;
    Some(start + found + 1)
}

/// The open fence after a line: a run of ``` or ~~~ opens it, a run as long of the same mark closes it.
fn next_fence<'a>(fence: Option<&'a str>, run: Option<&'a str>) -> Option<&'a str> {
    let (Some(fence), Some(run)) = (fence, run) else {
        return fence.or(run);
    };
    let same_mark = run.chars().next() == fence.chars().next();
    if same_mark && run.len() >= fence.len() {
        None
    } else {
        Some(fence)
    }
}

/// Drops block-level HTML comments, outside fenced code (judgment, spec/judgments.md).
fn without_block_comments(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut fence: Option<&str> = None;
    let mut i = 0;
    while let Some(line) = lines.get(i).copied() {
        let opened = FENCE
            .as_ref()
            .and_then(|regex| regex.captures(line))
            .and_then(|found| found.get(1))
            .map(|run| run.as_str());
        let opens_comment = COMMENT_OPEN
            .as_ref()
            .is_some_and(|regex| regex.is_match(line));
        let end = if fence.is_none() && opens_comment {
            comment_end(&lines, i)
        } else {
            None
        };
        if let Some(end) = end {
            i = end;
            continue;
        }
        fence = next_fence(fence, opened);
        kept.push(line);
        i += 1;
    }
    kept.join("\n")
}

/// errors.md, "Memory index is over its read limit": frontmatter and block comments do not load.
fn loaded_content(text: &str) -> String {
    without_block_comments(&without_first(&FRONTMATTER, text))
}

/// The part of a `MEMORY.md` index the system prompt includes, and whether content was cut.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryIndex {
    /// What is kept.
    pub text: String,
    /// Content was cut.
    pub truncated: bool,
}

/// The part of a `MEMORY.md` index that the subagent's system prompt includes: "the first 200 lines
/// or 25KB ..., whichever comes first". Frontmatter and block-level HTML comments are not loaded, so
/// they do not count (errors.md). `truncated` says content was cut, so the host adds the instruction
/// to curate the index. The cut at the byte limit is made between characters, not between lines
/// (judgment).
#[must_use]
pub fn truncate_memory_index(text: &str) -> MemoryIndex {
    let content = loaded_content(text);
    let lines: Vec<&str> = content.split('\n').collect();
    let line_count = lines.len() - usize::from(lines.last() == Some(&""));
    let by_lines = if line_count > MEMORY_MAX_LINES {
        lines
            .iter()
            .take(MEMORY_MAX_LINES)
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        content.clone()
    };
    let kept = prefix_within_bytes(&by_lines, MEMORY_MAX_BYTES);
    MemoryIndex {
        truncated: kept != content,
        text: kept.to_owned(),
    }
}

/// The `memory` scope of an agent (`AgentMemoryScope` of `src/format/agent-fields.ts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMemoryScope {
    /// `~/.claude/agent-memory/<name>/`.
    User,
    /// `.claude/agent-memory/<name>/`.
    Project,
    /// `.claude/agent-memory-local/<name>/`.
    Local,
}

/// The memory of a subagent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPlan {
    /// The memory directory: the subagent's own, created by the host when absent.
    pub directory: String,
    /// What of `MEMORY.md` the system prompt includes (empty when the file does not exist).
    pub index: String,
    /// The index was cut at 200 lines or 25KB.
    pub truncated: bool,
    /// "Read, Write, and Edit tools are automatically enabled so the subagent can manage its memory".
    pub add_tools: [&'static str; 3],
}

/// The project's `.claude` folder.
async fn project_claude<L: Locations, F>(ctx: &MemoryContext<L, F>) -> String {
    let project = ctx.locations.project_dir().await;
    ctx.locations.join_path(&project, ".claude").await
}

/// The `.claude` folder of each scope: the configuration directory's, or the project's.
async fn scope_claude<L: Locations, F>(
    scope: AgentMemoryScope,
    ctx: &MemoryContext<L, F>,
) -> String {
    match scope {
        AgentMemoryScope::User => {
            let home = ctx.locations.home_dir().await;
            config_directory_of(&home, ctx.config_dir.as_deref(), &JoinPath(&ctx.locations)).await
        }
        AgentMemoryScope::Project | AgentMemoryScope::Local => project_claude(ctx).await,
    }
}

/// The folder of each scope under its `.claude` folder.
const fn scope_folder(scope: AgentMemoryScope) -> &'static str {
    match scope {
        AgentMemoryScope::User | AgentMemoryScope::Project => "agent-memory",
        AgentMemoryScope::Local => "agent-memory-local",
    }
}

/// A plugin agent's type is `<plugin>:<name>`, and a `:` is no safe name for a folder everywhere, so
/// `:` (and a path separator, which must never leave the memory folder) becomes `-`; a name of only
/// dots becomes `-` too (judgment: the docs name the folder `<name-of-agent>` and say no more).
fn folder_name(agent_type: &str) -> String {
    let replaced = agent_type.replace([':', '/', '\\'], "-");
    if !replaced.is_empty() && replaced.chars().all(|c| c == '.') {
        "-".to_owned()
    } else {
        replaced
    }
}

/// What a plan reads from the host.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryContext<L, F> {
    /// Auto memory is on: `autoMemoryEnabled` is not false and `CLAUDE_CODE_DISABLE_AUTO_MEMORY` is unset.
    pub auto_memory_enabled: bool,
    /// Its `homeDir`, `projectDir` and `joinPath`.
    pub locations: L,
    /// `CLAUDE_CONFIG_DIR` as the session's environment gives it: the user scope's memory lives below it when set.
    #[serde(default)]
    pub config_dir: Option<String>,
    /// Where `MEMORY.md` is read.
    pub files: F,
    /// Aborts the read.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The persistent memory of a subagent from its `memory` scope: the directory (`~/.claude/agent-memory/<name>/`,
/// `.claude/agent-memory/<name>/` or `.claude/agent-memory-local/<name>/`), the part of its `MEMORY.md`
/// to include and the tools to add. With no scope, or with auto memory off ("the `memory` field has
/// no effect and the subagent launches without the memory instructions or the memory tool access"),
/// there is no plan. The instruction text for the system prompt is not quoted by the docs and is the
/// host's.
pub async fn plan_agent_memory<L: Locations, F: TextFileReader>(
    scope: Option<AgentMemoryScope>,
    agent_type: &str,
    ctx: &MemoryContext<L, F>,
) -> Option<MemoryPlan> {
    let scope = scope.filter(|_| ctx.auto_memory_enabled)?;
    let claude = scope_claude(scope, ctx).await;
    let folder = format!("{}/{}", scope_folder(scope), folder_name(agent_type));
    let directory = ctx.locations.join_path(&claude, &folder).await;
    let index = ctx.locations.join_path(&directory, "MEMORY.md").await;
    let raw = ctx.files.read_text(&index, ctx.signal.as_ref()).await;
    let MemoryIndex { text, truncated } = truncate_memory_index(&raw.unwrap_or_default());
    Some(MemoryPlan {
        directory,
        index: text,
        truncated,
        add_tools: ["Read", "Write", "Edit"],
    })
}

crate::registry::exports! {
    "agents/memory" {
        "truncateMemoryIndex" => ok |text: String| truncate_memory_index(&text);
        "planAgentMemory" => async ok |scope: Option<AgentMemoryScope>, agent_type: String, ctx: MemoryContext<Port, Port>|
            plan_agent_memory(scope, &agent_type, &ctx).await;
    }
}
