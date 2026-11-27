//! Port of `src/skills/invocation.ts`, switched.
//!
//! Who may invoke a skill and what the model and the `/` menu are told about it (skills.md,
//! "Control who invokes a skill" and "Override skill visibility from settings"). Pure: the host
//! reads the settings and passes `skillOverrides` in.

use serde::{Deserialize, Serialize};

use super::visibility::{ModelListing, Origin, Overridable, SkillOverrides, exposure_of};

/// A skill or a command file: both carry the two frontmatter fields (skills.md, "Choose where skills
/// load"). The body is not needed to decide who may invoke it or how it is listed, so a skill read
/// without its body (`InvocableHead`) is as good as a whole one. These are the members the policy
/// reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invocable {
    /// The name the skill goes by.
    pub name: String,
    /// What the user types after `/`.
    pub command: String,
    /// Where it came from.
    pub origin: Origin,
    /// `disable-model-invocation`, default `false`.
    pub disable_model_invocation: bool,
    /// `user-invocable`, default `true`.
    pub user_invocable: bool,
}

impl Invocable {
    fn overridable(&self) -> Overridable {
        Overridable {
            name: self.name.clone(),
            origin: self.origin.clone(),
        }
    }
}

/// Who starts a skill: the user typing `/name`, Claude through the Skill tool, a subagent's `skills`
/// list (preloading), a session-scoped scheduled task that fires with the skill as its prompt.
pub const INVOKERS: [Invoker; 4] = [
    Invoker::User,
    Invoker::Model,
    Invoker::Preload,
    Invoker::ScheduledTask,
];

/// One of `INVOKERS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Invoker {
    /// The user typing `/name`.
    #[serde(rename = "user")]
    User,
    /// Claude through the Skill tool.
    #[serde(rename = "model")]
    Model,
    /// A subagent's `skills` list.
    #[serde(rename = "preload")]
    Preload,
    /// A scheduled task's prompt.
    #[serde(rename = "scheduled-task")]
    ScheduledTask,
}

/// The whole matrix for one skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's JSON shape: one flag per way a skill is invoked"
)]
pub struct InvocationPolicy {
    /// Typing `/name` runs the skill.
    pub user: bool,
    /// Claude may load the skill on its own through the Skill tool.
    pub model: bool,
    /// What the skill listing the model reads shows of it.
    pub listed_to_model: ModelListing,
    /// The skill is a row of the `/` menu.
    pub in_menu: bool,
    /// A subagent's `skills` field may preload it.
    pub preloadable: bool,
    /// A scheduled task may fire with it as the prompt.
    pub scheduled_prompt: bool,
}

/// The whole matrix for one skill: the frontmatter and the override together. "Description not in
/// context" for `disable-model-invocation: true`; a skill the model cannot invoke is also not
/// preloadable and not run by a scheduled fire, because those "draw from the same set of skills
/// Claude can invoke" (sub-agents.md).
#[must_use]
pub fn invocation_policy(skill: &Invocable, overrides: &SkillOverrides) -> InvocationPolicy {
    let exposure = exposure_of(&skill.overridable(), overrides);
    let listed_to_model = if skill.disable_model_invocation {
        ModelListing::Hidden
    } else {
        exposure.model
    };
    let model = listed_to_model != ModelListing::Hidden;
    let user = skill.user_invocable && exposure.user;
    InvocationPolicy {
        user,
        model,
        listed_to_model,
        in_menu: user,
        preloadable: model,
        scheduled_prompt: model,
    }
}

const fn policy_allows(by: Invoker, policy: InvocationPolicy) -> bool {
    match by {
        Invoker::User => policy.user,
        Invoker::Model => policy.model,
        Invoker::Preload => policy.preloadable,
        Invoker::ScheduledTask => policy.scheduled_prompt,
    }
}

/// Why an invoker was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DenialReason {
    /// The skill sets `disable-model-invocation`.
    #[serde(rename = "disable-model-invocation")]
    DisableModelInvocation,
    /// The skill sets `user-invocable: false`.
    #[serde(rename = "user-invocable-false")]
    UserInvocableFalse,
    /// The `skillOverrides` setting.
    #[serde(rename = "skill-override")]
    SkillOverride,
}

/// Why an invoker was refused: the field that is set, else the override.
const fn denial_reason(by: Invoker, skill: &Invocable) -> DenialReason {
    match by {
        Invoker::User => {
            if skill.user_invocable {
                DenialReason::SkillOverride
            } else {
                DenialReason::UserInvocableFalse
            }
        }
        Invoker::Model | Invoker::Preload | Invoker::ScheduledTask => {
            if skill.disable_model_invocation {
                DenialReason::DisableModelInvocation
            } else {
                DenialReason::SkillOverride
            }
        }
    }
}

