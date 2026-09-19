import assert from 'node:assert/strict';
const base = process.env.TODO_TEST_URL || 'http://127.0.0.1:3000';
const prefix = `検証-${Date.now()}-`;
const created = new Set();
const passed = [];
async function html(path = '/') {
  const response = await fetch(base + path);
  assert.equal(response.status, 200);
  return response.text();
}
function rows(body) {
  return [...body.matchAll(/<li\b[^>]*data-todo-id="(\d+)"[^>]*>[\s\S]*?<\/li>/g)]
    .map(match => ({ id: match[1], html: match[0] }));
}
async function post(path, values, origin = base) {
  return fetch(base + path, { method: 'POST', headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: origin }, body: new URLSearchParams(values), redirect: 'manual' });
}
async function add(title) {
  const response = await post('/todos/add', { title, filter: 'all' });
  assert.equal(response.status, 303);
  assert.equal(response.headers.get('location'), '/?filter=all');
  const found = rows(await html()).find(row => row.html.includes(title.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')));
  assert.ok(found, `Created Todo not found: ${title}`);
  created.add(found.id);
  return found.id;
}
try {
  assert.equal(await (await fetch(base + '/health')).text(), 'ok');
  assert.match(await html(), /lang="ja"/);
  assert.match((await fetch(base + '/style.css')).headers.get('content-type'), /text\/css/);
  passed.push('HTTP health, Japanese page, CSS');
  const id = await add(prefix + '日本語のTodo');
  assert.equal((await post('/todos/complete', { id, completed: 'true', filter: 'all' })).status, 303);
  assert.equal((await post('/todos/complete', { id, completed: 'true', filter: 'all' })).status, 303);
  assert.ok(rows(await html('/?filter=completed')).some(row => row.id === id));
  assert.ok(!rows(await html('/?filter=active')).some(row => row.id === id));
  passed.push('Add, complete, idempotent completion, filters');
  await post('/todos/complete', { id, completed: 'false', filter: 'all' });
  assert.ok(rows(await html('/?filter=active')).some(row => row.id === id));
  passed.push('Return to incomplete');
  for (const title of ['  　 ', 'あ'.repeat(201)]) {
    const response = await post('/todos/add', { title, filter: 'all' });
    assert.equal(response.status, 303);
    assert.match(response.headers.get('location'), /error=title/);
  }
  passed.push('Reject whitespace and overlong titles');
  const maxTitle = prefix + 'あ'.repeat(200 - [...prefix].length);
  await add(maxTitle);
  passed.push('Accept 200 Unicode characters');
  const xss = prefix + '<img src=x onerror=alert(1)>';
  await add(xss);
  const body = await html();
  assert.ok(!body.includes('<span class="todo-title"><img'));
  assert.ok(body.includes('&lt;img src=x onerror=alert(1)&gt;'));
  passed.push('Escape Todo markup');
  assert.equal((await post('/todos/add', { title: prefix + 'cross-origin' }, 'https://example.invalid')).status, 403);
  passed.push('Reject cross-origin mutation');
  const before = rows(await html()).length;
  const titles = Array.from({ length: 8 }, (_, i) => prefix + `parallel-${i}`);
  await Promise.all(titles.map(title => post('/todos/add', { title, filter: 'all' }).then(response => assert.equal(response.status, 303))));
  const afterRows = rows(await html());
  for (const row of afterRows) if (row.html.includes(prefix)) created.add(row.id);
  assert.equal(afterRows.length, before + 8);
  passed.push('Eight concurrent writes without lost data');
  const missing = await post('/todos/delete', { id: '9223372036854775807', filter: 'active' });
  assert.match(missing.headers.get('location'), /filter=active&error=missing/);
  assert.equal((await post('/todos/complete', { id: 'bad', completed: 'true' })).status, 400);
  passed.push('Missing Todo and malformed input');
  await post('/todos/delete', { id, filter: 'all' });
  assert.ok(!rows(await html()).some(row => row.id === id));
  created.delete(id);
  passed.push('Delete');
  console.log(JSON.stringify({ passed }, null, 2));
} finally {
  const remaining = rows(await html()).filter(row => row.html.includes(prefix));
  for (const row of remaining) created.add(row.id);
  for (const id of created) await post('/todos/delete', { id, filter: 'all' });
}
