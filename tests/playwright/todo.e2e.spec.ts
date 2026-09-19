import { test, expect, addTodo, expectCounts, addDetailedTodo, searchTodos } from './fixtures';

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
  await expectCounts(page, 1, 1, 0);
  await page.getByRole('button', { name: '本を読むを削除', exact: true }).click();
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
  const context = await browser.newContext({ baseURL: app.url, javaScriptEnabled: false });
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

test('dates and priorities are saved and determine list order without JavaScript', async ({ page }) => {
  await expect(page.locator('script')).toHaveCount(0);
  await addDetailedTodo(page, '低い優先度', '2028-01-01', 'low');
  await addDetailedTodo(page, '高い優先度・遅い期限', '2028-03-01', 'high');
  await addDetailedTodo(page, '高い優先度・早い期限', '2028-02-29', 'high');
  await addTodo(page, '期限のないTodo');
  await expect(page.locator('.todo-title')).toHaveText(['高い優先度・早い期限', '高い優先度・遅い期限', '期限のないTodo', '低い優先度']);
  await expect(page.locator('.todo-row').first().locator('time')).toHaveAttribute('datetime', '2028-02-29');
  await expect(page.locator('.todo-row').first().locator('.priority')).toHaveText('優先度：高');
  await page.getByRole('button', { name: '高い優先度・早い期限を完了にする', exact: true }).click();
  await expect(page.locator('.todo-title').last()).toHaveText('高い優先度・早い期限');
  await page.reload();
  await expectCounts(page, 4, 3, 1);
});

