//! Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.
//!
//! Table 0 of the built-in tools' output shapes, as JSON (`tool_shape::read_table`).

/// The tools of this table by name, each with its output shape.
pub(super) const SHAPES: &str = r##"{
  "Agent": {
    "oneOf": [
      {
        "object": {
          "agentId": "string",
          "agentType?": "string",
          "content": {
            "array": {
              "object": {
                "type": {"literal":"text"},
                "text": "string",
                "citations?": {"oneOf":[{"array":"unknown"},"null"]}
              }
            }
          },
          "resolvedModel?": "string",
          "modelsUsed?": {"array":"string"},
          "totalToolUseCount": "number",
          "totalDurationMs": "number",
          "totalTokens": "number",
          "usage": {
            "object": {
              "input_tokens": "number",
              "output_tokens": "number",
              "cache_creation_input_tokens": {"oneOf":["number","null"]},
              "cache_read_input_tokens": {"oneOf":["number","null"]},
              "server_tool_use": {
                "oneOf": [
                  {"object":{"web_search_requests":"number","web_fetch_requests":"number"}},
                  "null"
                ]
              },
              "service_tier": {"oneOf":["string","null"]},
              "cache_creation": {
                "oneOf": [
                  {
                    "object": {
                      "ephemeral_1h_input_tokens": "number",
                      "ephemeral_5m_input_tokens": "number"
                    }
                  },
                  "null"
                ]
              },
              "inference_geo?": {"oneOf":["string","null"]},
              "speed?": {"oneOf":["string","null"]},
              "iterations?": "unknown",
              "output_tokens_details?": {
                "oneOf": [{"object":{"thinking_tokens?":{"oneOf":["number","null"]}}},"null"]
              },
              "fallback_credit?": "unknown"
            }
          },
          "toolsUsed?": {"array":"string"},
          "toolStats?": {
            "object": {
              "readCount": "number",
              "searchCount": "number",
              "bashCount": "number",
              "editFileCount": "number",
              "linesAdded": "number",
              "linesRemoved": "number",
              "otherToolCount": "number",
              "frameCount?": "number"
            }
          },
          "status": {"literal":"completed"},
          "prompt": "string",
          "worktreePath?": "string",
          "worktreeBranch?": "string"
        }
      },
      {
        "object": {
          "status": {"literal":"async_launched"},
          "isAsync?": {"literal":true},
          "agentId": "string",
          "description": "string",
          "resolvedModel?": "string",
          "modelsUsed?": {"array":"string"},
          "prompt": "string",
          "outputFile": "string",
          "canReadOutputFile?": "boolean"
        }
      },
      {
        "object": {
          "status": {"literal":"remote_launched"},
          "taskId": "string",
          "sessionUrl": "string",
          "description": "string",
          "prompt": "string",
          "outputFile": "string"
        }
      }
    ]
  },
  "Bash": {
    "object": {
      "stdout": "string",
      "stderr": "string",
      "rawOutputPath?": "string",
      "interrupted": "boolean",
      "isImage?": "boolean",
      "backgroundTaskId?": "string",
      "backgroundedByUser?": "boolean",
      "timedOutAfterMs?": "number",
      "backgroundCwdHint?": "string",
      "backgroundEndsWithFinalResponse?": {"literal":true},
      "backgroundWakesOwner?": {"literal":true},
      "dangerouslyDisableSandbox?": "boolean",
      "returnCodeInterpretation?": "string",
      "noOutputExpected?": "boolean",
      "structuredContent?": {"array":"unknown"},
      "persistedOutputPath?": "string",
      "persistedOutputSize?": "number",
      "staleReadFileStateHint?": "string",
      "ghRateLimitHint?": "string",
      "gitOperation?": {
        "object": {
          "commit?": {
            "object": {
              "sha": "string",
              "kind": {
                "oneOf": [{"literal":"committed"},{"literal":"amended"},{"literal":"cherry-picked"}]
              },
              "branch?": "string"
            }
          },
          "push?": {"object":{"branch":"string"}},
          "branch?": {
            "object": {
              "ref": "string",
              "action": {"oneOf":[{"literal":"merged"},{"literal":"rebased"}]}
            }
          },
          "pr?": {
            "object": {
              "number": "number",
              "url?": "string",
              "action": {
                "oneOf": [
                  {"literal":"created"},
                  {"literal":"edited"},
                  {"literal":"merged"},
                  {"literal":"commented"},
                  {"literal":"closed"},
                  {"literal":"reopened"},
                  {"literal":"ready"},
                  {"literal":"draft"},
                  {"literal":"auto-merge-enabled"},
                  {"literal":"auto-merge-disabled"}
                ]
              }
            }
          }
        }
      }
    }
  },
  "ExitPlanMode": {
    "object": {
      "plan": {"oneOf":["string","null"]},
      "isAgent": "boolean",
      "filePath?": "string",
      "hasTaskTool?": "boolean",
      "planWasEdited?": "boolean",
      "awaitingLeaderApproval?": "boolean",
      "requestId?": "string"
    }
  },
  "Edit": {
    "object": {
      "filePath": "string",
      "oldString": "string",
      "newString": "string",
      "originalFile": {"oneOf":["string","null"]},
      "structuredPatch": {
        "array": {
          "object": {
            "oldStart": "number",
            "oldLines": "number",
            "newStart": "number",
            "newLines": "number",
            "lines": {"array":"string"}
          }
        }
      },
      "userModified": "boolean",
      "replaceAll": "boolean",
      "gitDiff?": {
        "object": {
          "filename": "string",
          "status": {"oneOf":[{"literal":"modified"},{"literal":"added"}]},
          "additions": "number",
          "deletions": "number",
          "changes": "number",
          "patch": "string",
          "repository?": {"oneOf":["string","null"]}
        }
      },
      "staged?": "boolean"
    }
  }
}"##;
