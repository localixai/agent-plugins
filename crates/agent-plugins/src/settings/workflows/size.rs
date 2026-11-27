//! Port of `src/settings/workflows/size.ts` at sha256:604f8bb6196f.
//!
//! It takes the host's `PolicyView`, so it stays
//! TypeScript in the package: nothing of it is exported.

use crate::settings::layers::PolicyScope;
use crate::settings::view::PolicyView;

/// A size of the `workflowSizeGuideline` setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowSize {
    /// No guideline.
    Unrestricted,
    /// Fewer than 5 agents.
    Small,
    /// Fewer than 10 agents.
    Medium,
    /// Fewer than 50 agents.
    Large,
}

impl WorkflowSize {
    /// The size a setting names.
    fn named(text: &str) -> Option<Self> {
        match text {
            "unrestricted" => Some(Self::Unrestricted),
            "small" => Some(Self::Small),
            "medium" => Some(Self::Medium),
            "large" => Some(Self::Large),
            _ => None,
        }
    }
}

/// What the host tells the size guideline besides the layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowSizeContext {
    /// Pro signed in: the default is `small` there (settings-reference.md, `workflowSizeGuideline`).
    pub pro: bool,
    /// The Dynamic workflow size choice of `/config`, which Claude Code keeps in `~/.claude.json`: a file
    /// the kit never reads, so the host hands the value over.
    pub config_choice: Option<WorkflowSize>,
    /// The session has ultracode on (`/effort ultracode`, or the `ultracode` setting): a session state the
    /// host holds. The `Large workflow` warning does not show then (workflows.md, "Cost").
    pub ultracode: Option<bool>,
}

/// Where the size in force comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowSizeSource {
    /// A settings file.
    Settings,
    /// The `/config` choice.
    Config,
    /// The default.
    Default,
}

/// The size guideline in force.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowSizeDecision {
    /// The size.
    pub size: WorkflowSize,
    /// Where it comes from.
    pub source: WorkflowSizeSource,
    /// The file that set it, when `source` is `settings`.
    pub settings_scope: Option<PolicyScope>,
    /// Claude aims for fewer agents than this; none for `unrestricted`.
    pub agents_below: Option<u32>,
    /// Advice sent to Claude, "not an enforced cap".
    pub enforced: bool,
    /// Whether `/config` shows its Dynamic workflow size row: hidden while a settings file sets the key.
    pub config_row_visible: bool,
    /// The agent count above which the progress line shows `Large workflow` (workflows.md, "Cost");
    /// `None` while ultracode is on, which shows no warning.
    pub large_workflow_threshold: Option<u32>,
    /// The projected token total above which the progress line shows `Large workflow` (1.5 million);
    /// `None` while ultracode is on. The host compares it with its run's projection: the kit holds
    /// no run data (spec/judgments.md).
    pub large_workflow_tokens: Option<u32>,
}

/// "fewer than 5", "fewer than 10", "fewer than 50" (workflows.md, "Set a size guideline").
const fn agents_below(size: WorkflowSize) -> Option<u32> {
    match size {
        WorkflowSize::Unrestricted => None,
        WorkflowSize::Small => Some(5),
        WorkflowSize::Medium => Some(10),
        WorkflowSize::Large => Some(50),
    }
}

/// The built-in threshold of the `Large workflow` warning (workflows.md, "Cost").
const DEFAULT_THRESHOLD: u32 = 25;

/// The projected token total that also shows it: "its projected token total passes 1.5 million".
const TOKEN_THRESHOLD: u32 = 1_500_000;

const fn default_size(pro: bool) -> WorkflowSize {
    if pro {
        WorkflowSize::Small
    } else {
        WorkflowSize::Medium
    }
}

/// A chosen size replaces the 25-agent threshold with its own count; `unrestricted` has none.
fn agent_threshold(chosen: Option<WorkflowSize>, below: Option<u32>) -> u32 {
    match chosen {
        None => DEFAULT_THRESHOLD,
        Some(_) => below.unwrap_or(DEFAULT_THRESHOLD),
    }
}

/// The size guideline in force: a value in a settings file, else the `/config` choice, else the default
/// (`medium`, `small` on Pro). A value of another shape was dropped when the file was read, so it never
/// reaches here. A size the user chose replaces the 25-agent threshold with its own agent count;
/// `unrestricted` has none, so it leaves 25 (spec/judgments.md). With ultracode on there is no warning,
/// so neither threshold is returned.
#[must_use]
pub fn workflow_size(view: &dyn PolicyView, context: &WorkflowSizeContext) -> WorkflowSizeDecision {
    let holder = view.winner("workflowSizeGuideline");
    let chosen = holder
        .as_ref()
        .and_then(|holder| holder.value.as_str().and_then(WorkflowSize::named))
        .or(context.config_choice);
    let size = chosen.unwrap_or_else(|| default_size(context.pro));
    let source = if holder.is_some() {
        WorkflowSizeSource::Settings
    } else if chosen.is_some() {
        WorkflowSizeSource::Config
    } else {
        WorkflowSizeSource::Default
    };
    let below = agents_below(size);
    let warns = context.ultracode != Some(true);
    WorkflowSizeDecision {
        size,
        source,
        settings_scope: holder.as_ref().map(|holder| holder.scope),
        agents_below: below,
        enforced: false,
        config_row_visible: holder.is_none(),
        large_workflow_threshold: warns.then(|| agent_threshold(chosen, below)),
        large_workflow_tokens: warns.then_some(TOKEN_THRESHOLD),
    }
}
