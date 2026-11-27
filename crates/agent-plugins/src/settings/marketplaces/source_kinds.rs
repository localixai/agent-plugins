//! Port of `src/settings/marketplaces/source-kinds.ts` at sha256:6191a10845c0.
//!
//! The kinds of marketplace source an allow or block entry names (the official union's `source`),
//! and the helpers that ask a registry of per-kind functions about one member. Its two functions
//! take a registry of functions (callbacks the boundary does not carry), so they stay TypeScript in
//! the package; nothing of it is exported.

use crate::js::JsValue;

/// The kinds of marketplace source an allow or block entry names (the official union's `source`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    /// A directory.
    Directory,
    /// A file.
    File,
    /// A git repository.
    Git,
    /// A GitHub repository.
    Github,
    /// A host pattern.
    HostPattern,
    /// An npm package.
    Npm,
    /// A path pattern.
    PathPattern,
    /// The settings file's own list.
    Settings,
    /// The skills directory.
    SkillsDir,
    /// A URL.
    Url,
}

impl SourceKind {
    /// The kind a source object names, if it names one.
    #[must_use]
    pub fn of(source: &JsValue) -> Option<Self> {
        match source.get("source")?.as_str()? {
            "directory" => Some(Self::Directory),
            "file" => Some(Self::File),
            "git" => Some(Self::Git),
            "github" => Some(Self::Github),
            "hostPattern" => Some(Self::HostPattern),
            "npm" => Some(Self::Npm),
            "pathPattern" => Some(Self::PathPattern),
            "settings" => Some(Self::Settings),
            "skills-dir" => Some(Self::SkillsDir),
            "url" => Some(Self::Url),
            _ => None,
        }
    }
}

/// Asks a registry about a member: the function of its kind, called on it.
pub fn by_kind<T>(registry: impl Fn(SourceKind, &JsValue) -> T, source: &JsValue) -> Option<T> {
    SourceKind::of(source).map(|kind| registry(kind, source))
}

/// Asks a registry of functions that take more than the member.
pub fn by_kind_with<Rest, T>(
    registry: impl Fn(SourceKind, &JsValue, &Rest) -> T,
    source: &JsValue,
    rest: &Rest,
) -> Option<T> {
    SourceKind::of(source).map(|kind| registry(kind, source, rest))
}
