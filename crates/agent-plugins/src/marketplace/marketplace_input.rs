//! Port of `src/marketplace/marketplace-input.ts`, switched.
//!
//! The marketplace source `claude plugin marketplace add <source>` builds from the string you type
//! (plugins/cli-reference.md, "plugin marketplace add": the table of forms, the unrecognized-source
//! and bare-host messages, `--sparse`; plugins/marketplace-reference.md, "Marketplace sources"): pure
//! text in, a source or an issue out. The order the forms are tried in, and what the docs leave
//! open, are in spec/judgments.md.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::source_safety::is_github_repository;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::marketplace_source::DEFAULT_MARKETPLACE_PATH;
use crate::format::marketplace::types::MarketplaceSource;
use crate::format::path_text::PathPlatform;
use crate::js::regexp::RegExp;
use crate::js::trim;

/// The options of `parseMarketplaceInput`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct MarketplaceInputOptions {
    /// `.\`, `..\` and `C:\` paths are accepted only on Windows.
    #[serde(default)]
    pub platform: Option<PathPlatform>,
    /// The `--sparse` directories; for `github` and `git` sources only.
    #[serde(default)]
    pub sparse: Option<Vec<String>>,
}

/// A source, or the issue that says the text is none.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Parsed {
    /// The source.
    Source(MarketplaceSource),
    /// Why there is none.
    Issue(PluginIssue),
}

const UNRECOGNIZED: &str =
    "Invalid marketplace source format. Try: owner/repo, https://..., or ./path";

fn refused(message: &str) -> Parsed {
    Parsed::Issue(error("source", message))
}

/// A JavaScript regular expression literal, compiled once (`None` never happens: a test compiles each).
type Literal = LazyLock<Option<RegExp>>;

fn test(pattern: &Literal, text: &str) -> bool {
    pattern
        .as_ref()
        .is_some_and(|pattern| pattern.test_shared(text))
}

// ---- local paths ----

static POSIX_PATH: Literal = LazyLock::new(|| RegExp::new(r"^(?:\.\.?\/|\/|~\/)", "").ok());
static WINDOWS_PATH: Literal = LazyLock::new(|| RegExp::new(r"^(?:\.\.?\\|[A-Za-z]:\\)", "").ok());

fn is_path(text: &str, platform: PathPlatform) -> bool {
    test(&POSIX_PATH, text) || (platform == PathPlatform::Windows && test(&WINDOWS_PATH, text))
}

/// A path to a `.json` file is a `file` source; any other path is a directory.
fn path_source(path: &str) -> Parsed {
    let path = path.to_owned();
    Parsed::Source(if path.to_lowercase().ends_with(".json") {
        MarketplaceSource::File { path }
    } else {
        MarketplaceSource::Directory { path }
    })
}

// ---- http(s) URLs ----

static HTTP_URL: Literal =
    LazyLock::new(|| RegExp::new(r"^(https?):\/\/([^/?#]+)(\/[^?#]*)?(\?[^#]*)?(#.*)?$", "i").ok());
const FORGES: [&str; 2] = ["github.com", "gitlab.com"];

/// `#ref` after a git form: the text before it and the ref, which may not be empty.
struct Split<'a> {
    base: &'a str,
    reference: Option<&'a str>,
}

/// `#ref` after a git form: the text before it and the ref, which may not be empty.
fn split_ref(text: &str) -> Option<Split<'_>> {
    let Some((base, reference)) = text.split_once('#') else {
        return Some(Split {
            base: text,
            reference: None,
        });
    };
    (!reference.is_empty()).then_some(Split {
        base,
        reference: Some(reference),
    })
}

fn git(url: String, reference: Option<&str>) -> Parsed {
    Parsed::Source(MarketplaceSource::Git {
        url,
        path: DEFAULT_MARKETPLACE_PATH.to_owned(),
        reference: reference.map(str::to_owned),
        sparse_paths: None,
        skip_lfs: None,
    })
}

/// Segments of a forge path that are not part of a repository name: GitLab's `/-/` separates a project from its pages.
const FORGE_SEPARATOR: &str = "-";

/// A repository on github.com (exactly `owner/repo`) or gitlab.com (a namespace and a project,
/// subgroups included). A path to a file in a repository (`.json`, or after `/-/`) is not one.
fn is_forge_repository(host: &str, path: &str) -> bool {
    let forge = host.to_lowercase();
    if !FORGES.iter().any(|known| *known == forge) {
        return false;
    }
    let segments: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let last = segments.last().copied().unwrap_or_default();
    let sized = if forge == "github.com" {
        segments.len() == 2
    } else {
        segments.len() >= 2
    };
    sized && !segments.contains(&FORGE_SEPARATOR) && !last.to_lowercase().ends_with(".json")
}

/// A URL that names a git repository, and whether its `.git` is still to be appended.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CloneForm {
    AsIs,
    AppendGit,
}

/// A URL that names a git repository, and whether its `.git` is still to be appended.
fn clone_form(host: &str, path: &str, has_query: bool) -> Option<CloneForm> {
    if path.contains("/_git/") {
        return Some(CloneForm::AsIs);
    }
    if has_query {
        return None;
    }
    if path.ends_with(".git") {
        return Some(CloneForm::AsIs);
    }
    is_forge_repository(host, path).then_some(CloneForm::AppendGit)
}

