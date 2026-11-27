//! Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.
//!
//! Table 3 of the built-in tools' output shapes, as JSON (`tool_shape::read_table`).

/// The tools of this table by name, each with its output shape.
pub(super) const SHAPES: &str = r##"{
  "Artifact": {
    "oneOf": [
      {
        "object": {
          "url": "string",
          "path": "string",
          "artifact_id?": "string",
          "title?": "string",
          "version?": "string",
          "capabilities?": "unknown",
          "stored?": {
            "object": {
              "contract": "string",
              "preferredContract?": "string",
              "capabilities?": {"object":{}},
              "carried?": "boolean",
              "read?": "string"
            }
          },
          "warnings?": {"array":"string"},
          "publishesRemaining?": "number",
          "publishesResetAt?": "number",
          "contract?": "string",
          "updated?": "boolean",
          "owned?": "boolean",
          "icon?": "string",
          "faviconSent?": {"literal":true},
          "iconDropped?": {"literal":true},
          "audience?": "string",
          "seq?": "number",
          "unchanged?": {"literal":true},
          "liveSubscription?": "string",
          "pinned?": "boolean"
        }
      },
      {
        "object": {
          "artifacts": {
            "array": {
              "object": {
                "title": "string",
                "url": "string",
                "favicon?": "string",
                "updatedAt?": "string",
                "rel?": {"oneOf":[{"literal":"mine"},{"literal":"shared"}]},
                "external?": {"literal":true},
                "role?": {
                  "oneOf": [
                    {"literal":"editor"},
                    {"literal":"commenter"},
                    {"literal":"reader"},
                    {"literal":"viewer"}
                  ]
                },
                "outsidersMayHaveWritten?": {"literal":true},
                "pinned?": "boolean"
              }
            }
          },
          "truncated?": "boolean",
          "total?": "number",
          "total_at_least?": {"literal":true},
          "next_page_token?": "string",
          "pins_enabled?": "boolean",
          "scope?": {"oneOf":[{"literal":"shared"},{"literal":"all"}]},
          "external_listed?": {"literal":true}
        }
      },
      {
        "object": {
          "read": {
            "object": {
              "url": "string",
              "bytes": "number",
              "code": "number",
              "codeText": "string",
              "result": "string",
              "durationMs": "number",
              "title?": "string"
            }
          },
          "owned?": "boolean",
          "artifactRead?": {"object":{"slug":"string","ver?":"string","seeded?":{"literal":false}}}
        }
      },
      {
        "object": {
          "watch": {
            "object": {
              "url": "string",
              "watching": "boolean",
              "outcome": "string",
              "reason?": "string",
              "durable_skip_reason?": "string",
              "task_id?": "string",
              "since?": "number",
              "token_expires_at?": "number",
              "rail?": "string",
              "trigger_id?": "string",
              "durable_since?": "string",
              "status?": "number",
              "detail?": "string",
              "note?": "string",
              "events?": {"array":"string"}
            }
          }
        }
      },
      {"object":{"unwatch":{"object":{"url":"string","was_watching":"boolean"}}}},
      {
        "object": {
          "watches": {
            "array": {
              "oneOf": [
                {
                  "object": {
                    "url": "string",
                    "task_id": "string",
                    "since": "number",
                    "explicit": "boolean",
                    "connected": "boolean",
                    "connecting?": "boolean",
                    "token_expires_at": "number",
                    "armed_via?": "string"
                  }
                },
                {
                  "object": {
                    "url": "string",
                    "rail": {"literal":"durable_wake"},
                    "trigger_id": "string",
                    "since": "string",
                    "events?": {"array":"string"},
                    "restored?": "boolean"
                  }
                },
                {
                  "object": {
                    "url": "string",
                    "rail": {"literal":"live_stopped"},
                    "since?": "number",
                    "explicit?": "boolean",
                    "armed_via?": "string",
                    "stop_kind": "string"
                  }
                }
              ]
            }
          },
          "filter_url?": "string",
          "arms?": {
            "array": {
              "object": {
                "url": "string",
                "rail?": "string",
                "state": "string",
                "reconnect?": "boolean",
                "failures?": "number",
                "max_failures?": "number",
                "next_in_s?": "number",
                "last_failure?": "string",
                "reason?": "string",
                "detail?": "string",
                "server_message?": "string",
                "at?": "number"
              }
            }
          }
        }
      }
    ]
  }
}"##;
