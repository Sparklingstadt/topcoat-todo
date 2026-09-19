import { type Page } from '@playwright/test';
import { test, expect, addTodo, seedTodos } from './fixtures';

async function snapshot(page: Page, name: string) {
  await page.evaluate(() => document.fonts.ready);
  await page.mouse.move(0, 0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await expect(page).toHaveScreenshot(name, { fullPage: true });
}

test('empty list', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: '最初のTodoを追加しましょう' })).toBeVisible();
  await snapshot(page, 'empty-list.png');
});

test('mixed active and completed list', async ({ page }) => {
  await seedTodos(page);
  await snapshot(page, 'mixed-list.png');
});

test('active filter', async ({ page }) => {
  await seedTodos(page);
  await page.getByRole('link', { name: '未完了 2', exact: true }).click();
  await expect(page.locator('.todo-row')).toHaveCount(2);
  await snapshot(page, 'active-filter.png');
});

test('completed filter', async ({ page }) => {
  await seedTodos(page);
  await page.getByRole('link', { name: '完了 1', exact: true }).click();
  await expect(page.locator('.todo-row')).toHaveCount(1);
  await snapshot(page, 'completed-filter.png');
});

test('all complete with an empty active filter', async ({ page }) => {
  await page.goto('/');
  await addTodo(page, '今日のやること');
  await page.getByRole('button', { name: '今日のやることを完了にする', exact: true }).click();
  await page.getByRole('link', { name: '未完了 0', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'すべてのTodoが完了しました' })).toBeVisible();
  await snapshot(page, 'all-completed.png');
});

test('validation error', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('textbox', { name: '新しいTodo' }).fill('　 ');
  await page.getByRole('button', { name: '追加', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await snapshot(page, 'validation-error.png');
});

test('long Japanese and unbroken titles wrap', async ({ page }) => {
  await page.goto('/');
  await addTodo(page, '長いTodoも読みやすく折り返されることを確認する。'.repeat(4));
  await addTodo(page, 'abcdefghij'.repeat(12));
  await snapshot(page, 'long-titles.png');
});

test('delete confirmation dialog', async ({ page }) => {
  await seedTodos(page);
  await page.getByRole('button', { name: '読みかけの本を10ページ読むを削除', exact: true }).click();
  await expect(page.getByRole('alertdialog')).toBeVisible();
  await snapshot(page, 'delete-confirmation.png');
});

test('keyboard focus on the new Todo input', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('textbox', { name: '新しいTodo' }).focus();
  await snapshot(page, 'input-focus.png');
});