fn from_http(text: &str) -> Option<Parsed> {
    let parts = HTTP_URL.as_ref()?.clone().exec(text)?;
    let part = |index: usize| parts.captures.get(index).cloned().flatten();
    let split = split_ref(text);
    let form = clone_form(
        &part(2).unwrap_or_default(),
        &part(3).unwrap_or_default(),
        part(4).is_some(),
    );
    let Some(form) = form else {
        return Some(Parsed::Source(MarketplaceSource::Url {
            url: text.to_owned(),
            headers: None,
            headers_helper: None,
        }));
    };
    let Some(split) = split else {
        return Some(refused(UNRECOGNIZED));
    };
    let base = split.base.trim_end_matches('/');
    let url = if form == CloneForm::AppendGit {
        format!("{base}.git")
    } else {
        base.to_owned()
    };
    Some(git(url, split.reference))
}

// ---- SSH and GitHub shorthand ----

static SSH: Literal = LazyLock::new(|| RegExp::new(r"^[^\s/@:]+@[^\s/@:]+:(?!\/\/)\S+$", "").ok());

fn from_ssh(text: &str) -> Option<Parsed> {
    if !test(&SSH, text) {
        return None;
    }
    Some(match split_ref(text) {
        None => refused(UNRECOGNIZED),
        Some(split) => git(split.base.to_owned(), split.reference),
    })
}

fn from_shorthand(text: &str) -> Option<Parsed> {
    let (owner, rest) = text.split_once('/')?;
    // `rest.search(/[#@]/)`: the first `#` or `@`.
    let cut = rest.find(['#', '@']);
    let repo = cut.and_then(|cut| rest.get(..cut)).unwrap_or(rest);
    let reference = cut.and_then(|cut| rest.get(cut + 1..));
    // An empty `#ref` is an unrecognized format, not a misspelled shorthand (spec J15.9).
    if reference == Some("")
        && cut
            .and_then(|cut| rest.get(cut..))
            .is_some_and(|at| at.starts_with('#'))
    {
        return Some(refused(UNRECOGNIZED));
    }
    if !is_github_repository(owner, repo) || reference == Some("") {
        return Some(refused(&format!(
            "'{text}' is not a valid GitHub owner/repo shorthand. For a git repo, use the full https:// clone URL from your host (typically ending in .git — some hosts like Azure DevOps omit it). For a hosted marketplace.json, use its https:// URL. For a local path, use ./ or an absolute path."
        )));
    }
    Some(Parsed::Source(MarketplaceSource::Github {
        repo: format!("{owner}/{repo}"),
        path: DEFAULT_MARKETPLACE_PATH.to_owned(),
        reference: reference.map(str::to_owned),
        sparse_paths: None,
        skip_lfs: None,
    }))
}

// ---- --sparse ----

/// Which sources take `--sparse`: "`github` and `git` sources only" (`TAKES_SPARSE`), with the
/// directories set on one that does.
fn with_sparse_paths(source: MarketplaceSource, sparse: &[String]) -> Option<MarketplaceSource> {
    let sparse = Some(sparse.to_vec());
    match source {
        MarketplaceSource::Github {
            repo,
            path,
            reference,
            skip_lfs,
            ..
        } => Some(MarketplaceSource::Github {
            repo,
            path,
            reference,
            sparse_paths: sparse,
            skip_lfs,
        }),
        MarketplaceSource::Git {
            url,
            path,
            reference,
            skip_lfs,
            ..
        } => Some(MarketplaceSource::Git {
            url,
            path,
            reference,
            sparse_paths: sparse,
            skip_lfs,
        }),
        MarketplaceSource::Url { .. }
        | MarketplaceSource::Npm { .. }
        | MarketplaceSource::File { .. }
        | MarketplaceSource::Directory { .. }
        | MarketplaceSource::Settings { .. }
        | MarketplaceSource::SkillsDir {}
        | MarketplaceSource::HostPattern { .. }
        | MarketplaceSource::PathPattern { .. } => None,
    }
}

fn with_sparse(parsed: Parsed, sparse: Option<&[String]>) -> Parsed {
    let (Some(sparse), Parsed::Source(source)) =
        (sparse.filter(|sparse| !sparse.is_empty()), &parsed)
    else {
        return parsed;
    };
    with_sparse_paths(source.clone(), sparse).map_or_else(
        || refused("--sparse applies to github and git sources only"),
        Parsed::Source,
    )
}

/// The source the input stands for, or the issue that says it is none.
#[must_use]
pub fn parse_marketplace_input(text: &str, options: &MarketplaceInputOptions) -> Parsed {
    let input = trim(text);
    let platform = options.platform.unwrap_or(PathPlatform::Posix);
    let parsed = is_path(input, platform)
        .then(|| path_source(input))
        .or_else(|| from_http(input))
        .or_else(|| from_ssh(input))
        .or_else(|| from_shorthand(input))
        .unwrap_or_else(|| refused(UNRECOGNIZED));
    with_sparse(parsed, options.sparse.as_deref())
}

crate::registry::exports! {
    "marketplace/marketplace-input" {
        "parseMarketplaceInput" => ok |text: String, options: Option<MarketplaceInputOptions>|
            parse_marketplace_input(&text, &options.unwrap_or_default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_literal_compiles() {
        assert!(POSIX_PATH.is_some() && WINDOWS_PATH.is_some());
        assert!(HTTP_URL.is_some() && SSH.is_some());
    }

    #[test]
    fn an_ssh_address_is_a_git_source() {
        let parsed = parse_marketplace_input(
            "git@example.com:team/m.git#v1",
            &MarketplaceInputOptions::default(),
        );
        assert!(matches!(
            parsed,
            Parsed::Source(MarketplaceSource::Git { reference: Some(ref r), .. }) if r == "v1"
        ));
    }

    #[test]
    fn an_empty_hash_ref_is_unrecognized() {
        let parsed = parse_marketplace_input("acme/m#", &MarketplaceInputOptions::default());
        assert!(matches!(parsed, Parsed::Issue(ref issue) if issue.message == UNRECOGNIZED));
    }
}
