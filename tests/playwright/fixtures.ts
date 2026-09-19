import { test as base, expect, type Page } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

async function freePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => resolve());
  });
  const address = server.address();
  if (!address || typeof address === 'string') throw new Error('No test port');
  await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  return address.port;
}

export class TestApp {
  private process?: ChildProcess;
  private logs = '';
  private launchError?: Error;

  constructor(readonly directory: string, readonly port: number) {}

  get url() { return `http://127.0.0.1:${this.port}`; }
  get output() { return this.logs; }

  async start() {
    this.launchError = undefined;
    const binary = process.env.TODO_TEST_BINARY;
    if (!binary) throw new Error('Set TODO_TEST_BINARY or run ./scripts/test.sh (Docker).');
    const child = spawn(binary, [], {
      env: { ...process.env, HOST: '127.0.0.1', PORT: String(this.port), TODO_DB_PATH: path.join(this.directory, 'todos.sqlite3') },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    this.process = child;
    child.stdout?.on('data', chunk => { this.logs += chunk; });
    child.stderr?.on('data', chunk => { this.logs += chunk; });
    child.once('error', error => { this.launchError = error; });
    const deadline = Date.now() + 10_000;
    while (Date.now() < deadline) {
      if (this.launchError) throw this.launchError;
      if (child.exitCode !== null) throw new Error(`Test app exited: ${this.logs}`);
      try {
        const response = await fetch(`${this.url}/health`, { signal: AbortSignal.timeout(500) });
        if (response.ok && await response.text() === 'ok') return;
      } catch { /* Poll only until the isolated server is ready. */ }
      await delay(40);
    }
    throw new Error(`Test app did not become ready: ${this.logs}`);
  }

  async stop() {
    const child = this.process;
    if (!child || child.pid === undefined || child.exitCode !== null || child.signalCode !== null) return;
    await new Promise<void>(resolve => {
      const force = setTimeout(() => child.kill('SIGKILL'), 2_000);
      child.once('exit', () => { clearTimeout(force); resolve(); });
      child.kill('SIGTERM');
    });
    this.process = undefined;
  }

  async restart() {
    await this.stop();
    await this.start();
  }
}

export const test = base.extend<{ app: TestApp; pageErrors: void }>({
  app: async ({}, use, testInfo) => {
    const directory = await mkdtemp(path.join(tmpdir(), 'topcoat-playwright-'));
    const app = new TestApp(directory, await freePort());
    try {
      await app.start();
      await use(app);
    } finally {
      await app.stop();
      if (testInfo.status !== testInfo.expectedStatus) {
        await testInfo.attach('server.log', { body: app.output, contentType: 'text/plain' });
      }
      await rm(directory, { recursive: true, force: true });
    }
  },
  baseURL: async ({ app }, use) => { await use(app.url); },
  pageErrors: [async ({ page }, use) => {
    const errors: string[] = [];
    page.on('pageerror', error => errors.push(error.message));
    await use();
    expect(errors, 'Browser JavaScript errors').toEqual([]);
  }, { auto: true }],
});

export { expect };

export async function addTodo(page: Page, title: string, viaEnter = false) {
  await page.getByRole('textbox', { name: '新しいTodo' }).fill(title);
  if (viaEnter) await page.getByRole('textbox', { name: '新しいTodo' }).press('Enter');
  else await page.getByRole('button', { name: '追加', exact: true }).click();
  await expect(page.getByRole('button', { name: `${title.trim()}を完了にする`, exact: true })).toBeVisible();
}

export async function seedTodos(page: Page) {
  await page.goto('/');
  await addTodo(page, '読みかけの本を10ページ読む');
  await addTodo(page, '明日の予定を整理する');
  await addTodo(page, 'TopcoatでTodoを試す');
  await page.getByRole('button', { name: 'TopcoatでTodoを試すを完了にする', exact: true }).click();
  await expect(page.getByRole('button', { name: 'TopcoatでTodoを試すを未完了に戻す', exact: true })).toHaveAttribute('aria-pressed', 'true');
}

export async function expectCounts(page: Page, all: number, active: number, completed: number) {
  await expect(page.getByRole('link', { name: `すべて ${all}`, exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: `未完了 ${active}`, exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: `完了 ${completed}`, exact: true })).toBeVisible();
}
