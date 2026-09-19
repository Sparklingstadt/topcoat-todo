import AxeBuilder from '@axe-core/playwright';
import { test, expect, addTodo, expectCounts } from './fixtures';

test.beforeEach(async ({ page }) => { await page.goto('/'); });

test('add, filter, complete, restore, and delete through the UI', async ({ page }) => {
  await expect(page.getByRole('heading', { name: '最初のTodoを追加しましょう' })).toBeVisible();
  await expectCounts(page, 0, 0, 0);
  await addTodo(page, '　買い物に行く　', true);
  await addTodo(page, '本を読む');
  await expectCounts(page, 2, 2, 0);
  await page.getByRole('button', { name: '買い物に行くを完了にする', exact: true }).click();
  await expectCounts(page, 2, 1, 1);
  await page.getByRole('link', { name: '完了 1', exact: true }).click();
  await expect(page.getByRole('link', { name: '完了 1', exact: true })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('button', { name: '本を読むを完了にする', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: '買い物に行くを未完了に戻す', exact: true }).click();
  await expect(page.getByRole('heading', { name: '完了したTodoはまだありません' })).toBeVisible();
  await expectCounts(page, 2, 2, 0);
  await page.getByRole('link', { name: '未完了 2', exact: true }).click();
  await page.getByRole('button', { name: '買い物に行くを削除', exact: true }).click();
  await page.getByRole('alertdialog').getByRole('button', { name: '削除する', exact: true }).click();
  await expectCounts(page, 1, 1, 0);
  await page.getByRole('button', { name: '本を読むを削除', exact: true }).click();
  await page.getByRole('alertdialog').getByRole('button', { name: '削除する', exact: true }).click();
  await expectCounts(page, 0, 0, 0);
  await expect(page.getByRole('heading', { name: '最初のTodoを追加しましょう' })).toBeVisible();
});

test('reload, a new browser context, and a server restart keep data', async ({ page, browser, app }) => {
  const title = '再起動後も残るTodo';
  await addTodo(page, title);
  await page.getByRole('button', { name: `${title}を完了にする`, exact: true }).click();
  await page.reload();
  await expectCounts(page, 1, 0, 1);
  await app.restart();
  await page.reload();
  await expect(page.getByRole('button', { name: `${title}を未完了に戻す`, exact: true })).toHaveAttribute('aria-pressed', 'true');
  const context = await browser.newContext({ baseURL: app.url });
  try {
    const otherPage = await context.newPage();
    await otherPage.goto('/');
    await expectCounts(otherPage, 1, 0, 1);
    await expect(otherPage.getByRole('button', { name: `${title}を未完了に戻す`, exact: true })).toBeVisible();
  } finally {
    await context.close();
  }
});

test('adding from completed reveals the new item and adding from active keeps the filter', async ({ page }) => {
  await page.getByRole('link', { name: '完了 0', exact: true }).click();
  await addTodo(page, '完了タブから追加');
  await expect(page).toHaveURL(/\?filter=all$/);
  await page.getByRole('link', { name: '未完了 1', exact: true }).click();
  await addTodo(page, '未完了タブから追加');
  await expect(page).toHaveURL(/\?filter=active$/);
  await page.reload();
  await expectCounts(page, 2, 2, 0);
});

test('native required validation and server whitespace validation prevent empty todos', async ({ page }) => {
  const input = page.getByRole('textbox', { name: '新しいTodo' });
  await page.getByRole('button', { name: '追加', exact: true }).click();
  expect(await input.evaluate((element: HTMLInputElement) => element.validity.valueMissing)).toBe(true);
  await expectCounts(page, 0, 0, 0);
  await input.fill('　 \t ');
  await page.getByRole('button', { name: '追加', exact: true }).click();
  await expect(page.getByRole('alert')).toHaveText('Todoは空白以外の文字を含む、200文字以内で入力してください。');
  await expectCounts(page, 0, 0, 0);
});

test('200 Japanese characters are accepted and server rejects 201 even if HTML is bypassed', async ({ page, request, app }) => {
  const title = 'あ'.repeat(200);
  await expect(page.getByRole('textbox', { name: '新しいTodo' })).toHaveAttribute('maxlength', '200');
  await addTodo(page, title);
  const response = await request.post('/todos/add', {
    form: { title: `${title}あ`, filter: 'all' },
    headers: { Origin: app.url },
    maxRedirects: 0,
  });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe('/?filter=all&error=title');
  await page.goto(response.headers().location);
  await expect(page.getByRole('alert')).toBeVisible();
  await expectCounts(page, 1, 1, 0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});

test('HTML-like titles render as text and never execute', async ({ page }) => {
  const title = '\"><img src=x onerror="window.todoXss=1"><script>window.todoXss=1</script>';
  await addTodo(page, title);
  await expect(page.locator('.todo-title')).toHaveText(title);
  await expect(page.locator('.todo-list img, .todo-list script')).toHaveCount(0);
  expect(await page.evaluate(() => (window as unknown as { todoXss?: number }).todoXss)).toBeUndefined();
  await page.reload();
  await expect(page.locator('.todo-title')).toHaveText(title);
});

test('repeated completion requests are idempotent', async ({ page, request, app }) => {
  await addTodo(page, '二重送信を確認');
  const id = await page.locator('.todo-row').getAttribute('data-todo-id');
  expect(id).toBeTruthy();
  for (let index = 0; index < 2; index++) {
    const response = await request.post('/todos/complete', {
      form: { id: id!, completed: 'true', filter: 'all' },
      headers: { Origin: app.url }, maxRedirects: 0,
    });
    expect(response.status()).toBe(303);
  }
  await page.reload();
  await expectCounts(page, 1, 0, 1);
  await expect(page.getByRole('button', { name: '二重送信を確認を未完了に戻す', exact: true })).toHaveAttribute('aria-pressed', 'true');
});

test('invalid requests and cross-origin posts do not mutate the list', async ({ page, request, app }) => {
  expect((await request.get('/todos/add')).status()).toBe(405);
  expect((await request.post('/todos/complete', {
    form: { id: 'invalid', completed: 'true' }, headers: { Origin: app.url },
  })).status()).toBe(400);
  expect((await request.post('/todos/add', {
    form: { title: '他サイトからの追加' }, headers: { Origin: 'https://example.invalid' },
  })).status()).toBe(403);
  const missing = await request.post('/todos/delete', {
    form: { id: '9223372036854775807', filter: 'active' }, headers: { Origin: app.url }, maxRedirects: 0,
  });
  expect(missing.status()).toBe(303);
  await page.goto(missing.headers().location);
  await expect(page.getByRole('alert')).toHaveText('このTodoはすでに削除されています。');
  await expectCounts(page, 0, 0, 0);
});

test('parallel writes are all visible without lost data', async ({ page, request, app }) => {
  const titles = Array.from({ length: 8 }, (_, index) => `同時追加 ${index + 1}`);
  const responses = await Promise.all(titles.map(title => request.post('/todos/add', {
    form: { title, filter: 'all' }, headers: { Origin: app.url }, maxRedirects: 0,
  })));
  responses.forEach(response => expect(response.status()).toBe(303));
  await page.reload();
  await expectCounts(page, 8, 8, 0);
  await expect(page.locator('.todo-row')).toHaveCount(8);
  for (const title of titles) {
    await expect(page.getByRole('button', { name: `${title}を完了にする`, exact: true })).toBeVisible();
  }
});

test('deletion requires confirmation; cancel and Escape preserve data and return focus', async ({ page }) => {
  await addTodo(page, '残しておくTodo');
  await addTodo(page, '削除を確認するTodo');
  await page.getByRole('link', { name: '未完了 2', exact: true }).click();
  const trigger = page.getByRole('button', { name: '削除を確認するTodoを削除', exact: true });
  const dialog = page.getByRole('alertdialog', { name: 'このTodoを削除しますか？' });
  await trigger.click();
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'キャンセル' })).toBeFocused();
  await page.keyboard.press('/');
  await expect(dialog.getByRole('button', { name: 'キャンセル' })).toBeFocused();
  await page.keyboard.press('Shift+Tab');
  await expect(dialog.getByRole('button', { name: '削除する', exact: true })).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(dialog.getByRole('button', { name: 'キャンセル' })).toBeFocused();
  await dialog.getByRole('button', { name: 'キャンセル' }).click();
  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await expectCounts(page, 2, 2, 0);
  await trigger.click();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await trigger.click();
  await dialog.getByRole('button', { name: '削除する', exact: true }).click();
  await expect(page).toHaveURL(/\?filter=active$/);
  await expectCounts(page, 1, 1, 0);
  await expect(page.getByRole('button', { name: '残しておくTodoを完了にする', exact: true })).toBeVisible();
  await page.reload();
  await expect(trigger).toHaveCount(0);
});

