//! Ported from `src/marketplace/node-deps-checks.ts`; only the core uses it now, so it has no wrapper.
//!
//! The checks that decide whether the dependency install applies and may run (plugins/loading.md,
//! "When the dependency install runs", "Limits on the dependency install").

use indexmap::{IndexMap, IndexSet};

use super::node_deps_lockfile::{LockfileFacts, LockfileName};
use super::node_deps_spec::{
    DEPENDENCY_SECTIONS, PackageJson, dependency_refusal, download_link_refusal, specifier_refusals,
};
use super::node_deps_types::{
    BareReason, DepsOrigin, NodeDepsInput, NodeDepsSkip, PackageManager, ReadPackageJson, SkipWhy,
    skip_for,
};
use crate::js::{JsValue, is_integer, key_order};

// ---- lockfiles: which one counts, and what each means ----

/// "If a plugin contains more than one of these lockfiles, Claude Code uses the first match."
/// (`LOCKFILE_NAMES`, in its order.)
const LOCKFILE_ORDER: [LockfileName; 3] = [
    LockfileName::BunLock,
    LockfileName::NpmShrinkwrap,
    LockfileName::PackageLock,
];

const fn manager_of(name: LockfileName) -> PackageManager {
    match name {
        LockfileName::BunLock => PackageManager::Bun,
        LockfileName::NpmShrinkwrap | LockfileName::PackageLock => PackageManager::Npm,
    }
}

/// "an npm lockfile needs a `lockfileVersion` of `2` or `3` ... a `bun.lock` needs a
/// `lockfileVersion` no higher than `2`".
fn readable_version(name: LockfileName, version: f64) -> bool {
    match name {
        LockfileName::BunLock => is_integer(version) && (0.0..=2.0).contains(&version),
        LockfileName::NpmShrinkwrap | LockfileName::PackageLock => version == 2.0 || version == 3.0,
    }
}

/// Frozen resolution: npm ci and bun install --frozen-lockfile, both without scripts (judgment:
/// the flags are the tools' own).
#[must_use]
pub const fn arguments(manager: PackageManager) -> &'static [&'static str] {
    match manager {
        PackageManager::Npm => &["ci", "--ignore-scripts"],
        PackageManager::Bun => &["install", "--frozen-lockfile", "--ignore-scripts"],
    }
}

// ---- the checks ----

/// The package.json and lockfile the install would use.
#[derive(Clone, Debug)]
pub struct Ready {
    /// The `package.json`.
    pub package_json: PackageJson,
    /// The lockfile that counts.
    pub lockfile: LockfileName,
    /// What it holds.
    pub facts: LockfileFacts,
    /// The package manager that reads it.
    pub manager: PackageManager,
}

type Check = fn(&Ready, &NodeDepsInput) -> Option<NodeDepsSkip>;

/// `typeof value === 'object' && value !== null && Object.keys(value).length > 0`.
fn has_entries(value: Option<&JsValue>) -> bool {
    match value {
        Some(JsValue::Object(members)) => !members.is_empty(),
        Some(JsValue::Array(items)) => !items.is_empty(),
        _ => false,
    }
}

/// What is refused per package manager: "npm `overrides` ... from an npm lockfile", "Bun
/// `patchedDependencies` ... from `bun.lock`".
fn manager_refusals(manager: PackageManager, package_json: &PackageJson) -> Option<NodeDepsSkip> {
    match manager {
        PackageManager::Npm => {
            has_entries(package_json.overrides.as_ref()).then(|| skip_for(BareReason::Overrides))
        }
        PackageManager::Bun => has_entries(package_json.patched_dependencies.as_ref())
            .then(|| skip_for(BareReason::PatchedDependencies)),
    }
}

fn unreadable_format(ready: &Ready, _input: &NodeDepsInput) -> Option<NodeDepsSkip> {
    let readable = ready
        .facts
        .version
        .is_some_and(|version| readable_version(ready.lockfile, version));
    (!readable).then(|| {
        SkipWhy::LockfileFormat {
            lockfile: ready.lockfile,
            version: ready.facts.version,
        }
        .into()
    })
}

fn manager_missing(ready: &Ready, input: &NodeDepsInput) -> Option<NodeDepsSkip> {
    (!input.available.contains(&ready.manager)).then(|| {
        SkipWhy::PackageManagerMissing {
            manager: ready.manager,
        }
        .into()
    })
}

fn sets_overrides(ready: &Ready, _input: &NodeDepsInput) -> Option<NodeDepsSkip> {
    manager_refusals(ready.manager, &ready.package_json)
}

fn not_registry(ready: &Ready, _input: &NodeDepsInput) -> Option<NodeDepsSkip> {
    let mut named: Vec<String> = specifier_refusals(&ready.package_json)
        .into_iter()
        .map(|entry| format!("{}: {}", entry.name, entry.reason))
        .collect();
    named.extend(ready.facts.locked.iter().filter_map(|entry| {
        let reason = dependency_refusal(&entry.origin)?;
        Some(format!("{}: {reason}", entry.name))
    }));
    (!named.is_empty()).then(|| {
        SkipWhy::NonRegistryDependency {
            dependencies: named
                .into_iter()
                .collect::<IndexSet<_>>()
                .into_iter()
                .collect(),
        }
        .into()
    })
}

