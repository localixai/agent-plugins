//! Port of `src/settings/skills/policy.ts`, switched.
//!
//! The skill keys of the settings as one object over a view (docs/API.md, "Settings"). The host
//! holds each question as a function of the core's.

use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::bundled::{BundledSkillsDecision, bundled_skills};
use super::overrides::{SkillSubject, skill_visibility};
use super::shell::{ShellOrigin, shell_verdict};
use super::sync_skills::sync_skills_decision;
use crate::registry::{Function, Port};
use crate::settings::managed::parent::{ParentSettingsContext, effective_view};
use crate::settings::view::PolicyView;

/// What the host tells the skill policy besides the layers.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPolicyContext {
    /// The parent-settings facts.
    #[serde(flatten)]
    pub parent: ParentSettingsContext,
    /// The host's environment as data (`CLAUDE_CODE_DISABLE_BUNDLED_SKILLS`); the kit reads no
    /// `process.env`.
    pub env: IndexMap<String, Option<String>>,
}

/// The skill policy as the host holds it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSettingsPolicy {
    /// What `skillOverrides` does to one skill.
    pub visibility: Function,
    /// `disableBundledSkills` and its variable.
    pub bundled: Function,
    /// The bundled skills are off.
    pub bundled_skills_disabled: bool,
    /// Whether a skill's inline shell runs.
    pub shell: Function,
    /// The claude.ai skill sync.
    pub sync_claude_ai_skills: Function,
}

struct Visibility(Arc<dyn PolicyView>);

crate::registry::methods! {
    Visibility {
        "call" => sync async ok |this, name: String, subject: Option<SkillSubject>|
            skill_visibility(&*this.0, &name, &subject.unwrap_or_default()).await;
    }
}

struct Bundled(BundledSkillsDecision);

crate::registry::methods! {
    Bundled {
        "call" => ok |this| this.0.clone();
    }
}

struct Shell(Arc<dyn PolicyView>);

crate::registry::methods! {
    Shell {
        "call" => sync async ok |this, origin: ShellOrigin| shell_verdict(&*this.0, origin).await;
    }
}

struct SyncSkills(Arc<dyn PolicyView>);

crate::registry::methods! {
    SyncSkills {
        "call" => sync async ok |this| sync_skills_decision(&*this.0).await;
    }
}

/// `disableSkillShellExecution: true` in the SDK's `managedSettings` takes effect, but parent
/// settings are ignored where an administrator deploys managed settings
/// (claude-apps-gateway.md): the shell verdict reads the view under the parent rules.
pub async fn create_skill_settings_policy(
    view: Arc<dyn PolicyView>,
    context: &SkillPolicyContext,
) -> SkillSettingsPolicy {
    let effective = effective_view(Arc::clone(&view), &context.parent).await;
    let bundled = bundled_skills(&*view, &context.env).await;
    SkillSettingsPolicy {
        visibility: Function::new(Visibility(Arc::clone(&view))),
        bundled_skills_disabled: bundled.disabled,
        bundled: Function::new(Bundled(bundled)),
        shell: Function::new(Shell(effective)),
        sync_claude_ai_skills: Function::new(SyncSkills(view)),
    }
}

crate::registry::exports! {
    "settings/skills/policy" {
        "createSkillSettingsPolicy" => async ok |view: Port, context: SkillPolicyContext|
            create_skill_settings_policy(Arc::new(view), &context).await;
    }
}
