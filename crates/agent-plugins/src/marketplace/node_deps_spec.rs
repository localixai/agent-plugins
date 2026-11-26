//! Port of `src/marketplace/node-deps-spec.ts`, switched.
//!
//! What counts as a registry package (plugins/loading.md, "Limits on the dependency install":
//! "every dependency must be a registry package ... A plugin with a git, GitHub, folder, workspace,
//! or linked dependency gets no install", "a download link in the lockfile must use `https`, unless
//! it points at the installing user's own default npm registry"; plugins/marketplace-reference.md,
//! "npm plugin source": a git address, a folder or `file:` path, an `npm:` alias and some tarball
//! links are refused). One classifier of a specifier serves the `package` value of an npm source and
//! the dependencies of a plugin's `package.json`; a registry of rules typed over the kinds says what
//! each kind means for each of the two (K1).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::install_failures::NOT_A_LINK;
use crate::js::regexp::RegExp;
use crate::js::url::parse;
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;

/// What kind of specifier a value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecifierKind {
    /// A registry package.
    #[serde(rename = "registry")]
    Registry,
    /// An `npm:` alias.
    #[serde(rename = "alias")]
    Alias,
    /// A folder or `file:` path.
    #[serde(rename = "folder")]
    Folder,
    /// A git scheme (`github:`, `git+`).
    #[serde(rename = "git-scheme")]
    GitScheme,
    /// A git address (`.git` link, `owner/name`).
    #[serde(rename = "git-address")]
    GitAddress,
    /// A workspace link.
    #[serde(rename = "workspace")]
    Workspace,
    /// A `link:` dependency.
    #[serde(rename = "linked")]
    Linked,
    /// A tarball link on a refused host.
    #[serde(rename = "tarball-host")]
    TarballHost,
    /// A tarball link over http.
    #[serde(rename = "tarball-http")]
    TarballHttp,
    /// A tarball link over https.
    #[serde(rename = "tarball-https")]
    TarballHttps,
}

/// `pattern.test(text)` for a constant pattern; one that does not compile matches nothing.
fn matches(pattern: &str, flags: &str, text: &str) -> bool {
    RegExp::new(pattern, flags).is_ok_and(|mut regexp| regexp.test(text))
}

const LINK: &str = r"^https?:\/\/";

fn is_link(value: &str) -> bool {
    matches(LINK, "i", value)
}

/// Hosts whose tarball links are refused, GitHub release downloads included.
const REFUSED_TARBALL_HOSTS: [&str; 5] = [
    "github.com",
    "gist.github.com",
    "gitlab.com",
    "bitbucket.org",
    "git.sr.ht",
];
const GITLAB_REGISTRY_PATH: &str = "/api/v4/";

fn on_refused_host(link: &str) -> bool {
    let Some(url) = parse(link) else {
        return false;
    };
    let host = url.hostname.to_lowercase();
    let named = REFUSED_TARBALL_HOSTS
        .iter()
        .any(|name| host == *name || host.ends_with(&format!(".{name}")));
    named && !(host == "gitlab.com" && url.pathname.starts_with(GITLAB_REGISTRY_PATH))
}

/// One detector per kind, in the order the docs list them; the first that matches is the kind.
fn detects(kind: SpecifierKind, value: &str) -> bool {
    match kind {
        SpecifierKind::Alias => matches(r"^npm:", "i", value) || value.contains("@npm:"),
        SpecifierKind::Folder => {
            matches(r"^file:", "i", value)
                || matches(
                    r"^(?:\.{1,2}(?:[\\/]|$)|[\\/]|~(?:[\\/]|$)|[A-Za-z]:[\\/])",
                    "",
                    value,
                )
        }
        // "github:acme/formatter" is the docs' own example of a value that "is not an http or https link".
        SpecifierKind::GitScheme => matches(
            r"^(?:git\+|git:|git@|ssh:|github:|gitlab:|bitbucket:|gist:)",
            "i",
            value,
        ),
        SpecifierKind::Workspace => matches(r"^workspace:", "i", value),
        SpecifierKind::Linked => matches(r"^link:", "i", value),
        SpecifierKind::GitAddress => {
            (is_link(value) && matches(r"\.git(?:[#?].*)?$", "i", value))
                || (!value.starts_with('@') && matches(r"^[^\s/:@]+\/[^\s/:]+(?:#.*)?$", "", value))
        }
        SpecifierKind::TarballHost => is_link(value) && on_refused_host(value),
        SpecifierKind::TarballHttp => matches(r"^http:\/\/", "i", value),
        SpecifierKind::TarballHttps => matches(r"^https:\/\/", "i", value),
        SpecifierKind::Registry => false,
    }
}

/// In the order the docs list them.
const DETECTORS: [SpecifierKind; 9] = [
    SpecifierKind::Alias,
    SpecifierKind::Folder,
    SpecifierKind::GitScheme,
    SpecifierKind::Workspace,
    SpecifierKind::Linked,
    SpecifierKind::GitAddress,
    SpecifierKind::TarballHost,
    SpecifierKind::TarballHttp,
    SpecifierKind::TarballHttps,
];

