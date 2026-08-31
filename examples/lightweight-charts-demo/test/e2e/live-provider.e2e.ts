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
  for (let attempt = 0; attempt < 240; attempt += 1) {
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
  assert.ok(process.env.DATABENTO_API_KEY, 'DATABENTO_API_KEY is required for the live test');
  let gateway: ChildProcess | undefined;
  let vite: ChildProcess | undefined;
  const gatewayPort = configuredGatewayUrl === undefined ? await availablePort() : undefined;
  const gatewayUrl = configuredGatewayUrl ?? `http://127.0.0.1:${gatewayPort}`;
  const demoPort = configuredBaseUrl === undefined ? await availablePort() : undefined;
  const baseUrl = configuredBaseUrl ?? `http://127.0.0.1:${demoPort}`;

  if (configuredGatewayUrl === undefined) {
    gateway = spawn('cargo', ['run', '-p', 'databento-gateway', '--features', 'databento-compat'], {
      cwd: workspaceRoot,
      env: {
        ...process.env,
        DATABENTO_GATEWAY_SOURCE: 'historical',
        DATABENTO_LWC_ALLOWED_ORIGINS: baseUrl,
        DATABENTO_LWC_BIND_ADDR: `127.0.0.1:${gatewayPort}`,
      },
      stdio: 'inherit',
    });
    await waitForUrl(`${gatewayUrl}/health/ready`, gateway);
  } else {
    await waitForUrl(`${gatewayUrl}/health/ready`);
  }
  if (configuredBaseUrl === undefined) {
    const demoEnvironment = { ...process.env };
    delete demoEnvironment.DATABENTO_API_KEY;
    demoEnvironment.VITE_E2E_EXPOSE_APP = '1';
    demoEnvironment.VITE_GATEWAY_URL = gatewayUrl;
    vite = spawn(
      'pnpm',
      ['exec', 'vite', '--host', '127.0.0.1', '--port', String(demoPort), '--strictPort'],
      {
        cwd: demoRoot,
        env: demoEnvironment,
        stdio: 'inherit',
      },
    );
    await waitForUrl(baseUrl, vite);
  } else {
    await waitForUrl(baseUrl);
  }

  const browser = await puppeteer.launch({ headless: true });
  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 900, deviceScaleFactor: 1 });
    await page.goto(baseUrl, { waitUntil: 'networkidle0' });
    await page.select('#resolution', '1m');
    await page.click('#go-live');
    await page.waitForFunction(
      () => {
        const button = document.querySelector('#go-live') as HTMLButtonElement;
        const error = document.querySelector('#error') as HTMLElement;
        return !button.disabled || !error.hidden;
      },
      { timeout: 60_000 },
    );

    const error = await page.$eval('#error', (node) => ({
      hidden: (node as HTMLElement).hidden,
      text: node.textContent ?? '',
    }));
    assert.equal(error.hidden, true, error.text);
    const notice = await page.$eval('#notice', (node) => node.textContent ?? '');
    assert.match(notice, /Live snapshot loaded|Connection state: live/);
    assert.ok((await page.$$('#chart canvas')).length >= 2, 'Candles and volume must render.');
    assert.equal(
      await page.evaluate(() => typeof window.lwcDatabentoDemo?.dispose === 'function'),
      true,
      'The standalone consumer must mount the public adapter-backed demo.',
    );
    const resourceUrls = await page.evaluate(() =>
      performance.getEntriesByType('resource').map((entry) => entry.name),
    );
    assert.equal(
      resourceUrls.some((url) => url.includes('DATABENTO_API_KEY') || url.includes('db-')),
      false,
      'Browser resource URLs must not contain credentials.',
    );

    await page.click('#disconnect');
    await page.waitForFunction(() =>
      (document.querySelector('#notice')?.textContent ?? '').includes('Disconnected'),
    );
  } finally {
    await browser.close();
    await stop(vite);
    await stop(gateway);
  }
}

void run();
