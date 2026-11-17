// Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.
import type { ShapeTable } from '../tool-shape.ts';

export const SHAPES_5: ShapeTable = {
  Projects: {
    oneOf: [
      {
        object: {
          method: { literal: 'project_info' },
          'notice?': 'string',
          'local_file?': 'string',
          name: 'string',
          description: 'string',
          instructions: 'string',
          docs: {
            array: { object: { path: 'string', created_at: { oneOf: ['string', 'null'] } } },
          },
          'files?': {
            array: {
              object: {
                path: 'string',
                file_kind: 'string',
                created_at: { oneOf: ['string', 'null'] },
              },
            },
          },
          'sync_sources?': {
            array: { object: { type: { oneOf: ['string', 'null'] }, config: { object: {} } } },
          },
          knowledge: { object: { knowledge_size: 'number', max_knowledge_size: 'number' } },
        },
      },
      {
        object: {
          method: { literal: 'project_read' },
          'notice?': 'string',
          path: 'string',
          'file_kind?': 'string',
          'content?': 'string',
          'local_file?': 'string',
          created_at: { oneOf: ['string', 'null'] },
        },
      },
      {
        object: {
          method: { literal: 'project_search' },
          'notice?': 'string',
          rag: 'boolean',
          'hits?': {
            array: { object: { 'name?': 'string', 'doc_uuid?': 'string', 'text?': 'string' } },
          },
          'docs?': { array: 'string' },
        },
      },
      {
        object: {
          method: { literal: 'project_write' },
          'notice?': 'string',
          path: 'string',
          doc_uuid: 'string',
          replaced: 'boolean',
          'present_to_user?': 'boolean',
          'local_path?': 'string',
        },
      },
      {
        object: {
          method: { literal: 'project_delete' },
          'notice?': 'string',
          path: 'string',
          deleted: 'boolean',
        },
      },
      {
        object: {
          method: { literal: 'project_memory_list' },
          'notice?': 'string',
          files: {
            array: {
              object: {
                path: 'string',
                size_bytes: 'number',
                updated_at: { oneOf: ['string', 'null'] },
                truncated: 'boolean',
              },
            },
          },
          truncated: 'boolean',
        },
      },
      {
        object: {
          method: { literal: 'project_memory_read' },
          'notice?': 'string',
          path: 'string',
          'content?': 'string',
          'local_file?': 'string',
          size_bytes: 'number',
          updated_at: { oneOf: ['string', 'null'] },
          truncated: 'boolean',
        },
      },
    ],
  },
};