fn not_pinned(ready: &Ready, _input: &NodeDepsInput) -> Option<NodeDepsSkip> {
    (!ready.facts.unpinned.is_empty()).then(|| {
        SkipWhy::UnpinnedDependency {
            packages: ready.facts.unpinned.clone(),
        }
        .into()
    })
}

fn insecure_download(ready: &Ready, input: &NodeDepsInput) -> Option<NodeDepsSkip> {
    let links: Vec<String> = ready
        .facts
        .downloads
        .iter()
        .filter(|link| download_link_refusal(link, input.default_registry.as_deref()).is_some())
        .cloned()
        .collect();
    (!links.is_empty()).then(|| SkipWhy::InsecureDownload { links }.into())
}

/// The dependency names that `package.json` and the lockfile do not list the same way.
fn differing(
    listed: Option<&IndexMap<String, String>>,
    locked: Option<&IndexMap<String, String>>,
) -> Vec<String> {
    let keys = |map: Option<&IndexMap<String, String>>| {
        map.map(|map| {
            key_order(map.keys())
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
    };
    let mut names = keys(listed).into_iter().collect::<IndexSet<_>>();
    names.extend(keys(locked));
    names
        .into_iter()
        .filter(|name| listed.and_then(|map| map.get(name)) != locked.and_then(|map| map.get(name)))
        .collect()
}

fn out_of_date(ready: &Ready, _input: &NodeDepsInput) -> Option<NodeDepsSkip> {
    let differences: Vec<String> = DEPENDENCY_SECTIONS
        .iter()
        .filter_map(|section| {
            let names = differing(
                ready.package_json.section(section),
                ready.facts.root.get(*section),
            );
            (!names.is_empty()).then(|| format!("{section}: {}", names.join(", ")))
        })
        .collect();
    (!differences.is_empty()).then(|| SkipWhy::LockfileMismatch { differences }.into())
}

/// The checks, in the order they run: the first that refuses is the skip.
pub const CHECKS: [Check; 7] = [
    unreadable_format,
    manager_missing,
    sets_overrides,
    not_registry,
    not_pinned,
    insecure_download,
    out_of_date,
];

// ---- before the checks: does an install apply at all ----

fn by_origin(input: &NodeDepsInput) -> Option<NodeDepsSkip> {
    match input.origin {
        DepsOrigin::Copied => None,
        // "Claude Code doesn't install the dependencies into the source directory"; they resolve
        // there.
        DepsOrigin::InPlace | DepsOrigin::Inline => Some(
            SkipWhy::LoadedInPlace {
                resolves_from: input.plugin_root.clone(),
            }
            .into(),
        ),
        // "Claude Code skips the Node.js package dependency install for a link-mode plugin"
        DepsOrigin::Linked => Some(skip_for(BareReason::LinkMode)),
    }
}

fn select_lockfile(input: &NodeDepsInput) -> Option<LockfileName> {
    LOCKFILE_ORDER
        .into_iter()
        .find(|name| input.lockfiles.get(*name).is_some())
}

/// What `prepare` comes to: the files the install would use, or why none applies (`isReady`
/// tells them apart in the TypeScript; here the variant does).
#[derive(Clone, Debug)]
pub enum Prepared {
    /// The install would use these.
    Ready(Box<Ready>),
    /// Why none applies.
    Skip(NodeDepsSkip),
}

/// The package.json and lockfile the install would use, or why none applies.
#[must_use]
pub fn prepare(input: &NodeDepsInput) -> Prepared {
    if let Some(early) = by_origin(input) {
        return Prepared::Skip(early);
    }
    let package_json = match &input.package_json {
        None => return Prepared::Skip(skip_for(BareReason::NoPackageJson)),
        Some(ReadPackageJson::Unreadable(_)) => {
            return Prepared::Skip(skip_for(BareReason::PackageJsonUnreadable));
        }
        Some(ReadPackageJson::Json(package_json)) => &**package_json,
    };
    let found = select_lockfile(input)
        .and_then(|lockfile| Some((lockfile, input.lockfiles.get(lockfile)?)));
    let Some((lockfile, facts)) = found else {
        return Prepared::Skip(if input.other_lockfiles.is_empty() {
            skip_for(BareReason::NoLockfile)
        } else {
            SkipWhy::UnsupportedLockfile {
                lockfiles: input.other_lockfiles.clone(),
            }
            .into()
        });
    };
    Prepared::Ready(Box::new(Ready {
        package_json: package_json.clone(),
        lockfile,
        facts: facts.clone(),
        manager: manager_of(lockfile),
    }))
}
