import assert from 'node:assert/strict';
const base = process.env.TODO_TEST_URL || 'http://127.0.0.1:3000';
const prefix = `検証-${Date.now()}-`;
const passed = [];
async function state(path = '/') {
  const response = await fetch(base + path, { signal: AbortSignal.timeout(10_000) });
  assert.equal(response.status, 200);
  const html = await response.text();
  const attribute = html.match(/data-initial-state="([^"]*)"/);
  assert.ok(attribute, 'Topcoat must supply escaped initial state');
  const entities = { '&quot;': '"', '&lt;': '<', '&gt;': '>', '&amp;': '&', '&#x27;': "'", '&#39;': "'" };
  return JSON.parse(attribute[1].replace(/&quot;|&lt;|&gt;|&amp;|&#x27;|&#39;/g, value => entities[value]));
}
async function post(path, values, origin = base) {
  const response = await fetch(base + path, { signal: AbortSignal.timeout(10_000), method: 'POST', headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: origin }, body: new URLSearchParams(values), redirect: 'manual' });
  await response.arrayBuffer();
  return response;
}
async function asset(path) {
  const response = await fetch(base + path, { signal: AbortSignal.timeout(10_000) });
  assert.equal(response.status, 200);
  assert.ok((await response.arrayBuffer()).byteLength > 0);
  return response;
}
async function add(title) {
  const response = await post('/todos/add', { title, filter: 'all' });
  assert.equal(response.status, 303);
  assert.equal(response.headers.get('location'), '/?filter=all');
  const found = (await state()).todos.find(todo => todo.title === title);
  assert.ok(found, `Created Todo not found: ${title}`);
  assert.equal(typeof found.id, 'string');
  return found.id;
}
try {
  assert.equal(await (await fetch(base + '/health')).text(), 'ok');
  assert.match((await asset('/style.css')).headers.get('content-type'), /text\/css/);
  assert.match((await asset('/app.js')).headers.get('content-type'), /javascript/);
  passed.push('HTTP health, CSS, JavaScript');
  const id = await add(prefix + '日本語のTodo');
  for (let attempt = 0; attempt < 2; attempt++) {
    assert.equal((await post('/todos/complete', { id, completed: 'true', filter: 'all' })).status, 303);
  }
  const completed = await state('/?filter=completed');
  assert.equal(completed.filter, 'completed');
  assert.equal(completed.todos.find(todo => todo.id === id).completed, true);
  passed.push('Add, complete, idempotent completion, selected filter');
  await post('/todos/complete', { id, completed: 'false', filter: 'active' });
  const active = await state('/?filter=active');
  assert.equal(active.filter, 'active');
  assert.equal(active.todos.find(todo => todo.id === id).completed, false);
  passed.push('Return to incomplete');
  for (const title of ['  　 ', 'あ'.repeat(201)]) {
    const response = await post('/todos/add', { title, filter: 'all' });
    assert.equal(response.status, 303);
    assert.match(response.headers.get('location'), /error=title/);
  }
  passed.push('Reject whitespace and overlong titles');
  await add(prefix + 'あ'.repeat(200 - [...prefix].length));
  passed.push('Accept 200 Unicode characters');
  await add(prefix + '\"><img src=x onerror=alert(1)></script><script>&quot;');
  passed.push('Round-trip HTML-like titles through escaped JSON state');
  assert.equal((await post('/todos/add', { title: prefix + 'cross-origin' }, 'https://example.invalid')).status, 403);
  passed.push('Reject cross-origin mutation');
  const before = (await state()).todos.length;
  const titles = Array.from({ length: 8 }, (_, i) => prefix + `parallel-${i}`);
  await Promise.all(titles.map(title => post('/todos/add', { title, filter: 'all' }).then(response => assert.equal(response.status, 303))));
  assert.equal((await state()).todos.length, before + 8);
  passed.push('Eight concurrent writes without lost data');
  const missing = await post('/todos/delete', { id: '9223372036854775807', filter: 'active' });
  assert.match(missing.headers.get('location'), /filter=active&error=missing/);
  assert.equal((await post('/todos/complete', { id: 'bad', completed: 'true' })).status, 400);
  passed.push('Missing Todo and malformed input');
  await post('/todos/delete', { id, filter: 'all' });
  assert.ok(!(await state()).todos.some(todo => todo.id === id));
  passed.push('Delete');
  console.log(JSON.stringify({ passed }, null, 2));
} finally {
  for (const todo of (await state()).todos.filter(todo => todo.title.startsWith(prefix))) {
    await post('/todos/delete', { id: todo.id, filter: 'all' });
  }
}
