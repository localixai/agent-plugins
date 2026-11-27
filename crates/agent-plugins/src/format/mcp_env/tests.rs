use super::{expand_env_text, is_variable_name, split_default};
use crate::ports::now;

#[test]
fn references_expand_once_with_their_defaults() {
    let lookup = |name: &str| (name == "A").then(|| "${B}".to_owned());
    let expanded = now(expand_env_text("${A}-${B:-x}-${C}-${1}-${}", &lookup, None)).unwrap();
    assert_eq!(expanded.text, "${B}-x-${C}-${1}-${}");
    assert_eq!(expanded.missing, vec!["C".to_owned()]);
    let covered = |_: &str| true;
    let read = now(expand_env_text("a${A}", &lookup, Some(&covered))).unwrap();
    assert_eq!(
        (read.text.as_str(), read.credentials),
        ("a", vec!["A".to_owned()])
    );
    assert_eq!(split_default("N:-a:-b").fallback.as_deref(), Some("a:-b"));
    assert!(is_variable_name("_a1") && !is_variable_name("1a"));
}
