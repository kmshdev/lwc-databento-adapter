import assert from 'node:assert/strict';
import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import puppeteer from 'puppeteer';
import { WebSocketServer } from 'ws';

const testDirectory = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = join(testDirectory, '../../../..');
const demoRoot = join(workspaceRoot, 'examples/lightweight-charts-demo');

const metadata = () => ({
  dataset: 'GLBX.MDP3',
  requestedSymbol: '123',
  resolvedSymbol: 'ESZ4',
  instrumentId: 123,
  sourceSchema: 'ohlcv-1m',
  synthetic: false,
});

const pageFor = (from: number, to: number) => {
  const end = Math.min(to, from + 180);
  const times: number[] = [];
  for (let time = from; time < end; time += 60) times.push(time);
  return {
    bars: times.map((time, index) =>
      index === 1
        ? { time }
        : {
            time,
            open: 100 + index,
            high: 102 + index,
            low: 99 + index,
            close: 101 + index,
          },
    ),
    volumes: times.map((time, index) => ({ time, value: 40 + index })),
    metadata: times.map((time) => ({ time, ...metadata() })),
  };
};

const readJson = async (request: IncomingMessage): Promise<Record<string, unknown>> => {
  const chunks: Buffer[] = [];
  for await (const chunk of request) chunks.push(Buffer.from(chunk));
  return JSON.parse(Buffer.concat(chunks).toString('utf8')) as Record<string, unknown>;
};

const sendJson = (response: ServerResponse, value: unknown, origin: string): void => {
  response.writeHead(200, {
    'content-type': 'application/json',
    'access-control-allow-origin': origin,
  });
  response.end(JSON.stringify(value));
};

async function availablePort(): Promise<number> {
  const probe = createServer();
  await new Promise<void>((resolve) => probe.listen(0, '127.0.0.1', resolve));
  const address = probe.address();
  assert.ok(address && typeof address === 'object');
  await new Promise<void>((resolve, reject) =>
    probe.close((error) => (error ? reject(error) : resolve())),
  );
  return address.port;
}

