import { readFile } from 'node:fs/promises';
import { describe, expect, it } from 'vitest';
import {
  barPageResponseSchema,
  clientCommandSchema,
  errorResponseSchema,
  serverEventSchema,
} from '../../src/client/schemas.js';

const fixture = async (
  path: string,
): Promise<{ valid: boolean; direction: string; payload: unknown }> =>
  JSON.parse(await readFile(new URL(path, import.meta.url), 'utf8')) as {
    valid: boolean;
    direction: string;
    payload: unknown;
  };

const cases = [
  ['../../../../contracts/fixtures/http/valid/history-response.json', barPageResponseSchema, true],
  ['../../../../contracts/fixtures/http/valid/error-response.json', errorResponseSchema, true],
  ['../../../../contracts/fixtures/http/invalid/unsafe-time.json', barPageResponseSchema, false],
  [
    '../../../../contracts/fixtures/http/invalid/credential-field.json',
    barPageResponseSchema,
    false,
  ],
  ['../../../../contracts/fixtures/websocket/valid/open-bars.json', clientCommandSchema, true],
  ['../../../../contracts/fixtures/websocket/valid/subscribed.json', serverEventSchema, true],
  ['../../../../contracts/fixtures/websocket/valid/snapshot.json', serverEventSchema, true],
  ['../../../../contracts/fixtures/websocket/valid/bar.json', serverEventSchema, true],
  ['../../../../contracts/fixtures/websocket/valid/cancelled.json', serverEventSchema, true],
  [
    '../../../../contracts/fixtures/websocket/valid/resolved-instrument-changed.json',
    serverEventSchema,
    true,
  ],
  ['../../../../contracts/fixtures/websocket/invalid/unknown-event.json', serverEventSchema, false],
  [
    '../../../../contracts/fixtures/websocket/invalid/mismatched-volume.json',
    serverEventSchema,
    false,
  ],
  [
    '../../../../contracts/fixtures/websocket/invalid/unknown-command-field.json',
    clientCommandSchema,
    false,
  ],
] as const;

describe('protocol v1 fixtures', () => {
  for (const [path, schema, valid] of cases) {
    it(`${valid ? 'accepts' : 'rejects'} ${path}`, async () => {
      const value = await fixture(path);
      expect(schema.safeParse(value.payload).success).toBe(valid);
      expect(value.valid).toBe(valid);
    });
  }
});