test('edit and cancel preserve completion; dates can be cleared and changes survive restart', async ({ page, app }) => {
  await addDetailedTodo(page, '編集するTodo', '2028-02-29', 'high');
  await page.getByRole('button', { name: '編集するTodoを完了にする', exact: true }).click();
  await page.getByRole('link', { name: '完了 1', exact: true }).click();
  await page.getByRole('link', { name: '編集するTodoを編集', exact: true }).click();
  await expect(page.getByLabel('Todoの内容', { exact: true })).toHaveValue('編集するTodo');
  await page.getByLabel('Todoの内容', { exact: true }).fill('キャンセルする編集');
  await page.getByRole('link', { name: 'キャンセル', exact: true }).click();
  await expect(page.getByRole('button', { name: '編集するTodoを未完了に戻す', exact: true })).toBeVisible();
  await page.getByRole('link', { name: '編集するTodoを編集', exact: true }).click();
  await page.getByLabel('Todoの内容', { exact: true }).fill('  変更したTodo  ');
  await page.getByLabel(/^期限\s*任意$/).fill('');
  await page.getByLabel('優先度', { exact: true }).selectOption('low');
  await page.getByRole('button', { name: '変更を保存', exact: true }).click();
  await expect(page).toHaveURL(/filter=completed$/);
  await expect(page.getByRole('button', { name: '変更したTodoを未完了に戻す', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.todo-row time')).toHaveCount(0);
  await expect(page.locator('.priority')).toHaveText('優先度：低');
  await app.restart();
  await page.reload();
  await expect(page.locator('.todo-title')).toHaveText('変更したTodo');
  await expectCounts(page, 1, 0, 1);
});

test('invalid edits preserve the submitted fields and do not modify stored data', async ({ page, request, app }) => {
  await addTodo(page, '編集前のTodo');
  const id = await page.locator('.todo-row').getAttribute('data-todo-id');
  await page.getByRole('link', { name: '編集前のTodoを編集', exact: true }).click();
  await page.getByLabel('Todoの内容', { exact: true }).fill('　 ');
  await page.getByLabel(/^期限\s*任意$/).fill('2028-02-29');
  await page.getByLabel('優先度', { exact: true }).selectOption('high');
  await page.getByRole('button', { name: '変更を保存', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('200文字以内');
  await expect(page.getByLabel('Todoの内容', { exact: true })).toHaveValue('　 ');
  await expect(page.getByLabel(/^期限\s*任意$/)).toHaveValue('2028-02-29');
  await expect(page.getByLabel('優先度', { exact: true })).toHaveValue('high');
  for (const changes of [{ due_date: '2026-02-30' }, { priority: 'urgent' }, { title: 'あ'.repeat(201) }]) {
    const response = await request.post('/todos/update', { form: { id: id!, title: '不正な編集', due_date: '', priority: 'normal', ...changes }, headers: { Origin: app.url }, maxRedirects: 0 });
    expect(response.status()).toBe(400);
  }
  await page.getByRole('link', { name: 'キャンセル', exact: true }).click();
  await expect(page.locator('.todo-title')).toHaveText('編集前のTodo');
  await expect(page.locator('.todo-row time')).toHaveCount(0);
  await expect(page.locator('.priority')).toHaveText('優先度：通常');
});

test('literal search, filters, edits, and deletion retain the search context', async ({ page }) => {
  const first = 'Rust %_ & /?を読む';
  await addTodo(page, first);
  await addTodo(page, 'RUST %_ & /?のメモ');
  await addTodo(page, '検索対象外');
  await searchTodos(page, 'rust %_ & /?');
  await expectCounts(page, 2, 2, 0);
  await page.getByRole('button', { name: `${first}を完了にする`, exact: true }).click();
  await expect(page.getByRole('searchbox')).toHaveValue('rust %_ & /?');
  await page.getByRole('link', { name: '完了 1', exact: true }).click();
  await page.getByRole('link', { name: `${first}を編集`, exact: true }).click();
  await page.getByLabel('Todoの内容', { exact: true }).fill(`${first}・改訂`);
  await page.getByRole('button', { name: '変更を保存', exact: true }).click();
  await expect(page.getByRole('searchbox')).toHaveValue('rust %_ & /?');
  await expect(page.getByRole('link', { name: '完了 1', exact: true })).toHaveAttribute('aria-current', 'page');
  await page.getByRole('button', { name: `${first}・改訂を削除`, exact: true }).click();
  await expect(page.getByRole('heading', { name: '条件に合うTodoがありません' })).toBeVisible();
  await page.getByRole('link', { name: 'すべて 1', exact: true }).click();
  await searchTodos(page, '見つからない');
  await expectCounts(page, 0, 0, 0);
  await page.getByRole('link', { name: '検索を解除', exact: true }).click();
  await expectCounts(page, 2, 2, 0);
});

test('bulk deletion has a read-only confirmation, cancellation, and required consent', async ({ page }) => {
  await addTodo(page, '完了のTodo');
  await addTodo(page, '残るTodo');
  await page.getByRole('button', { name: '完了のTodoを完了にする', exact: true }).click();
  await page.getByRole('link', { name: '完了済み 1 件を削除', exact: true }).click();
  await expect(page.getByRole('heading', { name: '1 件を削除しますか？' })).toBeVisible();
  await page.getByRole('link', { name: 'キャンセル', exact: true }).click();
  await expectCounts(page, 2, 1, 1);
  await page.getByRole('link', { name: '完了済み 1 件を削除', exact: true }).click();
  await page.getByRole('button', { name: '1 件を削除する', exact: true }).click();
  expect(await page.getByRole('checkbox').evaluate((element: HTMLInputElement) => element.validity.valueMissing)).toBe(true);
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: '1 件を削除する', exact: true }).click();
  await expectCounts(page, 1, 1, 0);
  await expect(page.locator('.todo-title')).toHaveText('残るTodo');
});

test('bulk deletion respects search and the snapshot when other requests change the list', async ({ page, request, app }) => {
  for (const title of ['対象A', '対象B', '別件', '後で対象']) await addTodo(page, title);
  const ids: Record<string, string> = {};
  for (const row of await page.locator('.todo-row').all()) ids[await row.locator('.todo-title').innerText()] = (await row.getAttribute('data-todo-id'))!;
  for (const title of ['対象A', '対象B', '別件']) await page.getByRole('button', { name: `${title}を完了にする`, exact: true }).click();
  await searchTodos(page, '対象');
  await page.getByRole('link', { name: '完了 2', exact: true }).click();
  await page.getByRole('link', { name: '完了済み 2 件を削除', exact: true }).click();
  await expect(page.locator('.confirmation-list li')).toHaveCount(2);
  for (const [title, completed] of [['対象A', 'false'], ['後で対象', 'true']]) {
    expect((await request.post('/todos/complete', { form: { id: ids[title], completed }, headers: { Origin: app.url }, maxRedirects: 0 })).status()).toBe(303);
  }
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: '2 件を削除する', exact: true }).click();
  await expect(page.getByRole('searchbox')).toHaveValue('対象');
  await expect(page.getByRole('button', { name: '後で対象を未完了に戻す', exact: true })).toBeVisible();
  await page.goto('/');
  await expectCounts(page, 3, 1, 2);
  await expect(page.getByRole('button', { name: '対象Aを完了にする', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: '別件を未完了に戻す', exact: true })).toBeVisible();
});

test('new endpoints reject invalid dates, priorities, unconfirmed bulk requests, and foreign origins', async ({ page, request, app }) => {
  await addTodo(page, 'APIで守るTodo');
  const id = (await page.locator('.todo-row').getAttribute('data-todo-id'))!;
  await page.getByRole('button', { name: 'APIで守るTodoを完了にする', exact: true }).click();
  for (const fields of [{ due_date: '2026-02-30' }, { due_date: '0000-01-01' }, { priority: 'urgent' }]) {
    const response = await request.post('/todos/add', { form: { title: '無効な追加', ...fields }, headers: { Origin: app.url }, maxRedirects: 0 });
    expect(response.status()).toBe(303);
    expect(response.headers().location).toMatch(/error=(date|priority)$/);
  }
  for (const form of [{ ids: id }, { ids: '-1', confirmation: 'delete' }, { ids: 'abc', confirmation: 'delete' }]) {
    expect((await request.post('/todos/clear-completed', { form, headers: { Origin: app.url }, maxRedirects: 0 })).status()).toBe(400);
  }
  for (const [url, form] of [
    ['/todos/update', { id, title: '不正更新' }],
    ['/todos/clear-completed', { ids: id, confirmation: 'delete' }],
  ] as const) {
    expect((await request.post(url, { form, headers: { Origin: 'https://example.invalid' } })).status()).toBe(403);
  }
  expect((await request.get('/todos/update')).status()).toBe(405);
  await page.goto('/todos/edit?id=9223372036854775807');
  await expect(page.getByRole('alert')).toHaveText('このTodoはすでに削除されています。');
  await expectCounts(page, 1, 0, 1);
  await expect(page.locator('.todo-title')).toHaveText('APIで守るTodo');
});

test('escaped titles and search values cannot execute even with JavaScript enabled', async ({ browser, app }) => {
  const context = await browser.newContext({ baseURL: app.url, javaScriptEnabled: true });
  try {
    const page = await context.newPage();
    const title = '\"><img src=x onerror="window.todoXss=1"><script>window.todoXss=1</script>';
    await page.goto('/');
    await addTodo(page, title);
    await page.getByRole('link', { name: `${title}を編集`, exact: true }).click();
    await expect(page.getByLabel('Todoの内容', { exact: true })).toHaveValue(title);
    await page.getByRole('button', { name: '変更を保存', exact: true }).click();
    await searchTodos(page, title);
    await expect(page.locator('.todo-title')).toHaveText(title);
    await expect(page.locator('script, img')).toHaveCount(0);
    expect(await page.evaluate(() => (window as unknown as { todoXss?: number }).todoXss)).toBeUndefined();
  } finally { await context.close(); }
});