async function waitForUrl(url: string, process?: ChildProcess): Promise<void> {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (process?.exitCode !== null || process.signalCode !== null)
      throw new Error(`Process exited before ${url} became ready`);
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The bounded retry loop is the startup synchronization boundary.
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function run(): Promise<void> {
  assert.ok(
    existsSync(join(workspaceRoot, '.puppeteerrc.cjs')),
    'Puppeteer configuration is required at the workspace root.',
  );

  let historyRequestCount = 0;
  const historyRequests: Array<Record<string, unknown>> = [];
  let activeHistoryRequests = 0;
  let maxConcurrentHistoryRequests = 0;
  const demoPort = await availablePort();
  const baseUrl = `http://127.0.0.1:${demoPort}`;
  const gateway = createServer(async (request, response) => {
    if (request.method === 'OPTIONS') {
      response.writeHead(204, {
        'access-control-allow-origin': baseUrl,
        'access-control-allow-headers': 'content-type,x-request-id',
        'access-control-allow-methods': 'GET,POST,OPTIONS',
      });
      response.end();
      return;
    }
    if (request.url === '/health/live' || request.url === '/health/ready')
      return sendJson(response, { status: 'ok' }, baseUrl);
    if (request.url === '/v1/history/bars') {
      const body = await readJson(request);
      historyRequests.push(body);
      historyRequestCount += 1;
      activeHistoryRequests += 1;
      maxConcurrentHistoryRequests = Math.max(maxConcurrentHistoryRequests, activeHistoryRequests);
      await new Promise((resolve) => setTimeout(resolve, 40));
      const page =
        historyRequestCount % 2 === 0
          ? { bars: [], volumes: [], metadata: [] }
          : pageFor(Number(body.from), Number(body.to));
      sendJson(
        response,
        {
          v: 1,
          requestId: 'e2e-history',
          ...page,
        },
        baseUrl,
      );
      activeHistoryRequests -= 1;
      return;
    }
    if (request.url === '/v1/symbols/resolve') {
      return sendJson(
        response,
        {
          v: 1,
          requestId: 'e2e-resolve',
          mappings: [
            {
              dataset: 'GLBX.MDP3',
              requestedSymbol: 'ES.FUT',
              resolvedSymbol: 'ESZ4',
              instrumentId: 123,
              effectiveFrom: 1_700_000_000,
            },
            {
              dataset: 'GLBX.MDP3',
              requestedSymbol: 'ES.FUT',
              resolvedSymbol: 'ESH5',
              instrumentId: 124,
              effectiveFrom: 1_700_000_000,
            },
          ],
        },
        baseUrl,
      );
    }
    response.writeHead(404).end();
  });
  let openBarsCount = 0;
  let openBarsTo: number | undefined;
  let unsubscribeCount = 0;
  const sockets = new WebSocketServer({
    server: gateway,
    handleProtocols: (protocols) =>
      protocols.has('databento-lwc.v1') ? 'databento-lwc.v1' : false,
  });
  sockets.on('connection', (socket) => {
    socket.on('message', (raw) => {
      const command = JSON.parse(raw.toString()) as {
        type: string;
        commandId: string;
        subscriptionId: string;
        targetCommandId?: string;
        request?: { to?: number };
      };
      if (command.type === 'unsubscribe') {
        unsubscribeCount += 1;
        socket.send(
          JSON.stringify({
            v: 1,
            type: 'unsubscribed',
            commandId: command.commandId,
            subscriptionId: command.subscriptionId,
          }),
        );
        return;
      }
      if (command.type === 'cancel') {
        socket.send(
          JSON.stringify({
            v: 1,
            type: 'cancelled',
            commandId: command.commandId,
            targetCommandId: command.targetCommandId,
            subscriptionId: command.subscriptionId,
          }),
        );
        return;
      }
      const time = command.request?.to ?? Math.floor(Date.now() / 60) * 60;
      socket.send(
        JSON.stringify({
          v: 1,
          type: 'subscribed',
          commandId: command.commandId,
          subscriptionId: command.subscriptionId,
          state: command.type === 'open_bars' ? 'replaying' : 'live',
          resolvedSymbols: [
            {
              dataset: 'GLBX.MDP3',
              requestedSymbol: '123',
              resolvedSymbol: 'ESZ4',
              instrumentId: 123,
              effectiveFrom: time - 60,
            },
          ],
        }),
      );
      if (command.type === 'open_bars') {
        openBarsCount += 1;
        openBarsTo = command.request?.to;
        socket.send(
          JSON.stringify({
            v: 1,
            type: 'snapshot',
            subscriptionId: command.subscriptionId,
            ...pageFor(time - 120, time),
          }),
        );
        socket.send(
          JSON.stringify({
            v: 1,
            type: 'status',
            subscriptionId: command.subscriptionId,
            state: 'live',
            retryable: false,
            reason: 'replay_completed',
          }),
        );
        socket.send(
          JSON.stringify({
            v: 1,
            type: 'bar',
            subscriptionId: command.subscriptionId,
            data: { time, open: 103, high: 105, low: 102, close: 104 },
            volume: { time, value: 50 },
            meta: metadata(),
          }),
        );
      }
    });
  });
  await new Promise<void>((resolve) => gateway.listen(0, '127.0.0.1', resolve));
  const gatewayAddress = gateway.address();
  assert.ok(gatewayAddress && typeof gatewayAddress === 'object');
  const gatewayUrl = `http://127.0.0.1:${gatewayAddress.port}`;

  const vite = spawn(
    'pnpm',
    ['exec', 'vite', '--host', '127.0.0.1', '--port', String(demoPort), '--strictPort'],
    {
      cwd: demoRoot,
      env: { ...process.env, VITE_E2E_EXPOSE_APP: '1', VITE_GATEWAY_URL: gatewayUrl },
      stdio: 'ignore',
    },
  );
  await waitForUrl(baseUrl, vite);
  const browser = await puppeteer.launch({ headless: true });
  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 900, deviceScaleFactor: 1 });
    await page.goto(baseUrl, { waitUntil: 'networkidle0' });

    for (const selector of [
      '#history-form',
      '#chart',
      '#notice',
      '#go-live',
      '#disconnect',
      '#fit-content',
      '#toggle-volume',
    ])
      assert.notEqual(await page.$(selector), null, `${selector} should exist`);
    await page.focus('#chart');
    assert.equal(await page.$eval('#chart', (node) => document.activeElement === node), true);

    await page.click('button[type="submit"]');
    await page.waitForFunction(
      () => !(document.querySelector('button[type="submit"]') as HTMLButtonElement).disabled,
    );
    assert.ok(historyRequestCount >= 1);
    await page.waitForFunction(
      () =>
        (
          window.lwcDatabentoDemo as unknown as { bars?: Array<Record<string, unknown>> }
        ).bars?.some((bar) => !('open' in bar)) === true,
    );
    await page.waitForFunction(
      () => (window.lwcDatabentoDemo as unknown as { pageLoading?: boolean }).pageLoading === false,
    );

    const requestsBeforePaging = historyRequestCount;
    const activeHistoryRequest = historyRequests.at(-1);
    await page.$eval('#dataset', (node) => {
      (node as HTMLInputElement).value = 'DRAFT.DATASET';
    });
    await page.$eval('#symbol', (node) => {
      (node as HTMLInputElement).value = 'DRAFT.SYMBOL';
    });
    await page.select('#resolution', '5m');
    await page.evaluate(() => {
      const app = window.lwcDatabentoDemo as unknown as {
        historyExhausted: boolean;
        loadOlderIfNeeded(range: { from: number; to: number }): Promise<void>;
        chart: {
          timeScale(): {
            getVisibleLogicalRange(): { from: number; to: number } | null;
            setVisibleLogicalRange(range: { from: number; to: number }): void;
          };
        };
      };
      app.historyExhausted = false;
      app.chart.timeScale().setVisibleLogicalRange({ from: -1, to: 2 });
      const range = app.chart.timeScale().getVisibleLogicalRange();
      const to = Math.max(range?.to ?? 2, 2);
      for (let index = 0; index < 3; index += 1) void app.loadOlderIfNeeded({ from: 0, to });
    });
    await new Promise((resolve) => setTimeout(resolve, 250));
    assert.ok(historyRequestCount >= requestsBeforePaging + 1, 'paging should request older data');
    assert.equal(maxConcurrentHistoryRequests, 1, 'paging must keep one request in flight');
    assert.deepEqual(
      {
        dataset: historyRequests.at(-1)?.dataset,
        symbol: historyRequests.at(-1)?.symbol,
        stypeIn: historyRequests.at(-1)?.stypeIn,
        resolution: historyRequests.at(-1)?.resolution,
      },
      {
        dataset: activeHistoryRequest?.dataset,
        symbol: activeHistoryRequest?.symbol,
        stypeIn: activeHistoryRequest?.stypeIn,
        resolution: activeHistoryRequest?.resolution,
      },
      'paging must retain the request that produced the displayed chart',
    );
    await page.click('#fit-content');

    await page.click('#toggle-volume');
    assert.equal(
      await page.$eval('#toggle-volume', (node) => node.getAttribute('aria-pressed')),
      'false',
    );
    await page.click('#toggle-volume');
    assert.equal(
      await page.$eval('#toggle-volume', (node) => node.getAttribute('aria-pressed')),
      'true',
    );

    await page.select('#stype', 'parent');
    await page.click('button[type="submit"]');
    await page.waitForFunction(
      () => !(document.querySelector('#parent-picker') as HTMLElement).hidden,
    );
    assert.equal(await page.$$eval('#child-symbol option', (nodes) => nodes.length), 2);
    await page.click('#use-child');
    await page.waitForFunction(() => {
      const stype = document.querySelector('#stype') as HTMLSelectElement;
      const goLive = document.querySelector('#go-live') as HTMLButtonElement;
      return stype.value === 'instrument_id' && !goLive.disabled;
    });

    await page.click('#go-live');
    await page.waitForFunction(
      () => !(document.querySelector('#go-live') as HTMLButtonElement).disabled,
    );
    assert.equal(openBarsCount, 1);
    assert.ok(openBarsTo !== undefined && openBarsTo < 10_000_000_000);
    assert.equal(await page.$eval('#error', (node) => (node as HTMLElement).hidden), true);

    await page.click('#disconnect');
    await new Promise((resolve) => setTimeout(resolve, 1_000));
    const disconnectNotice = await page.$eval('#notice', (node) => node.textContent ?? '');
    assert.equal(unsubscribeCount, 1);
    assert.equal(
      await page.evaluate(
        () => (window.lwcDatabentoDemo as unknown as { disconnected: boolean }).disconnected,
      ),
      true,
    );
    assert.match(disconnectNotice, /Disconnected/, disconnectNotice);
    assert.ok((await page.$$('#chart canvas')).length > 0, 'Disconnect must preserve the chart.');
    await page.waitForFunction(
      () =>
        (window.lwcDatabentoDemo as unknown as { subscription?: unknown }).subscription ===
        undefined,
    );
    const barsBeforeLateHistory = await page.evaluate(() =>
      JSON.stringify(
        (window.lwcDatabentoDemo as unknown as { bars: Array<Record<string, unknown>> }).bars,
      ),
    );
    await page.evaluate(() => {
      const app = window.lwcDatabentoDemo as unknown as {
        loadHistory(): Promise<void>;
      };
      void app.loadHistory();
    });
    await new Promise((resolve) => setTimeout(resolve, 10));
    await page.evaluate(async () => {
      const app = window.lwcDatabentoDemo as unknown as { disconnect(): Promise<void> };
      await app.disconnect();
    });
    await new Promise((resolve) => setTimeout(resolve, 150));
    const lateHistoryResult = await page.evaluate((barsBefore) => {
      const app = window.lwcDatabentoDemo as unknown as {
        bars: Array<Record<string, unknown>>;
        disconnected: boolean;
      };
      return {
        barsUnchanged: JSON.stringify(app.bars) === barsBefore,
        disconnected: app.disconnected,
      };
    }, barsBeforeLateHistory);
    assert.deepEqual(lateHistoryResult, { barsUnchanged: true, disconnected: true });
    assert.match(await page.$eval('#notice', (node) => node.textContent ?? ''), /Disconnected/);

    const source = await readFile(join(demoRoot, 'src/demo-app.ts'), 'utf8');
    assert.ok(!source.includes(['lightweight', 'chart', 'react'].join('-')));
    assert.ok(!source.includes(['src', 'model'].join('/')));
    assert.ok(!source.includes(['DATABENTO', 'API', 'KEY'].join('_')));
  } finally {
    await browser.close();
    vite.kill('SIGTERM');
    sockets.close();
    await new Promise<void>((resolve, reject) =>
      gateway.close((error) => (error ? reject(error) : resolve())),
    );
  }
}

void run();
