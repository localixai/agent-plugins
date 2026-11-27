//! Port of `src/loading/resolve/content.ts`, switched.
//!
//! Substitutes `${...}` in the Markdown body of a skill, command or agent
//! (plugins/manifest-reference.md, "Where each variable resolves": "Anywhere in the Markdown
//! body", nothing exported).

use super::prepared::{PreparedSubstituter, prepare_substituter};
use super::substitute::{Substituted, SubstitutionScope};
use super::surfaces::{SubstitutionSurface, substitute_surface, surface_policy_of};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::registry::{Function, Port};

/// Substitutes `${...}` in the Markdown body of a skill, command or agent
/// (plugins/manifest-reference.md, "Where each variable resolves": "Anywhere in the Markdown
/// body", nothing exported). The three path variables resolve, `${user_config.KEY}` resolves for
/// non-sensitive options and a sensitive one becomes `SENSITIVE_PLACEHOLDER` (the page words the
/// `user_config` rule for skill and agent content; a command body takes it too on the strength of
/// the table's single row, a reading recorded in `spec/judgments.md`); `${VAR}` of the environment
/// stays for the model to read. Variables the skill owns (`CLAUDE_SKILL_DIR`, `CLAUDE_SESSION_ID`,
/// `CLAUDE_EFFORT`) come in `scope.extra`.
///
/// It takes the body: the caller splits the frontmatter off (`parseFrontmatter`), and the
/// `$ARGUMENTS` placeholders and the `allowed-tools` rules are the skills' own. A reference with no
/// value stays as written and is a warning at `path`.
pub async fn substitute_content<L: Locations + Clone, E: Environment>(
    body: &str,
    scope: &SubstitutionScope<L, E>,
    path: &str,
) -> Substituted {
    substitute_surface(SubstitutionSurface::MarkdownContent, body, scope, path).await
}

/// `prepareContentSubstitution`'s function: a prepared substituter and the content's policy.
pub struct ContentSubstitution<L, E>(PreparedSubstituter<L, E>);

impl<L, E: Environment> ContentSubstitution<L, E> {
    /// The body substituted, with the same rules as `substituteContent`.
    pub async fn call(&self, body: &str, path: &str) -> Substituted {
        let policy = surface_policy_of(SubstitutionSurface::MarkdownContent);
        self.0.run(body, &policy, path).await
    }
}

/// `substituteContent` for a host that reads a body synchronously (a lazily loaded skill): the
/// variables are asked for once, by `prepareSubstituter`, and each call after is synchronous, with
/// the same rules.
pub async fn prepare_content_substitution<L: Locations + Clone, E>(
    scope: SubstitutionScope<L, E>,
) -> ContentSubstitution<L, E> {
    ContentSubstitution(prepare_substituter(scope).await)
}

crate::registry::methods! {
    ContentSubstitution<Port, Port> {
        "call" => sync async ok |this, body: String, path: Option<String>|
            this.call(&body, path.as_deref().unwrap_or("content")).await;
    }
}

crate::registry::exports! {
    "loading/resolve/content" {
        "substituteContent" => async ok |body: String, scope: SubstitutionScope, path: Option<String>|
            substitute_content(&body, &scope, path.as_deref().unwrap_or("content")).await;
        "prepareContentSubstitution" => async ok |scope: SubstitutionScope|
            Function::new(prepare_content_substitution(scope).await);
    }
}