/// Why the skill is withheld, as a clause: used in the texts of the invokers that are not the model.
const fn denial_clause(reason: DenialReason) -> &'static str {
    match reason {
        DenialReason::DisableModelInvocation => {
            "it sets disable-model-invocation, so Claude cannot invoke it"
        }
        DenialReason::UserInvocableFalse => "it sets user-invocable: false",
        DenialReason::SkillOverride => "the skillOverrides setting hides it from Claude",
    }
}

/// What the refused party is told. Judgment (spec/judgments.md): the docs say Claude "is told ... not
/// to reproduce the deploy steps another way", so it suggests `/deploy`, and that the `skillOverrides`
/// error is returned, but they quote neither text. A scheduled fire withheld from a skill "reaches
/// Claude as plain text instead of executing" (scheduled-tasks.md), so its text says that; a preload
/// is simply not injected.
fn denial_message(by: Invoker, command: &str, reason: DenialReason) -> String {
    match by {
        Invoker::User => {
            if reason == DenialReason::UserInvocableFalse {
                format!(
                    "/{command} sets user-invocable: false: only Claude can invoke it, so typing /{command} does not run it."
                )
            } else {
                format!(
                    "/{command} is hidden by the skillOverrides setting, so it is not run. Ask the user to change skillOverrides if it should run."
                )
            }
        }
        Invoker::Model => {
            if reason == DenialReason::SkillOverride {
                format!(
                    "/{command} is hidden by the skillOverrides setting, so it is not run. Ask the user to change skillOverrides if it should run."
                )
            } else {
                format!(
                    "/{command} sets disable-model-invocation: Claude cannot run it on its own. Ask the user to run /{command} themselves. Do not carry out its steps another way."
                )
            }
        }
        Invoker::Preload => format!(
            "/{command} is not preloaded into the subagent: {}.",
            denial_clause(reason)
        ),
        Invoker::ScheduledTask => format!(
            "/{command} is not run by the scheduled task: {}. The prompt reaches Claude as plain text instead of executing.",
            denial_clause(reason)
        ),
    }
}

/// Whether an invoker may start a skill; when not, the reason and the text to give back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InvocationVerdict {
    /// Whether it may.
    pub allowed: bool,
    /// Why not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<DenialReason>,
    /// The text to give back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

const ALLOWED: InvocationVerdict = InvocationVerdict {
    allowed: true,
    reason: None,
    message: None,
};

/// What `checkInvocation` is told besides the skill.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationOptions {
    /// The user's message names the skill after plain text (`go ahead and /deploy`, see
    /// `readSkillMentions`): "The name counts as your permission for that message". Judgment
    /// (spec/judgments.md): it lifts `disable-model-invocation` for the model's invocation in that
    /// message only, never a `skillOverrides` state, and never a preload or a scheduled fire.
    #[serde(default)]
    pub permitted_by_mention: Option<bool>,
}

/// Whether `by` may start `skill`, with the reason and the text to give back when not.
#[must_use]
pub fn check_invocation(
    skill: &Invocable,
    by: Invoker,
    overrides: &SkillOverrides,
    options: InvocationOptions,
) -> InvocationVerdict {
    if policy_allows(by, invocation_policy(skill, overrides)) {
        return ALLOWED;
    }
    if by == Invoker::Model
        && options.permitted_by_mention == Some(true)
        && skill.disable_model_invocation
    {
        let without_flag = Invocable {
            disable_model_invocation: false,
            ..skill.clone()
        };
        if invocation_policy(&without_flag, overrides).model {
            return ALLOWED;
        }
    }
    let reason = denial_reason(by, skill);
    InvocationVerdict {
        allowed: false,
        reason: Some(reason),
        message: Some(denial_message(by, &skill.command, reason)),
    }
}

crate::registry::exports! {
    "skills/invocation" {
        "INVOKERS" => ok || INVOKERS;
        "invocationPolicy" => ok |skill: Invocable, overrides: Option<SkillOverrides>|
            invocation_policy(&skill, &overrides.unwrap_or_default());
        "checkInvocation" => ok |skill: Invocable, by: Invoker, overrides: Option<SkillOverrides>, options: Option<InvocationOptions>|
            check_invocation(&skill, by, &overrides.unwrap_or_default(), options.unwrap_or_default());
    }
}