/// The kind of a specifier.
#[must_use]
pub fn classify_specifier(value: &str) -> SpecifierKind {
    DETECTORS
        .into_iter()
        .find(|kind| detects(*kind, value))
        .unwrap_or(SpecifierKind::Registry)
}

// ---- links to the user's own default registry ----

fn trailing_slash(url: &str) -> &str {
    url.trim_end_matches('/')
}

/// True when `link` lies under the installing user's own default npm registry (compared by whole segments).
fn is_default_registry_link(link: &str, default_registry: Option<&str>) -> bool {
    default_registry.is_some_and(|registry| {
        format!("{}/", trailing_slash(link))
            .to_lowercase()
            .starts_with(&format!("{}/", trailing_slash(registry)).to_lowercase())
    })
}

/// True when `registry` is the user's own default registry itself.
#[must_use]
pub fn is_default_registry(registry: &str, default_registry: Option<&str>) -> bool {
    default_registry.is_some_and(|default| {
        trailing_slash(registry).to_lowercase() == trailing_slash(default).to_lowercase()
    })
}

/// The refusal of a download link: it "must use `https`, unless it points at the installing user's
/// own default npm registry". Any other scheme (`ftp:`, `file:`) and a link with none is refused too.
#[must_use]
pub fn download_link_refusal(link: &str, default_registry: Option<&str>) -> Option<String> {
    if matches(r"^https:\/\/", "i", link) || is_default_registry_link(link, default_registry) {
        None
    } else {
        Some(format!("the download link {link} does not use https"))
    }
}

// ---- what each kind means ----

const NOT_A_REGISTRY_PACKAGE: &str = "it is a tarball link, not a registry package";
const GIT_DEPENDENCY: &str = "it is a git or GitHub dependency";

/// The reason an npm `package` value of this kind is refused, if it is.
fn package_value_reason(
    kind: SpecifierKind,
    value: &str,
    default_registry: Option<&str>,
) -> Option<&'static str> {
    match kind {
        SpecifierKind::Registry | SpecifierKind::TarballHttps => None,
        // "use the package's own name" for the source; the alias still resolves from the registry (judgment).
        SpecifierKind::Alias => Some("it is an npm: alias; use the package's own name"),
        SpecifierKind::Folder => Some(
            "it is a folder or file: path; use a relative path source for a folder in the marketplace",
        ),
        SpecifierKind::GitScheme => Some(NOT_A_LINK),
        SpecifierKind::GitAddress => Some("it is a git address"),
        SpecifierKind::Workspace => Some("it is a workspace link, not a registry package"),
        SpecifierKind::Linked => Some("it is a link: dependency, not a registry package"),
        SpecifierKind::TarballHost => Some(
            "it is a tarball link on github.com, gist.github.com, gitlab.com, bitbucket.org or git.sr.ht (only a GitLab npm registry link under gitlab.com/api/v4/ is allowed)",
        ),
        SpecifierKind::TarballHttp => {
            if is_default_registry_link(value, default_registry) {
                None
            } else {
                Some(
                    "it is a tarball link over http, which is allowed only on your own default npm registry",
                )
            }
        }
    }
}

/// The reason a `package.json` dependency of this kind gets no install, if it gets none.
const fn dependency_reason(kind: SpecifierKind) -> Option<&'static str> {
    match kind {
        SpecifierKind::Registry | SpecifierKind::Alias => None,
        SpecifierKind::Folder => Some("it is a folder dependency"),
        SpecifierKind::GitScheme | SpecifierKind::GitAddress => Some(GIT_DEPENDENCY),
        SpecifierKind::Workspace => Some("it is a workspace dependency"),
        SpecifierKind::Linked => Some("it is a linked dependency"),
        SpecifierKind::TarballHost | SpecifierKind::TarballHttp | SpecifierKind::TarballHttps => {
            Some(NOT_A_REGISTRY_PACKAGE)
        }
    }
}

/// Why an npm source's `package` value is refused before anything is fetched.
#[must_use]
pub fn package_value_refusal(value: &str, default_registry: Option<&str>) -> Option<&'static str> {
    package_value_reason(classify_specifier(value), value, default_registry)
}

/// Why a `package.json` dependency specifier gets no install; `undefined` for a registry package.
#[must_use]
pub fn dependency_refusal(specifier: &str) -> Option<&'static str> {
    dependency_reason(classify_specifier(specifier))
}

/// Which kinds a lockfile entry may have and still be a registry package, by where the value is read:
/// `(resolved, id)`.
const fn registry_kind(kind: SpecifierKind) -> (bool, bool) {
    match kind {
        SpecifierKind::Registry | SpecifierKind::Alias => (true, true),
        SpecifierKind::Folder
        | SpecifierKind::GitScheme
        | SpecifierKind::GitAddress
        | SpecifierKind::Workspace
        | SpecifierKind::Linked
        | SpecifierKind::TarballHost => (false, false),
        // A `resolved` link is how npm records a registry download; a bun id with a link is a tarball dependency.
        SpecifierKind::TarballHttp | SpecifierKind::TarballHttps => (true, false),
    }
}