test('keyboard skip link, slash shortcut, and Enter submission work without intercepting typing', async ({ page }) => {
  await expect(page.getByRole('textbox', { name: '新しいTodo' })).toBeVisible();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Todoへ移動' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('#main')).toBeFocused();
  await page.keyboard.press('/');
  const input = page.getByRole('textbox', { name: '新しいTodo' });
  await expect(input).toBeFocused();
  await input.fill('キーボード');
  await input.press('/');
  await expect(input).toHaveValue('キーボード/');
  await input.press('Enter');
  await expect(page.getByRole('button', { name: 'キーボード/を完了にする', exact: true })).toBeVisible();
  await page.reload();
  await expectCounts(page, 1, 1, 0);
});

test('progress exposes correct accessible values for empty, mixed, completed, and restored lists', async ({ page }) => {
  const progress = page.getByRole('progressbar', { name: 'Todoの完了率' });
  await expect(progress).toHaveAttribute('aria-valuenow', '0');
  await addTodo(page, 'ひとつめ');
  await addTodo(page, 'ふたつめ');
  await page.getByRole('button', { name: 'ひとつめを完了にする', exact: true }).click();
  await expect(progress).toHaveAttribute('aria-valuenow', '50');
  await expect(page.getByRole('group', { name: '2件中1件完了' })).toBeVisible();
  await page.getByRole('button', { name: 'ふたつめを完了にする', exact: true }).click();
  await expect(progress).toHaveAttribute('aria-valuenow', '100');
  await page.getByRole('button', { name: 'ふたつめを未完了に戻す', exact: true }).click();
  await expect(progress).toHaveAttribute('aria-valuenow', '50');
});

