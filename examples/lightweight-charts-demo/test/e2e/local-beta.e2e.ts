import assert from 'node:assert/strict';
import { spawn, type ChildProcess } from 'node:child_process';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import puppeteer from 'puppeteer';

const testDirectory = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = join(testDirectory, '../../../..');
const demoRoot = join(workspaceRoot, 'examples/lightweight-charts-demo');
const configuredGatewayUrl = process.env.E2E_GATEWAY_URL;
const configuredBaseUrl = process.env.E2E_BASE_URL;

async function isReady(url: string): Promise<boolean> {
  try {
    return (await fetch(url)).ok;
  } catch {
    return false;
  }
}

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
  for (let attempt = 0; attempt < 120; attempt += 1) {
    if (process?.exitCode !== null || process.signalCode !== null)
      throw new Error(`Process exited before ${url} became ready`);
    if (await isReady(url)) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function stop(process: ChildProcess | undefined): Promise<void> {
  if (process === undefined || process.exitCode !== null) return;
  process.kill('SIGTERM');
  await Promise.race([once(process, 'exit'), new Promise((resolve) => setTimeout(resolve, 5_000))]);
  if (process.exitCode === null) process.kill('SIGKILL');
}

async function run(): Promise<void> {
  let gateway: ChildProcess | undefined;
  let vite: ChildProcess | undefined;
  const gatewayPort = configuredGatewayUrl === undefined ? await availablePort() : undefined;
  const gatewayUrl = configuredGatewayUrl ?? `http://127.0.0.1:${gatewayPort}`;
  const demoPort = configuredBaseUrl === undefined ? await availablePort() : undefined;
  const baseUrl = configuredBaseUrl ?? `http://127.0.0.1:${demoPort}`;

  if (configuredGatewayUrl === undefined) {
    gateway = spawn('cargo', ['run', '-p', 'databento-gateway'], {
      cwd: workspaceRoot,
      env: {
        ...process.env,
        DATABENTO_LWC_ALLOWED_ORIGINS: baseUrl,
        DATABENTO_LWC_BIND_ADDR: `127.0.0.1:${gatewayPort}`,
      },
      stdio: 'ignore',
    });
    await waitForUrl(`${gatewayUrl}/health/ready`, gateway);
  } else {
    await waitForUrl(`${gatewayUrl}/health/ready`);
  }
  if (configuredBaseUrl === undefined) {
    vite = spawn(
      'pnpm',
      ['exec', 'vite', '--host', '127.0.0.1', '--port', String(demoPort), '--strictPort'],
      {
        cwd: demoRoot,
        env: {
          ...process.env,
          VITE_E2E_EXPOSE_APP: '1',
          VITE_GATEWAY_URL: gatewayUrl,
        },
        stdio: 'ignore',
      },
    );
    await waitForUrl(baseUrl, vite);
  } else {
    await waitForUrl(baseUrl);
  }

  const browser = await puppeteer.launch({ headless: true });
  try {
    assert.equal(await isReady(`${gatewayUrl}/health/live`), true, 'gateway liveness must pass');
    assert.equal(await isReady(`${gatewayUrl}/health/ready`), true, 'gateway readiness must pass');

    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 900, deviceScaleFactor: 1 });
    await page.goto(baseUrl, { waitUntil: 'networkidle0' });

    await page.select('#stype', 'parent');
    await page.click('button[type="submit"]');
    await page.waitForFunction(
      () => !(document.querySelector('#parent-picker') as HTMLElement).hidden,
    );
    assert.ok((await page.$$eval('#child-symbol option', (nodes) => nodes.length)) > 0);
    await page.click('#use-child');
    await page.waitForFunction(
      () => !(document.querySelector('button[type="submit"]') as HTMLButtonElement).disabled,
    );
    const historyError = await page.$eval('#error', (node) => ({
      hidden: (node as HTMLElement).hidden,
      text: node.textContent ?? '',
    }));
    assert.equal(historyError.hidden, true, historyError.text);
    assert.ok((await page.$$('#chart canvas')).length >= 2, 'Candles and volume must render.');

    await page.click('#go-live');
    await page.waitForFunction(
      () => !(document.querySelector('#go-live') as HTMLButtonElement).disabled,
    );
    const liveError = await page.$eval('#error', (node) => ({
      hidden: (node as HTMLElement).hidden,
      text: node.textContent ?? '',
    }));
    assert.equal(liveError.hidden, true, liveError.text);
    assert.ok(
      (await page.$$('#chart canvas')).length >= 2,
      'Go live must retain a rendered chart.',
    );

    await page.click('#disconnect');
    await page.waitForFunction(() =>
      (document.querySelector('#notice')?.textContent ?? '').includes('Disconnected'),
    );
    assert.ok(
      (await page.$$('#chart canvas')).length >= 2,
      'Disconnect must preserve chart panes.',
    );
  } finally {
    await browser.close();
    await stop(vite);
    await stop(gateway);
  }
}

void run();
