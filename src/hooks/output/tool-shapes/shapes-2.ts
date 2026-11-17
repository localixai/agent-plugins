// Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.
import type { ShapeTable } from '../tool-shape.ts';

export const SHAPES_2: ShapeTable = {
  TodoWrite: {
    object: {
      oldTodos: {
        array: {
          object: {
            content: 'string',
            status: {
              oneOf: [{ literal: 'pending' }, { literal: 'in_progress' }, { literal: 'completed' }],
            },
            activeForm: 'string',
          },
        },
      },
      newTodos: {
        array: {
          object: {
            content: 'string',
            status: {
              oneOf: [{ literal: 'pending' }, { literal: 'in_progress' }, { literal: 'completed' }],
            },
            activeForm: 'string',
          },
        },
      },
    },
  },
  WebFetch: {
    object: {
      bytes: 'number',
      code: 'number',
      codeText: 'string',
      result: 'string',
      durationMs: 'number',
      url: 'string',
      'artifactRead?': {
        object: { slug: 'string', 'ver?': 'string', 'seeded?': { literal: false } },
      },
    },
  },
  WebSearch: {
    object: {
      query: 'string',
      results: {
        array: {
          oneOf: [
            {
              object: {
                tool_use_id: 'string',
                content: {
                  array: { object: { title: 'string', url: 'string', 'snippet?': 'string' } },
                },
              },
            },
            'string',
          ],
        },
      },
      durationSeconds: 'number',
      'searchCount?': 'number',
    },
  },
  AskUserQuestion: {
    object: {
      questions: {
        array: {
          object: {
            question: 'string',
            header: 'string',
            options: {
              oneOf: [
                {
                  array: {
                    object: { label: 'string', description: 'string', 'preview?': 'string' },
                  },
                },
                {
                  array: {
                    object: { label: 'string', description: 'string', 'preview?': 'string' },
                  },
                },
                {
                  array: {
                    object: { label: 'string', description: 'string', 'preview?': 'string' },
                  },
                },
              ],
            },
            multiSelect: 'boolean',
          },
        },
      },
      answers: { object: {} },
      'response?': 'string',
      'annotations?': { object: {} },
      'afkTimeoutMs?': 'number',
    },
  },
  SendFeedback: { object: { success: 'boolean', message: 'string' } },
  EnterWorktree: {
    object: { worktreePath: 'string', 'worktreeBranch?': 'string', message: 'string' },
  },
  ExitWorktree: {
    object: {
      action: { oneOf: [{ literal: 'keep' }, { literal: 'remove' }] },
      originalCwd: 'string',
      worktreePath: 'string',
      'worktreeBranch?': 'string',
      'tmuxSessionName?': 'string',
      'discardedFiles?': 'number',
      'discardedCommits?': 'number',
      message: 'string',
    },
  },
  TaskCreate: {
    object: { task: { object: { id: 'string', subject: 'string', 'description?': 'string' } } },
  },
  TaskGet: {
    object: {
      task: {
        oneOf: [
          {
            object: {
              id: 'string',
              subject: 'string',
              description: 'string',
              status: {
                oneOf: [
                  { literal: 'pending' },
                  { literal: 'in_progress' },
                  { literal: 'completed' },
                ],
              },
              blocks: { array: 'string' },
              blockedBy: { array: 'string' },
            },
          },
          'null',
        ],
      },
    },
  },
  TaskUpdate: {
    object: {
      success: 'boolean',
      taskId: 'string',
      updatedFields: { array: 'string' },
      'error?': 'string',
      'statusChange?': { object: { from: 'string', to: 'string' } },
    },
  },
  TaskList: {
    object: {
      tasks: {
        array: {
          object: {
            id: 'string',
            subject: 'string',
            status: {
              oneOf: [{ literal: 'pending' }, { literal: 'in_progress' }, { literal: 'completed' }],
            },
            'owner?': 'string',
            blockedBy: { array: 'string' },
          },
        },
      },
    },
  },
};
