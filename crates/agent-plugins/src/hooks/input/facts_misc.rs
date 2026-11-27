//! Ported from `src/hooks/input/facts-misc.ts`; only the core uses it now, so it has no wrapper.
//!
//! Facts of the remaining events (hooks.md, "Notification input", "Elicitation", "ConfigChange
//! input", "CwdChanged input", "WorktreeCreate input", "InstructionsLoaded input", "FileChanged
//! input", "DirectoryAdded input").

use std::sync::LazyLock;

use super::facts_shared::optional;
use crate::zod::{self, Schema, fields};

static ELICITATION_MODE: LazyLock<Schema> = LazyLock::new(|| zod::r#enum(["form", "url"]));

/// The facts of `Notification`.
pub static NOTIFICATION_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "message" => zod::string(),
        "title" => optional(&zod::string()),
        "notification_type" => zod::string(),
    })
});

/// The facts of `Elicitation`.
pub static ELICITATION_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "mcp_server_name" => zod::string(),
        "message" => zod::string(),
        "mode" => optional(&ELICITATION_MODE),
        "url" => optional(&zod::string()),
        "elicitation_id" => optional(&zod::string()),
        "requested_schema" => optional(&zod::record(zod::string(), zod::unknown())),
    })
});

/// The facts of `ElicitationResult`.
pub static ELICITATION_RESULT_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "mcp_server_name" => zod::string(),
        "elicitation_id" => optional(&zod::string()),
        "mode" => optional(&ELICITATION_MODE),
        "action" => zod::r#enum(["accept", "decline", "cancel"]),
        "content" => optional(&zod::record(zod::string(), zod::unknown())),
    })
});

/// The facts of `ConfigChange`.
pub static CONFIG_CHANGE_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "source" => zod::r#enum([
            "user_settings",
            "project_settings",
            "local_settings",
            "policy_settings",
            "skills",
        ]),
        "file_path" => optional(&zod::string()),
    })
});

/// The facts of `CwdChanged`.
pub static CWD_CHANGED_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "old_cwd" => zod::string(),
        "new_cwd" => zod::string(),
    })
});

/// The facts of `WorktreeCreate`.
pub static WORKTREE_CREATE_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "name" => zod::string(),
    })
});

/// The facts of `WorktreeRemove`.
pub static WORKTREE_REMOVE_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "worktree_path" => zod::string(),
    })
});

/// The facts of `InstructionsLoaded`.
pub static INSTRUCTIONS_LOADED_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "file_path" => zod::string(),
        "memory_type" => zod::r#enum(["User", "Project", "Local", "Managed"]),
        "load_reason" => zod::r#enum([
            "session_start",
            "nested_traversal",
            "path_glob_match",
            "include",
            "compact",
        ]),
        "globs" => optional(&zod::array(zod::string())),
        "trigger_file_path" => optional(&zod::string()),
        "parent_file_path" => optional(&zod::string()),
    })
});

/// The facts of `FileChanged`.
pub static FILE_CHANGED_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "file_path" => zod::string(),
        "event" => zod::r#enum(["change", "add", "unlink"]),
    })
});

/// The facts of `DirectoryAdded`.
pub static DIRECTORY_ADDED_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "directory" => zod::string(),
        "source" => zod::r#enum(["slash_command", "register_repo_root"]),
    })
});
