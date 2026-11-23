// Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.
import type { ShapeTable } from '../tool-shape.ts';

export const SHAPES_4: ShapeTable = {
  Artifact: {
    oneOf: [
      {
        object: {
          'written?': { object: { url: 'string' } },
          'owned?': 'boolean',
          asset_upload: {
            object: {
              id: 'string',
              url: 'string',
              size_bytes: 'number',
              content_type: 'string',
              'sha256?': 'string',
              file_name: 'string',
            },
          },
        },
      },
      {
        object: {
          'written?': { object: { url: 'string' } },
          'owned?': 'boolean',
          asset_uploads: {
            object: {
              url: 'string',
              results: {
                array: {
                  oneOf: [
                    {
                      object: {
                        file_path: 'string',
                        status: { literal: 'uploaded' },
                        id: 'string',
                        url: 'string',
                        size_bytes: 'number',
                        content_type: 'string',
                        'sha256?': 'string',
                        file_name: 'string',
                      },
                    },
                    {
                      object: {
                        file_path: 'string',
                        status: { oneOf: [{ literal: 'failed' }, { literal: 'not_attempted' }] },
                        reason: 'string',
                        message: 'string',
                        'may_be_stored?': { literal: true },
                      },
                    },
                  ],
                },
              },
            },
          },
        },
      },
      {
        object: {
          asset_list: {
            object: {
              url: 'string',
              assets: {
                array: {
                  object: {
                    id: 'string',
                    url: 'string',
                    content_type: 'string',
                    size_bytes: 'number',
                    'sha256?': 'string',
                    created_at: 'string',
                  },
                },
              },
              usage: {
                object: {
                  files: 'number',
                  bytes: 'number',
                  max_files: 'number',
                  max_bytes: 'number',
                },
              },
              'next?': 'string',
              'cowritten?': { literal: true },
              'outside_writer?': { literal: true },
            },
          },
        },
      },
      {
        object: {
          asset_read: {
            object: {
              id: 'string',
              path: 'string',
              size_bytes: 'number',
              content_type: 'string',
              sha256: 'string',
              'cowritten?': { literal: true },
              'outside_writer?': { literal: true },
              'public_read?': { literal: true },
              'foreign?': { literal: true },
            },
          },
        },
      },
      {
        object: {
          'written?': { object: { url: 'string' } },
          'owned?': 'boolean',
          asset_delete: { object: { id: 'string', deleted: 'boolean' } },
        },
      },
      {
        object: {
          pin: {
            object: {
              action: { oneOf: [{ literal: 'pin' }, { literal: 'unpin' }] },
              url: 'string',
              pinned: 'boolean',
              'title?': 'string',
            },
          },
        },
      },
    ],
  },
  RemoteTrigger: { object: { status: 'number', json: 'string', 'summary?': 'string' } },
  ShowOnboardingRolePicker: { object: { 'role?': 'string', 'dismissed?': 'boolean' } },
  OfferChromeSetup: {
    object: {
      outcome: {
        oneOf: [{ literal: 'connected' }, { literal: 'not_now' }, { literal: 'no_attempt_yet' }],
      },
    },
  },
  ReadNotifications: {
    object: {
      notifications: {
        array: {
          object: {
            notification_id: 'string',
            origin: 'string',
            queued_at: 'string',
            content: 'string',
            'arrived_at?': 'string',
          },
        },
      },
      remaining: 'number',
      'read_at?': 'string',
    },
  },
  ScheduleWakeup: {
    object: {
      scheduledFor: 'number',
      clampedDelaySeconds: 'number',
      wasClamped: 'boolean',
      'stopped?': 'boolean',
      'cancelledWakeups?': 'number',
    },
  },
  Monitor: {
    object: {
      taskId: 'string',
      timeoutMs: 'number',
      'persistent?': 'boolean',
      'keptInSandboxByCheck?': 'boolean',
    },
  },
  ProposeSkills: { object: { proposalCount: 'number' } },
  ProposeGoal: { object: { condition: 'string', askUser: 'boolean' } },
  EnterPlanMode: { object: { message: 'string' } },
  Workflow: {
    object: {
      status: { oneOf: [{ literal: 'async_launched' }, { literal: 'remote_launched' }] },
      taskId: 'string',
      'taskType?': { oneOf: [{ literal: 'local_workflow' }, { literal: 'remote_agent' }] },
      'workflowName?': 'string',
      'runId?': 'string',
      'summary?': 'string',
      'transcriptDir?': 'string',
      'scriptPath?': 'string',
      'sessionUrl?': 'string',
      'warning?': 'string',
      'error?': 'string',
    },
  },
  CronCreate: {
    object: {
      id: 'string',
      humanSchedule: 'string',
      recurring: 'boolean',
      'durable?': 'boolean',
      'nextFireAt?': 'number',
      'maxAgeMs?': 'number',
      'otherOneShots?': {
        array: { object: { id: 'string', humanSchedule: 'string', prompt: 'string' } },
      },
    },
  },
  CronDelete: { object: { id: 'string', 'endedLoopWakeup?': { literal: true } } },
  CronList: {
    object: {
      jobs: {
        array: {
          object: {
            id: 'string',
            cron: 'string',
            humanSchedule: 'string',
            prompt: 'string',
            'recurring?': 'boolean',
            'durable?': 'boolean',
            'nextFireAt?': 'number',
          },
        },
      },
    },
  },
  PushNotification: {
    object: {
      message: 'string',
      'pushSent?': 'boolean',
      'localSent?': 'boolean',
      'disabledReason?': {
        oneOf: [
          { literal: 'config_off' },
          { literal: 'user_present' },
          { literal: 'no_transport' },
        ],
      },
      'sentAt?': 'string',
    },
  },
  ClaudeDesign: {
    object: { operation: 'string', content: { array: { object: {} } }, 'isError?': 'boolean' },
  },
};
