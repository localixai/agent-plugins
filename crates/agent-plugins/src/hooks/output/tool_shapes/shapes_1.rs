//! Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.
//!
//! Table 1 of the built-in tools' output shapes, as JSON (`tool_shape::read_table`).

/// The tools of this table by name, each with its output shape.
pub(super) const SHAPES: &str = r##"{
  "Read": {
    "oneOf": [
      {
        "object": {
          "type": {"literal":"text"},
          "file": {
            "object": {
              "filePath": "string",
              "content": "string",
              "numLines": "number",
              "startLine": "number",
              "totalLines": "number",
              "truncatedByTokenCap?": "boolean"
            }
          },
          "artifactRead?": {"object":{"slug":"string","ver":"string"}}
        }
      },
      {
        "object": {
          "type": {"literal":"image"},
          "file": {
            "object": {
              "base64": "string",
              "type": {
                "oneOf": [
                  {"literal":"image/jpeg"},
                  {"literal":"image/png"},
                  {"literal":"image/gif"},
                  {"literal":"image/webp"}
                ]
              },
              "originalSize": "number",
              "dimensions?": {
                "object": {
                  "originalWidth?": "number",
                  "originalHeight?": "number",
                  "displayWidth?": "number",
                  "displayHeight?": "number"
                }
              }
            }
          }
        }
      },
      {
        "object": {
          "type": {"literal":"notebook"},
          "file": {"object":{"filePath":"string","cells":{"array":"unknown"}}}
        }
      },
      {
        "object": {
          "type": {"literal":"pdf"},
          "file": {"object":{"filePath":"string","base64":"string","originalSize":"number"}}
        }
      },
      {
        "object": {
          "type": {"literal":"parts"},
          "file": {
            "object": {
              "filePath": "string",
              "originalSize": "number",
              "count": "number",
              "outputDir": "string"
            }
          },
          "firstPage?": "number",
          "pages?": {
            "array": {
              "object": {
                "base64": "string",
                "mediaType": {
                  "oneOf": [
                    {"literal":"image/jpeg"},
                    {"literal":"image/png"},
                    {"literal":"image/gif"},
                    {"literal":"image/webp"}
                  ]
                },
                "error?": "string"
              }
            }
          }
        }
      }
    ]
  },
  "Write": {
    "object": {
      "type": {"oneOf":[{"literal":"create"},{"literal":"update"}]},
      "filePath": "string",
      "content": "string",
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
      "originalFile": {"oneOf":["string","null"]},
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
      "userModified?": "boolean",
      "staged?": "boolean"
    }
  },
  "Glob": {
    "object": {
      "durationMs": "number",
      "numFiles": "number",
      "filenames": {"array":"string"},
      "truncated": "boolean",
      "totalMatches?": "number",
      "countIsComplete?": "boolean",
      "timedOut?": "boolean"
    }
  },
  "Grep": {
    "object": {
      "mode?": {
        "oneOf": [{"literal":"content"},{"literal":"files_with_matches"},{"literal":"count"}]
      },
      "numFiles": "number",
      "filenames": {"array":"string"},
      "content?": "string",
      "numLines?": "number",
      "numMatches?": "number",
      "totalFiles?": "number",
      "totalLines?": "number",
      "appliedLimit?": "number",
      "appliedOffset?": "number",
      "timedOut?": "boolean"
    }
  },
  "TaskStop": {
    "object": {"message":"string","task_id":"string","task_type":"string","command?":"string"}
  },
  "ListMcpResourcesTool": {
    "array": {
      "object": {
        "uri": "string",
        "name": "string",
        "mimeType?": "string",
        "description?": "string",
        "server": "string"
      }
    }
  },
  "RefreshMcpTools": {
    "array": {
      "object": {
        "server": "string",
        "status": {
          "oneOf": [{"literal":"refreshed"},{"literal":"error"},{"literal":"not_connected"}]
        },
        "toolCount?": "number",
        "added?": {"array":"string"},
        "removed?": {"array":"string"},
        "error?": "string"
      }
    }
  },
  "NotebookEdit": {
    "object": {
      "new_source": "string",
      "old_source?": "string",
      "cell_id?": "string",
      "cell_type": {"oneOf":[{"literal":"code"},{"literal":"markdown"}]},
      "language": "string",
      "edit_mode": "string",
      "error?": "string",
      "notebook_path": "string",
      "original_file": "string",
      "updated_file": "string"
    }
  },
  "ReadMcpResourceDir": {
    "object": {
      "resources": {"array":{"object":{"uri":"string","name":"string","mimeType?":"string"}}},
      "error?": "string"
    }
  },
  "ReadMcpResourceTool": {
    "object": {
      "contents": {
        "array": {
          "object": {"uri":"string","mimeType?":"string","text?":"string","blobSavedTo?":"string"}
        }
      },
      "error?": "string"
    }
  },
  "ReportFindings": {
    "object": {
      "count": "number",
      "level?": {
        "oneOf": [
          {"literal":"low"},
          {"literal":"medium"},
          {"literal":"high"},
          {"literal":"xhigh"},
          {"literal":"max"}
        ]
      },
      "findings": {
        "array": {
          "object": {
            "file": "string",
            "line?": "number",
            "summary": "string",
            "short_summary?": "string",
            "failure_scenario": "string",
            "category?": "string",
            "verdict?": {"oneOf":[{"literal":"CONFIRMED"},{"literal":"PLAUSIBLE"}]},
            "outcome?": {
              "oneOf": [{"literal":"fixed"},{"literal":"skipped"},{"literal":"no_change_needed"}]
            }
          }
        }
      }
    }
  },
  "TodoWrite": {
    "object": {
      "oldTodos": {
        "array": {
          "object": {
            "content": "string",
            "status": {
              "oneOf": [{"literal":"pending"},{"literal":"in_progress"},{"literal":"completed"}]
            },
            "activeForm": "string"
          }
        }
      },
      "newTodos": {
        "array": {
          "object": {
            "content": "string",
            "status": {
              "oneOf": [{"literal":"pending"},{"literal":"in_progress"},{"literal":"completed"}]
            },
            "activeForm": "string"
          }
        }
      }
    }
  }
}"##;