test('deep links and browser history keep the selected filter', async ({ page }) => {
  await addTodo(page, '履歴で確認');
  await page.getByRole('button', { name: '履歴で確認を完了にする', exact: true }).click();
  await page.goto('/?filter=completed');
  await expect(page.getByRole('link', { name: '完了 1', exact: true })).toHaveAttribute('aria-current', 'page');
  await page.getByRole('link', { name: '未完了 0', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'すべてのTodoが完了しました' })).toBeVisible();
  await page.goBack();
  await expect(page.getByRole('button', { name: '履歴で確認を未完了に戻す', exact: true })).toBeVisible();
  await page.goForward();
  await expect(page.getByRole('link', { name: '未完了 0', exact: true })).toHaveAttribute('aria-current', 'page');
  await page.goto('/?filter=unknown');
  await expect(page.getByRole('link', { name: 'すべて 1', exact: true })).toHaveAttribute('aria-current', 'page');
});


test('empty, populated, and dialog states have no automated accessibility violations', async ({ page }) => {
  await expect(page.getByRole('heading', { name: '最初のTodoを追加しましょう' })).toBeVisible();
  const audit = () => new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
  expect((await audit()).violations).toEqual([]);
  await addTodo(page, 'アクセシビリティを確認');
  expect((await audit()).violations).toEqual([]);
  await page.getByRole('button', { name: 'アクセシビリティを確認を削除', exact: true }).click();
  await expect(page.getByRole('alertdialog')).toBeVisible();
  expect((await audit()).violations).toEqual([]);
});