/// Whether an npm lockfile `resolved` value is a registry download (its scheme is checked as a download link).
#[must_use]
pub fn is_registry_origin(resolved: &str) -> bool {
    registry_kind(classify_specifier(resolved)).0
}

/// Whether the specifier of a `bun.lock` entry id (`name@spec`) is a registry package.
#[must_use]
pub fn is_registry_id(specifier: &str) -> bool {
    registry_kind(classify_specifier(specifier)).1
}

// ---- package.json ----

/// The sections of a `package.json` that list dependencies.
pub const DEPENDENCY_SECTIONS: [&str; 4] = [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

/// The string entries of a dependency object; anything else lists nothing.
#[must_use]
pub fn read_dependency_map(value: &JsValue) -> IndexMap<String, String> {
    let Some(members) = value.as_object().filter(|_| is_record(value)) else {
        return IndexMap::new();
    };
    property_order(members)
        .into_iter()
        .filter_map(|(name, entry)| Some((name.clone(), entry.as_str()?.to_owned())))
        .collect()
}

/// A parsed `package.json` value narrowed to the parts the install reads; `'unreadable'` for a value that is not an object.
#[must_use]
pub fn read_package_json(value: &JsValue) -> JsValue {
    let Some(members) = value.as_object().filter(|_| is_record(value)) else {
        return JsValue::from("unreadable");
    };
    let mut out = JsObject::new();
    for section in DEPENDENCY_SECTIONS {
        let map = read_dependency_map(members.get(section).unwrap_or(&JsValue::Undefined));
        if !map.is_empty() {
            let entries: JsObject = map
                .into_iter()
                .map(|(name, specifier)| (name, JsValue::from(specifier)))
                .collect();
            out.insert(section.to_owned(), JsValue::Object(entries));
        }
    }
    for kept in ["overrides", "patchedDependencies"] {
        if let Some(entry) = members.get(kept) {
            out.insert(kept.to_owned(), entry.clone());
        }
    }
    JsValue::Object(out)
}

/// The parts of a plugin's `package.json` the dependency install looks at.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    /// `dependencies`.
    #[serde(default)]
    pub dependencies: Option<IndexMap<String, String>>,
    /// `devDependencies`.
    #[serde(default)]
    pub dev_dependencies: Option<IndexMap<String, String>>,
    /// `optionalDependencies`.
    #[serde(default)]
    pub optional_dependencies: Option<IndexMap<String, String>>,
    /// `peerDependencies`.
    #[serde(default)]
    pub peer_dependencies: Option<IndexMap<String, String>>,
}

impl PackageJson {
    /// `packageJson[section]`.
    fn section(&self, section: &str) -> Option<&IndexMap<String, String>> {
        match section {
            "dependencies" => self.dependencies.as_ref(),
            "devDependencies" => self.dev_dependencies.as_ref(),
            "optionalDependencies" => self.optional_dependencies.as_ref(),
            "peerDependencies" => self.peer_dependencies.as_ref(),
            _ => None,
        }
    }
}

/// One dependency that gets no install.
#[derive(Clone, Debug, Serialize)]
pub struct SpecifierRefusal {
    /// Its section.
    pub section: &'static str,
    /// The package's name.
    pub name: String,
    /// What it asks for.
    pub specifier: String,
    /// Why it gets no install.
    pub reason: &'static str,
}

/// Every dependency of `package.json` that is not a registry package, with the reason.
#[must_use]
pub fn specifier_refusals(package_json: &PackageJson) -> Vec<SpecifierRefusal> {
    DEPENDENCY_SECTIONS
        .into_iter()
        .flat_map(|section| {
            package_json
                .section(section)
                .into_iter()
                .flatten()
                .filter_map(move |(name, specifier)| {
                    let reason = dependency_refusal(specifier)?;
                    Some(SpecifierRefusal {
                        section,
                        name: name.clone(),
                        specifier: specifier.clone(),
                        reason,
                    })
                })
        })
        .collect()
}

crate::registry::exports! {
    "marketplace/node-deps-spec" {
        "classifySpecifier" => ok |value: String| classify_specifier(&value);
        "isDefaultRegistry" => ok |registry: String, default_registry: Option<String>|
            is_default_registry(&registry, default_registry.as_deref());
        "downloadLinkRefusal" => ok |link: String, default_registry: Option<String>|
            download_link_refusal(&link, default_registry.as_deref());
        "packageValueRefusal" => ok |value: String, default_registry: Option<String>|
            package_value_refusal(&value, default_registry.as_deref());
        "dependencyRefusal" => ok |specifier: String| dependency_refusal(&specifier);
        "isRegistryOrigin" => ok |resolved: String| is_registry_origin(&resolved);
        "isRegistryId" => ok |specifier: String| is_registry_id(&specifier);
        "DEPENDENCY_SECTIONS" => ok || DEPENDENCY_SECTIONS;
        "readDependencyMap" => ok |value: JsValue| read_dependency_map(&value);
        "readPackageJson" => ok |value: JsValue| read_package_json(&value);
        "specifierRefusals" => ok |package_json: PackageJson| specifier_refusals(&package_json);
    }
}
