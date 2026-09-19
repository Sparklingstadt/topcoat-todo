import { test, expect, addTodo, addDetailedTodo, expectCounts, searchTodos } from './fixtures';

test.beforeEach(async ({ page }) => { await page.goto('/'); });

test('saving an editor after another request deletes its Todo does not recreate it', async ({ page, request, app }) => {
  await addTodo(page, '共有のTodo');
  await searchTodos(page, '共有');
  await page.getByRole('link', { name: '未完了 1', exact: true }).click();
  const id = (await page.locator('.todo-row').getAttribute('data-todo-id'))!;
  await page.getByRole('link', { name: '共有のTodoを編集', exact: true }).click();
  const removed = await request.post('/todos/delete', {
    form: { id }, headers: { Origin: app.url }, maxRedirects: 0,
  });
  expect(removed.status()).toBe(303);
  await page.getByLabel('Todoの内容', { exact: true }).fill('削除後に保存しようとした内容');
  await page.getByRole('button', { name: '変更を保存', exact: true }).click();
  await expect(page.getByRole('alert')).toHaveText('このTodoはすでに削除されています。');
  await expect(page.getByRole('searchbox')).toHaveValue('共有');
  await expect(page.getByRole('link', { name: '未完了 0', exact: true })).toHaveAttribute('aria-current', 'page');
  await page.goto('/');
  await expectCounts(page, 0, 0, 0);
});

test('saving an old editor retains completion changed in a second tab', async ({ page }) => {
  await addDetailedTodo(page, '別タブと共有', '2028-02-29', 'high');
  await page.getByRole('link', { name: '別タブと共有を編集', exact: true }).click();
  const otherPage = await page.context().newPage();
  try {
    await otherPage.goto('/');
    await otherPage.getByRole('button', { name: '別タブと共有を完了にする', exact: true }).click();
    await page.getByLabel('Todoの内容', { exact: true }).fill('保存後の共有Todo');
    await page.getByRole('button', { name: '変更を保存', exact: true }).click();
    await expectCounts(page, 1, 0, 1);
    await expect(page.getByRole('button', { name: '保存後の共有Todoを未完了に戻す', exact: true })).toHaveAttribute('aria-pressed', 'true');
    await otherPage.reload();
    await expect(otherPage.locator('.todo-title')).toHaveText('保存後の共有Todo');
    await expect(otherPage.locator('time')).toHaveAttribute('datetime', '2028-02-29');
  } finally { await otherPage.close(); }
});

test('identical titles can be edited and deleted independently by their IDs', async ({ page, request, app }) => {
  for (let index = 0; index < 2; index++) {
    const response = await request.post('/todos/add', {
      form: { title: '同じ名前' }, headers: { Origin: app.url }, maxRedirects: 0,
    });
    expect(response.status()).toBe(303);
  }
  await page.reload();
  await expectCounts(page, 2, 2, 0);
  const ids = await page.locator('.todo-row').evaluateAll(rows => rows.map(row => row.getAttribute('data-todo-id')));
  expect(new Set(ids).size).toBe(2);
  await page.locator('.todo-row').first().getByRole('link', { name: '同じ名前を編集', exact: true }).click();
  await page.getByLabel('Todoの内容', { exact: true }).fill('片方だけ変更');
  await page.getByRole('button', { name: '変更を保存', exact: true }).click();
  await expect(page.locator(`[data-todo-id="${ids[0]}"] .todo-title`)).toHaveText('片方だけ変更');
  await expect(page.locator(`[data-todo-id="${ids[1]}"] .todo-title`)).toHaveText('同じ名前');
  await page.getByRole('button', { name: '片方だけ変更を削除', exact: true }).click();
  await expectCounts(page, 1, 1, 0);
  await expect(page.locator('.todo-row')).toHaveAttribute('data-todo-id', ids[1]!);
  await expect(page.locator('.todo-title')).toHaveText('同じ名前');
});

