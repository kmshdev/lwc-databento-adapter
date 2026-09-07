import { readdir, readFile } from 'node:fs/promises';
import { describe, expect, it } from 'vitest';
import { Ajv2020 } from 'ajv/dist/2020.js';

interface Fixture {
  valid: boolean;
  direction: 'http-response' | 'client-command' | 'server-event';
  payload: unknown;
}

interface SchemaDocument {
  roots: Record<string, unknown>;
  $defs: Record<string, unknown>;
}

const contractsUrl = new URL('../../../../contracts/', import.meta.url);

const loadJson = async <T>(url: URL): Promise<T> => JSON.parse(await readFile(url, 'utf8')) as T;

const schemaDocument = await loadJson<SchemaDocument>(
  new URL('protocol-v1.schema.json', contractsUrl),
);

const ajv = new Ajv2020({ strict: false, validateFormats: false });
const validatorFor = (root: string) =>
  ajv.compile({
    $ref: `#/$defs/${root}`,
    $defs: schemaDocument.$defs,
  });

const rootsByDirection: Record<Fixture['direction'], readonly string[]> = {
  'client-command': ['ClientCommand'],
  'server-event': ['ServerEvent'],
  'http-response': [
    'BarPageResponse',
    'ResolveResponse',
    'SearchResponse',
    'DatasetResponse',
    'ErrorResponse',
  ],
};

const validators = new Map(
  Object.values(rootsByDirection)
    .flat()
    .map((root) => [root, validatorFor(root)] as const),
);

const validFixturePaths = async (): Promise<string[]> => {
  const paths: string[] = [];
  for (const transport of ['http', 'websocket']) {
    const dir = new URL(`fixtures/${transport}/valid/`, contractsUrl);
    for (const name of await readdir(dir)) {
      paths.push(`fixtures/${transport}/valid/${name}`);
    }
  }
  return paths;
};

describe('protocol v1 generated JSON Schema conformance', () => {
  it('declares every root the fixture directions require', () => {
    for (const root of Object.values(rootsByDirection).flat()) {
      expect(schemaDocument.roots).toHaveProperty(root);
    }
  });

  it('accepts every valid fixture under its direction roots', async () => {
    const paths = await validFixturePaths();
    expect(paths.length).toBeGreaterThan(0);
    for (const path of paths) {
      const fixture = await loadJson<Fixture>(new URL(path, contractsUrl));
      expect(fixture.valid).toBe(true);
      const accepted = rootsByDirection[fixture.direction].some((root) =>
        validators.get(root)?.(fixture.payload),
      );
      expect(accepted, `${path} must satisfy a ${fixture.direction} schema root`).toBe(true);
    }
  });

  it('rejects a structurally unknown command field', () => {
    const validate = validators.get('ClientCommand');
    expect(
      validate?.({
        v: 1,
        type: 'unsubscribe',
        commandId: 'cmd-1',
        subscriptionId: 'sub-1',
        apiKey: 'db-secret',
      }),
    ).toBe(false);
  });
});
