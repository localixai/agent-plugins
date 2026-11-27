//! Port of `src/rules/path/depth.ts`, switched.
//!
//! How deep a relative pattern matches (permissions.md, "How deep a pattern matches"): "A relative
//! pattern with a single directory segment, such as `src/**`, matches at different depths depending
//! on the rule type. Allow rules: `Edit(src/**)` matches only `<cwd>/src` and the files under it.
//! Deny and ask rules: `Read(secrets/**)` matches a directory named `secrets` at any depth under the
//! current directory. Every other pattern shape matches at the same depth in every rule type."
//!
//! "Single directory segment" is read as: one literal directory name (a wildcard is allowed in it, `**`
//! is not), one slash, and something below it (`src/**`, `src/*.ts`, `src/index.ts`), including a `**`
//! between that directory and the rest (`src/**` followed by `/*.ts`: still one named directory, so
//! it is widened). A pattern that begins with `/`, `**` or `!`, or holds two directories, is another
//! shape. Widening `src/index.ts` and `src/**` followed by more path is a judgment (the docs show
//! `src/**` only); it errs towards blocking (spec/judgments.md).

use indexmap::IndexMap;

use crate::registry::Function;
use crate::rules::types::PermissionBehavior;

/// `SINGLE_DIRECTORY`, `/^(?!\*\*)(?!\.{1,2}\/)[^/!][^/]*\/(?:[^/]+\/?|\*\*\/.+)$/`: a first segment
/// that is neither `**…`, `./…` nor `../…` and does not begin with `/` or `!`, a slash, and then
/// either one more segment (with an optional slash) or `**/` and any text of one line.
fn is_single_directory(pattern: &str) -> bool {
    if pattern.starts_with("**") || pattern.starts_with("./") || pattern.starts_with("../") {
        return false;
    }
    if pattern.starts_with(['/', '!']) {
        return false;
    }
    let Some((first, rest)) = pattern.split_once('/') else {
        return false;
    };
    if first.is_empty() {
        return false;
    }
    let one_segment = rest
        .strip_suffix('/')
        .unwrap_or(rest)
        .chars()
        .all(|character| character != '/')
        && !rest.is_empty()
        && rest != "/";
    let below_globstar = rest.strip_prefix("**/").is_some_and(|after| {
        !after.is_empty()
            && !after
                .chars()
                .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    });
    one_segment || below_globstar
}

fn widen(pattern: &str) -> String {
    if is_single_directory(pattern) {
        format!("**/{pattern}")
    } else {
        pattern.to_owned()
    }
}

fn keep(pattern: &str) -> String {
    pattern.to_owned()
}

/// The pattern a rule of `behavior` matches with: only deny and ask rules reach every depth.
#[must_use]
pub fn depth_of(behavior: PermissionBehavior, pattern: &str) -> String {
    match behavior {
        PermissionBehavior::Allow => keep(pattern),
        PermissionBehavior::Deny | PermissionBehavior::Ask => widen(pattern),
    }
}

/// One behavior's entry of the table, a function the host calls.
#[derive(Debug)]
struct Depth(PermissionBehavior);

crate::registry::methods! {
    Depth {
        "call" => ok |this, pattern: String| depth_of(this.0, &pattern);
    }
}

/// The pattern a rule of each behaviour matches with: only deny and ask rules reach every depth.
const DEPTH: [(&str, PermissionBehavior); 3] = [
    ("allow", PermissionBehavior::Allow),
    ("deny", PermissionBehavior::Deny),
    ("ask", PermissionBehavior::Ask),
];

crate::registry::exports! {
    "rules/path/depth" {
        "DEPTH" => ok || DEPTH
            .map(|(name, behavior)| (name, Function::new(Depth(behavior))))
            .into_iter()
            .collect::<IndexMap<_, _>>();
    }
}

#[cfg(test)]
mod tests {
    use super::is_single_directory;

    #[test]
    fn a_single_directory_pattern_is_one_named_directory_with_something_below() {
        assert!(is_single_directory("src/**"));
        assert!(is_single_directory("src/*.ts"));
        assert!(is_single_directory("src/index.ts"));
        assert!(is_single_directory("src/**/*.ts"));
        assert!(is_single_directory("src/dir/"));
        assert!(!is_single_directory("**/x"));
        assert!(!is_single_directory("./src/x"));
        assert!(!is_single_directory("../src/x"));
        assert!(!is_single_directory("/src/x"));
        assert!(!is_single_directory("!src/x"));
        assert!(!is_single_directory("src/a/b"));
        assert!(!is_single_directory("src/"));
        assert!(!is_single_directory("src"));
    }
}