test('replaying an already submitted bulk form cannot delete a newly created completed Todo', async ({ page, request, app }) => {
  await addTodo(page, '削除して作り直す');
  await page.getByRole('button', { name: '削除して作り直すを完了にする', exact: true }).click();
  await page.getByRole('link', { name: '完了済み 1 件を削除', exact: true }).click();
  const ids = await page.locator('input[name="ids"]').inputValue();
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: '1 件を削除する', exact: true }).click();
  await expectCounts(page, 0, 0, 0);
  await addTodo(page, '削除して作り直す');
  await page.getByRole('button', { name: '削除して作り直すを完了にする', exact: true }).click();
  const newId = await page.locator('.todo-row').getAttribute('data-todo-id');
  expect(newId).not.toBe(ids);
  const replay = await request.post('/todos/clear-completed', {
    form: { ids, confirmation: 'delete' }, headers: { Origin: app.url }, maxRedirects: 0,
  });
  expect(replay.status()).toBe(303);
  await page.reload();
  await expectCounts(page, 1, 0, 1);
  await expect(page.locator('.todo-row')).toHaveAttribute('data-todo-id', newId!);
});

test('adding from a searched completed list clears the search and reveals new details', async ({ page }) => {
  await addTodo(page, '検索対象の完了Todo');
  await page.getByRole('button', { name: '検索対象の完了Todoを完了にする', exact: true }).click();
  await searchTodos(page, '検索対象');
  await page.getByRole('link', { name: '完了 1', exact: true }).click();
  await addDetailedTodo(page, '違う内容の新しいTodo', '2028-03-01', 'low');
  await expect(page).toHaveURL(/\?filter=all$/);
  await expect(page.getByRole('searchbox')).toHaveValue('');
  await expectCounts(page, 2, 1, 1);
  await expect(page.locator('.todo-row').first().locator('time')).toHaveAttribute('datetime', '2028-03-01');
  await expect(page.locator('.todo-row').first().locator('.priority')).toHaveText('優先度：低');
  await page.reload();
  await expectCounts(page, 2, 1, 1);
});

test('normalized searches survive reload and edited nonmatches remain in the full list', async ({ page }) => {
  await addTodo(page, 'Rustの予定');
  await addTodo(page, '別の予定');
  await searchTodos(page, '　 Rust　 ');
  await expect(page.getByRole('searchbox')).toHaveValue('Rust');
  await expectCounts(page, 1, 1, 0);
  await expect(page.getByRole('group', { name: '2件中0件完了', exact: true })).toBeVisible();
  const bookmark = page.url();
  await page.reload();
  await expect(page.getByRole('searchbox')).toHaveValue('Rust');
  await page.getByRole('link', { name: 'Rustの予定を編集', exact: true }).click();
  await page.getByLabel('Todoの内容', { exact: true }).fill('変更後の予定');
  await page.getByRole('button', { name: '変更を保存', exact: true }).click();
  await expectCounts(page, 0, 0, 0);
  await expect(page.getByRole('heading', { name: '条件に合うTodoがありません' })).toBeVisible();
  await page.goto(bookmark);
  await expectCounts(page, 0, 0, 0);
  await page.getByRole('link', { name: '検索を解除', exact: true }).click();
  await expectCounts(page, 2, 2, 0);
  await expect(page.getByRole('button', { name: '変更後の予定を完了にする', exact: true })).toBeVisible();
});

test('keyboard activation supports add options, editing, search, and cancellation', async ({ page }) => {
  const options = page.locator('.add-options > summary');
  await options.focus();
  await page.keyboard.press('Space');
  await expect(page.getByLabel('優先度', { exact: true })).toBeVisible();
  await page.getByLabel('優先度', { exact: true }).focus();
  await page.keyboard.press('ArrowUp');
  await expect(page.getByLabel('優先度', { exact: true })).toHaveValue('high');
  await page.getByRole('textbox', { name: '新しいTodo' }).focus();
  await page.keyboard.type('Keyboard Todo');
  await page.keyboard.press('Enter');
  await expect(page.locator('.priority')).toHaveText('優先度：高');
  await page.getByRole('link', { name: 'Keyboard Todoを編集', exact: true }).focus();
  await page.keyboard.press('Enter');
  await page.getByLabel('Todoの内容', { exact: true }).focus();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.type('Keyboard updated');
  await page.getByRole('button', { name: '変更を保存', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('.todo-title')).toHaveText('Keyboard updated');
  await page.getByRole('searchbox').focus();
  await page.keyboard.type('updated');
  await page.keyboard.press('Enter');
  await expectCounts(page, 1, 1, 0);
  await page.getByRole('link', { name: 'Keyboard updatedを編集', exact: true }).focus();
  await page.keyboard.press('Enter');
  await page.getByRole('link', { name: 'キャンセル', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('searchbox')).toHaveValue('updated');
});
