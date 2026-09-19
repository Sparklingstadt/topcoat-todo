import { type Page } from '@playwright/test';
import { test, expect, addTodo, seedTodos, addDetailedTodo, searchTodos } from './fixtures';

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

test('priority, due dates, and expanded add options', async ({ page }) => {
  await page.goto('/');
  await addDetailedTodo(page, '大切な予定を整理する', '2028-02-29', 'high');
  await addDetailedTodo(page, '時間があれば本を読む', '2028-03-10', 'low');
  await page.locator('.add-options > summary').click();
  await snapshot(page, 'due-priority-options.png');
});

test('edit form', async ({ page }) => {
  await page.goto('/');
  await addDetailedTodo(page, '大切な予定を整理する', '2028-02-29', 'high');
  await page.getByRole('link', { name: '大切な予定を整理するを編集', exact: true }).click();
  await snapshot(page, 'edit-form.png');
});

test('invalid edit retains draft fields', async ({ page }) => {
  await page.goto('/');
  await addDetailedTodo(page, '大切な予定を整理する', '2028-02-29', 'high');
  await page.getByRole('link', { name: '大切な予定を整理するを編集', exact: true }).click();
  await page.getByLabel('Todoの内容', { exact: true }).fill('　 ');
  await page.getByRole('button', { name: '変更を保存', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await snapshot(page, 'edit-error.png');
});

test('search results', async ({ page }) => {
  await seedTodos(page);
  await searchTodos(page, '予定');
  await snapshot(page, 'search-results.png');
});

test('empty search results', async ({ page }) => {
  await seedTodos(page);
  await searchTodos(page, '見つからない');
  await snapshot(page, 'search-empty.png');
});

test('bulk deletion confirmation', async ({ page }) => {
  await seedTodos(page);
  await page.getByRole('link', { name: '完了済み 1 件を削除', exact: true }).click();
  await snapshot(page, 'bulk-confirmation.png');
});
