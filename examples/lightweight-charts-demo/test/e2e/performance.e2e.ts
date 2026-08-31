import assert from 'node:assert/strict';
import { spawn, type ChildProcess } from 'node:child_process';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import puppeteer from 'puppeteer';

const testDirectory = dirname(fileURLToPath(import.meta.url));
const demoRoot = join(testDirectory, '../..');
const routes = ['/'] as const;
const warmupRuns = 3;
const measuredRuns = 20;
const maximumP95Milliseconds = 50;

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

async function waitForUrl(url: string, process: ChildProcess): Promise<void> {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (process.exitCode !== null || process.signalCode !== null)
      throw new Error(`Preview exited before ${url} became ready`);
    try {
      if ((await fetch(url)).ok) return;
    } catch {
      // The bounded retry loop is the preview startup boundary.
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function stop(process: ChildProcess): Promise<void> {
  if (process.exitCode !== null) return;
  process.kill('SIGTERM');
  await Promise.race([once(process, 'exit'), new Promise((resolve) => setTimeout(resolve, 5_000))]);
  if (process.exitCode === null) process.kill('SIGKILL');
}

function percentile(values: number[], percentileValue: number): number {
  const sorted = values.toSorted((left, right) => left - right);
  return sorted[Math.ceil(sorted.length * percentileValue) - 1] ?? Number.POSITIVE_INFINITY;
}

async function run(): Promise<void> {
  const port = await availablePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const preview = spawn(
    'pnpm',
    ['exec', 'vite', 'preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'],
    { cwd: demoRoot, stdio: 'ignore' },
  );
  await waitForUrl(baseUrl, preview);

  const browser = await puppeteer.launch({ headless: true });
  try {
    for (const route of routes) {
      const page = await browser.newPage();
      await page.setCacheEnabled(false);
      await page.setViewport({ width: 1280, height: 900, deviceScaleFactor: 1 });
      const url = new URL(route, baseUrl).href;

      for (let runIndex = 0; runIndex < warmupRuns; runIndex += 1)
        await page.goto(url, { waitUntil: 'load' });
      assert.equal(
        await page.evaluate(() => window.lwcDatabentoDemo),
        undefined,
        'production preview must not expose the E2E app handle',
      );

      const durations: number[] = [];
      for (let runIndex = 0; runIndex < measuredRuns; runIndex += 1) {
        await page.goto(url, { waitUntil: 'load' });
        durations.push(
          await page.evaluate(() => {
            const entry = performance.getEntriesByType(
              'navigation',
            )[0] as PerformanceNavigationTiming;
            return entry.duration;
          }),
        );
      }

      const p95 = percentile(durations, 0.95);
      const maximum = Math.max(...durations);
      console.log(
        JSON.stringify({ route, warmupRuns, measuredRuns, cache: 'disabled', p95, maximum }),
      );
      assert.ok(
        p95 < maximumP95Milliseconds,
        `${route} p95 ${p95.toFixed(2)} ms must be below ${maximumP95Milliseconds} ms`,
      );
      await page.close();
    }
  } finally {
    await browser.close();
    await stop(preview);
  }
}

void run();
